use ignore::WalkBuilder;
use klink_common::{Confidence, EdgeMeta, FileId, ParseFailure, Result, ScanConfig, Severity};
use klink_graph::{
    brandes_betweenness, compute_metrics, condense_graph, kahn_topo_sort, tarjan_scc_iterative,
    write_csr_file, CsrGraph, GraphBuilder, ImpactEngine, RawEdge, SccResult,
};
use klink_index::{extract_symbol_refs, SymbolStore, SymbolTable, UnifiedSearch, XrefIndex};
use klink_parse::{
    buildfiles::extract_buildfile_facts, detect_language, extract_binary_facts, generated::detect_generator,
    langs::extract_source_facts, should_skip_file, Fact, ImportResolver, Language, SkipReason,
    SymbolDef, TextReferenceScanner,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hotspot {
    pub path: String,
    pub language: String,
    pub fan_in: u32,
    pub fan_out: u32,
    pub instability: f32,
    pub centrality: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanReport {
    pub root: String,
    pub file_count: usize,
    pub skipped_count: usize,
    pub edge_count: usize,
    pub static_edges: usize,
    pub heuristic_edges: usize,
    pub unresolved_facts: usize,
    pub symbol_count: usize,
    pub cycle_count: usize,
    pub topo_layers: usize,
    pub duration_ms: u128,
    pub hotspots: Vec<Hotspot>,
    pub warnings: Vec<ParseFailure>,
}

pub struct ScanArtifacts {
    pub report: ScanReport,
    pub file_paths: Vec<String>,
    pub graph: CsrGraph,
    pub scc: SccResult,
    pub symbols: SymbolTable,
    pub xrefs: XrefIndex,
    pub search: UnifiedSearch,
    pub impact: ImpactEngine,
}

struct DiscoveredFile {
    abs_path: PathBuf,
    rel_norm: String,
    lang: Language,
}

struct ParsedFile {
    file_id: FileId,
    content: Option<String>,
    facts: Vec<Fact>,
    symbols: Vec<SymbolDef>,
    warning: Option<ParseFailure>,
    skipped: bool,
}

fn normalize_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Full Phase 2 scan: parse all 8 Tier-1 languages, build CSR + SCC + Brandes centrality + Symbol/XRef index + `.klink/` persistence.
pub fn scan_repository_full(
    root: &Path,
    cfg: &ScanConfig,
    persist_klink_dir: bool,
) -> Result<ScanArtifacts> {
    cfg.validate()?;
    let t0 = Instant::now();

    let mut files: Vec<DiscoveredFile> = Vec::new();
    let walker = WalkBuilder::new(root)
        .follow_links(false)
        .standard_filters(true)
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }
        let abs_path = entry.into_path();
        let rel_norm = normalize_rel(root, &abs_path);
        if cfg
            .ignore_patterns
            .iter()
            .any(|pat| rel_norm.split('/').any(|seg| seg == pat))
        {
            continue;
        }

        let mut head_buf = [0u8; 256];
        let head_len = fs::File::open(&abs_path)
            .and_then(|mut f| std::io::Read::read(&mut f, &mut head_buf))
            .unwrap_or(0);

        if let Some(lang) = detect_language(&abs_path, &head_buf[..head_len]) {
            files.push(DiscoveredFile {
                abs_path,
                rel_norm,
                lang,
            });
        }
    }

    files.sort_by(|a, b| a.rel_norm.cmp(&b.rel_norm));
    let norm_paths: Vec<String> = files.iter().map(|f| f.rel_norm.clone()).collect();
    let text_scanner = TextReferenceScanner::build(&norm_paths);

    let parsed: Vec<ParsedFile> = files
        .par_iter()
        .enumerate()
        .map(|(idx, df)| {
            let file_id = FileId(idx as u32);
            let bytes = match fs::read(&df.abs_path) {
                Ok(b) => b,
                Err(e) => {
                    return ParsedFile {
                        file_id,
                        content: None,
                        facts: Vec::new(),
                        symbols: Vec::new(),
                        warning: Some(ParseFailure {
                            path: df.rel_norm.clone(),
                            reason: format!("read error: {e}"),
                            severity: Severity::Error,
                        }),
                        skipped: true,
                    };
                }
            };

            if df.lang != Language::Binary {
            if let Some(reason) = should_skip_file(&bytes, cfg) {
                let msg = match reason {
                    SkipReason::TooLarge(sz) => format!("skipped: exceeds max_file_bytes ({sz} bytes)"),
                    SkipReason::BinaryBlob => "skipped: binary NUL bytes detected".to_string(),
                    SkipReason::Minified { mean_line_len } => {
                        format!("skipped: minified (mean line length {mean_line_len})")
                    }
                    SkipReason::HighEntropy { entropy } => {
                        format!("skipped: high entropy ({entropy:.2} bits/byte)")
                    }
                };
                return ParsedFile {
                    file_id,
                    content: None,
                    facts: Vec::new(),
                    symbols: Vec::new(),
                    warning: Some(ParseFailure {
                        path: df.rel_norm.clone(),
                        reason: msg,
                        severity: Severity::Warning,
                    }),
                    skipped: true,
                };
            }
            }

            let content = String::from_utf8_lossy(&bytes).into_owned();
            let mut facts = Vec::new();
            let mut symbols = Vec::new();

            if let Some(gen_fact) = detect_generator(file_id, &content) {
                facts.push(gen_fact);
            }

            match df.lang {
                Language::BuildFile => {
                    facts.extend(extract_buildfile_facts(file_id, &df.abs_path, &content));
                }
                Language::ConfigText => {
                    if let Some(scanner) = &text_scanner {
                        facts.extend(scanner.scan(file_id, &content));
                    }
                }
                Language::Binary => {
                    let (bf, bs, _) = extract_binary_facts(file_id, &bytes);
                    facts.extend(bf);
                    symbols.extend(bs);
                }
                src_lang => {
                    let (f, s) = extract_source_facts(src_lang, file_id, &content);
                    facts.extend(f);
                    symbols.extend(s);
                }
            }

            ParsedFile {
                file_id,
                content: if df.lang.is_source() { Some(content) } else { None },
                facts,
                symbols,
                warning: None,
                skipped: false,
            }
        })
        .collect();

    let resolver = ImportResolver::new(norm_paths.clone());
    let mut raw_edges: Vec<RawEdge> = Vec::new();
    let mut warnings: Vec<ParseFailure> = Vec::new();
    let mut all_symbol_defs: Vec<SymbolDef> = Vec::new();
    let mut source_contents: Vec<(FileId, String)> = Vec::new();
    let mut skipped_count = 0usize;
    let mut unresolved_facts = 0usize;

    for pf in parsed {
        if pf.skipped {
            skipped_count += 1;
        }
        if let Some(w) = pf.warning {
            warnings.push(w);
        }
        if let Some(c) = pf.content {
            source_contents.push((pf.file_id, c));
        }
        all_symbol_defs.extend(pf.symbols);

        for fact in pf.facts {
            if fact.confidence == Confidence::Unresolved {
                unresolved_facts += 1;
                continue;
            }
            if let Some((dst, conf)) = resolver.resolve(fact.source, &fact.raw_target, fact.confidence) {
                raw_edges.push(RawEdge {
                    src: fact.source.as_u32(),
                    dst: dst.as_u32(),
                    meta: EdgeMeta::new(conf, fact.kind),
                });
            }
        }
    }

    let graph = GraphBuilder::build(files.len() as u32, raw_edges);
    let scc = tarjan_scc_iterative(&graph);
    let dag = condense_graph(&graph, &scc);
    let topo = kahn_topo_sort(&dag);
    let metrics = compute_metrics(&graph);
    let centrality = brandes_betweenness(&graph);
    let impact = ImpactEngine::build(&graph, 128 * 1024 * 1024);

    let symbols = SymbolTable::from_defs(all_symbol_defs);
    let xrefs = extract_symbol_refs(&symbols, &source_contents);
    let search = UnifiedSearch::build(&symbols);

    let mut static_edges = 0usize;
    let mut heuristic_edges = 0usize;
    for m in &graph.fwd.meta {
        match m.confidence() {
            Confidence::Static => static_edges += 1,
            Confidence::Heuristic => heuristic_edges += 1,
            Confidence::Unresolved => {}
        }
    }

    let mut indexed_metrics: Vec<(usize, &klink_graph::NodeMetrics)> =
        metrics.iter().enumerate().collect();
    indexed_metrics.sort_by(|a, b| b.1.fan_in.cmp(&a.1.fan_in).then_with(|| a.0.cmp(&b.0)));

    let hotspots: Vec<Hotspot> = indexed_metrics
        .into_iter()
        .take(10)
        .filter(|(_, m)| m.fan_in > 0 || m.fan_out > 0)
        .map(|(i, m)| Hotspot {
            path: files[i].rel_norm.clone(),
            language: files[i].lang.as_str().to_string(),
            fan_in: m.fan_in,
            fan_out: m.fan_out,
            instability: m.instability,
            centrality: centrality[i],
        })
        .collect();

    if persist_klink_dir {
        let out_dir = root.join(".klink");
        fs::create_dir_all(&out_dir)?;
        write_csr_file(&out_dir.join("csr.fwd.bin"), &graph.fwd)?;
        write_csr_file(&out_dir.join("csr.rev.bin"), &graph.rev)?;
        let store = SymbolStore::open(&out_dir.join("symbols.redb"))?;
        store.write_all(&norm_paths, &symbols, &xrefs)?;
    }

    let report = ScanReport {
        root: root.to_string_lossy().to_string(),
        file_count: files.len(),
        skipped_count,
        edge_count: graph.fwd.edge_count(),
        static_edges,
        heuristic_edges,
        unresolved_facts,
        symbol_count: symbols.len(),
        cycle_count: scc.cycle_count(),
        topo_layers: topo.len(),
        duration_ms: t0.elapsed().as_millis(),
        hotspots,
        warnings,
    };

    Ok(ScanArtifacts {
        report,
        file_paths: norm_paths,
        graph,
        scc,
        symbols,
        xrefs,
        search,
        impact,
    })
}

/// Phase 1 compatibility wrapper used by existing adversarial tests.
pub fn scan_repository(
    root: &Path,
    cfg: &ScanConfig,
    persist_klink_dir: bool,
) -> Result<(ScanReport, CsrGraph)> {
    let art = scan_repository_full(root, cfg, persist_klink_dir)?;
    Ok((art.report, art.graph))
}
