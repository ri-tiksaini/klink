use crate::state::ScanState;
use arc_swap::ArcSwapOption;
use bytemuck::cast_slice;
use ignore::WalkBuilder;
use klink_common::{Confidence, EdgeMeta, FileId, KlinkError, Result, ScanConfig};
use klink_graph::{
    brandes_betweenness, compute_metrics, tarjan_scc_iterative, write_csr_file, CsrGraph,
    GraphBuilder, ImpactEngine, NodeMetrics, RawEdge, SccResult,
};
use klink_index::{extract_symbol_refs, SymbolStore, SymbolTable, UnifiedSearch, XrefIndex};
use klink_layout::{
    build_lod_clusters, deterministic_seed_positions, directory_clusters, pack_edges, pack_nodes,
    refine_folder_clusters, GpuEdgeInstance, GpuNodeInstance,
};
use klink_parse::{
    buildfiles::extract_buildfile_facts, detect_language, extract_binary_facts, generated::detect_generator,
    langs::extract_source_facts, should_skip_file, Fact, ImportResolver, Language, SymbolDef,
    TextReferenceScanner,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterDto {
    pub id: u32,
    pub name: String,
    pub cx: f32,
    pub cy: f32,
    pub radius: f32,
    pub member_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanSummaryDto {
    pub root: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub static_edges: usize,
    pub heuristic_edges: usize,
    pub symbol_count: usize,
    pub cycle_count: usize,
    pub cluster_count: u32,
    pub unresolved_count: usize,
    pub duration_ms: u128,
    pub lang_counts: [u32; 9],
    pub clusters: Vec<ClusterDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImpactSummaryDto {
    pub node_id: u32,
    pub total_affected: u64,
    pub direct_count: usize,
    pub via_cycle_count: usize,
    pub query_us: u128,
    pub from_cache: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeDetailDto {
    pub node_id: u32,
    pub path: String,
    pub language: String,
    pub cluster_name: String,
    pub x: f32,
    pub y: f32,
    pub fan_in: u32,
    pub fan_out: u32,
    pub instability: f32,
    pub centrality: f32,
    pub in_cycle: bool,
    pub direct_dependents: Vec<(u32, String)>,
    pub symbols: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHitDto {
    pub name: String,
    pub file_id: u32,
    pub path: String,
    pub x: f32,
    pub y: f32,
    pub line: u32,
    pub match_kind: String,
    pub xref_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeDiffItem {
    pub src: String,
    pub dst: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanDiffDto {
    pub added_edges: Vec<EdgeDiffItem>,
    pub removed_edges: Vec<EdgeDiffItem>,
    pub added_files: Vec<String>,
    pub removed_files: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileSearchDto {
    pub node_id: u32,
    pub path: String,
    pub language: String,
    pub x: f32,
    pub y: f32,
}

pub struct ProjectState {
    pub summary: ScanSummaryDto,
    pub file_paths: Vec<String>,
    pub file_langs: Vec<Language>,
    pub node_to_cluster: Vec<u32>,
    pub cluster_names: Vec<String>,
    pub positions: Vec<[f32; 2]>,
    pub graph: CsrGraph,
    pub scc: SccResult,
    pub metrics: Vec<NodeMetrics>,
    pub centrality: Vec<f32>,
    pub symbols: SymbolTable,
    pub xrefs: XrefIndex,
    pub search: UnifiedSearch,
    pub gpu_nodes: Vec<GpuNodeInstance>,
    pub gpu_edges: Vec<GpuEdgeInstance>,
    pub impact: Mutex<ImpactEngine>,
}

/// Core Klink engine owning lock-free immutable snapshots (`ArcSwapOption`).
pub struct KlinkEngine {
    snapshot: ArcSwapOption<ProjectState>,
    previous: ArcSwapOption<ProjectState>,
    state: AtomicU8,
    cancelled: AtomicBool,
}

impl Default for KlinkEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl KlinkEngine {
    pub fn new() -> Self {
        Self {
            snapshot: ArcSwapOption::from(None),
            previous: ArcSwapOption::from(None),
            state: AtomicU8::new(ScanState::Idle as u8),
            cancelled: AtomicBool::new(false),
        }
    }

    pub fn scan_state(&self) -> ScanState {
        ScanState::from_u8(self.state.load(Ordering::Relaxed))
    }

    pub fn cancel_scan(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    fn lang_to_u8(lang: Language) -> u8 {
        match lang {
            Language::Rust => 0,
            Language::C => 1,
            Language::Cpp => 2,
            Language::Python => 3,
            Language::TypeScript => 4,
            Language::JavaScript => 5,
            Language::Go => 6,
            Language::Java => 7,
            Language::BuildFile | Language::ConfigText | Language::Binary => 8,
        }
    }

    pub fn scan_folder(&self, root: &Path, cfg: &ScanConfig) -> Result<ScanSummaryDto> {
        cfg.validate()?;
        self.cancelled.store(false, Ordering::SeqCst);
        self.state.store(ScanState::Walking as u8, Ordering::Relaxed);
        let t0 = Instant::now();

        let mut discovered: Vec<(PathBuf, String, Language)> = Vec::new();
        let walker = WalkBuilder::new(root)
            .follow_links(false)
            .standard_filters(true)
            .build();

        for entry in walker.flatten() {
            if self.cancelled.load(Ordering::Relaxed) {
                self.state.store(ScanState::Cancelled as u8, Ordering::Relaxed);
                return Err(KlinkError::Cancelled);
            }
            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }
            let abs = entry.into_path();
            let rel = abs
                .strip_prefix(root)
                .unwrap_or(&abs)
                .to_string_lossy()
                .replace('\\', "/");
            if cfg.ignore_patterns.iter().any(|pat| rel.split('/').any(|s| s == pat)) {
                continue;
            }
            let mut head = [0u8; 256];
            let n = fs::File::open(&abs)
                .and_then(|mut f| std::io::Read::read(&mut f, &mut head))
                .unwrap_or(0);
            if let Some(lang) = detect_language(&abs, &head[..n]) {
                discovered.push((abs, rel, lang));
            }
        }
        discovered.sort_by(|a, b| a.1.cmp(&b.1));

        self.state.store(ScanState::Parsing as u8, Ordering::Relaxed);
        let file_paths: Vec<String> = discovered.iter().map(|d| d.1.clone()).collect();
        let file_langs: Vec<Language> = discovered.iter().map(|d| d.2).collect();
        let text_scanner = TextReferenceScanner::build(&file_paths);

        let parsed: Vec<(FileId, Option<String>, Vec<Fact>, Vec<SymbolDef>)> = discovered
            .par_iter()
            .enumerate()
            .map(|(i, (abs, _, lang))| {
                let fid = FileId(i as u32);
                let Ok(bytes) = fs::read(abs) else {
                    return (fid, None, Vec::new(), Vec::new());
                };
                if *lang == Language::Binary {
                    let (bf, bs, _) = extract_binary_facts(fid, &bytes);
                    return (fid, None, bf, bs);
                }
                if should_skip_file(&bytes, cfg).is_some() {
                    return (fid, None, Vec::new(), Vec::new());
                }
                let content = String::from_utf8_lossy(&bytes).into_owned();
                let mut facts = Vec::new();
                let mut syms = Vec::new();
                if let Some(gf) = detect_generator(fid, &content) {
                    facts.push(gf);
                }
                match *lang {
                    Language::BuildFile => facts.extend(extract_buildfile_facts(fid, abs, &content)),
                    Language::ConfigText => {
                        if let Some(sc) = &text_scanner {
                            facts.extend(sc.scan(fid, &content));
                        }
                    }
                    src_l => {
                        let (f, s) = extract_source_facts(src_l, fid, &content);
                        facts.extend(f);
                        syms.extend(s);
                    }
                }
                let keep_src = if lang.is_source() { Some(content) } else { None };
                (fid, keep_src, facts, syms)
            })
            .collect();

        self.state.store(ScanState::Resolving as u8, Ordering::Relaxed);
        let resolver = ImportResolver::new(file_paths.clone());
        let mut raw_edges = Vec::new();
        let mut all_syms = Vec::new();
        let mut src_texts = Vec::new();
        let mut unresolved_count = 0usize;

        for (fid, content, facts, syms) in parsed {
            if let Some(c) = content {
                src_texts.push((fid, c));
            }
            all_syms.extend(syms);
            for fact in facts {
                if fact.confidence == Confidence::Unresolved {
                    unresolved_count += 1;
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

        self.state.store(ScanState::Building as u8, Ordering::Relaxed);
        let graph = GraphBuilder::build(file_paths.len() as u32, raw_edges);
        let scc = tarjan_scc_iterative(&graph);
        let metrics = compute_metrics(&graph);
        let centrality = brandes_betweenness(&graph);
        let impact = ImpactEngine::build(&graph, 128 * 1024 * 1024);

        let mut static_edges = 0usize;
        let mut heuristic_edges = 0usize;
        for m in &graph.fwd.meta {
            match m.confidence() {
                Confidence::Static => static_edges += 1,
                Confidence::Heuristic => heuristic_edges += 1,
                Confidence::Unresolved => {}
            }
        }

        self.state.store(ScanState::Indexing as u8, Ordering::Relaxed);
        let symbols = SymbolTable::from_defs(all_syms);
        let xrefs = extract_symbol_refs(&symbols, &src_texts);
        let search = UnifiedSearch::build(&symbols);

        // Folder-based cluster layout: one distinct visual island per folder
        self.state.store(ScanState::Layout as u8, Ordering::Relaxed);
        let clusters = directory_clusters(&file_paths);
        let mut pos = deterministic_seed_positions(&graph, &clusters);
        refine_folder_clusters(&graph, &clusters, &mut pos, 35);
        let lod_clusters = build_lod_clusters(&pos, &clusters);

        let mut lang_counts = [0u32; 9];
        let lang_u8: Vec<u8> = file_langs
            .iter()
            .map(|&l| {
                let code = Self::lang_to_u8(l);
                lang_counts[code as usize] += 1;
                code
            })
            .collect();

        let gpu_nodes = pack_nodes(&pos, &centrality, &lang_u8, &clusters.node_to_cluster, &scc);
        let gpu_edges = pack_edges(&graph, &pos, 60_000);

        let kdir = root.join(".klink");
        let _ = fs::create_dir_all(&kdir);
        let _ = write_csr_file(&kdir.join("csr.fwd.bin"), &graph.fwd);
        let _ = write_csr_file(&kdir.join("csr.rev.bin"), &graph.rev);
        if let Ok(store) = SymbolStore::open(&kdir.join("symbols.redb")) {
            let _ = store.write_all(&file_paths, &symbols, &xrefs);
        }

        let cluster_dtos: Vec<ClusterDto> = lod_clusters
            .into_iter()
            .map(|c| ClusterDto {
                id: c.id,
                name: c.name,
                cx: c.cx,
                cy: c.cy,
                radius: c.radius,
                member_count: c.member_count,
            })
            .collect();

        let summary = ScanSummaryDto {
            root: root.to_string_lossy().to_string(),
            node_count: file_paths.len(),
            edge_count: graph.fwd.edge_count(),
            static_edges,
            heuristic_edges,
            symbol_count: symbols.len(),
            cycle_count: scc.cycle_count(),
            cluster_count: clusters.cluster_count,
            unresolved_count,
            duration_ms: t0.elapsed().as_millis(),
            lang_counts,
            clusters: cluster_dtos,
        };

        let proj = Arc::new(ProjectState {
            summary: summary.clone(),
            file_paths,
            file_langs,
            node_to_cluster: clusters.node_to_cluster,
            cluster_names: clusters.cluster_names,
            positions: pos,
            graph,
            scc,
            metrics,
            centrality,
            symbols,
            xrefs,
            search,
            gpu_nodes,
            gpu_edges,
            impact: Mutex::new(impact),
        });

        let prev = self.snapshot.swap(Some(proj));
        if prev.is_some() {
            self.previous.store(prev);
        }
        self.state.store(ScanState::Ready as u8, Ordering::Relaxed);
        Ok(summary)
    }

    pub fn packed_nodes_bytes(&self) -> Vec<u8> {
        match self.snapshot.load_full() {
            Some(p) => cast_slice(&p.gpu_nodes).to_vec(),
            None => Vec::new(),
        }
    }

    pub fn packed_edges_bytes(&self) -> Vec<u8> {
        match self.snapshot.load_full() {
            Some(p) => cast_slice(&p.gpu_edges).to_vec(),
            None => Vec::new(),
        }
    }

    pub fn query_impact_mask(&self, node_id: u32, tex_words: usize) -> Option<(Vec<u8>, ImpactSummaryDto)> {
        let proj = self.snapshot.load_full()?;
        if node_id >= proj.graph.node_count {
            return None;
        }
        let t0 = Instant::now();
        let res = {
            let mut guard = proj.impact.lock().ok()?;
            guard.query_impact(&proj.graph, &proj.scc, node_id)
        };
        let mut words = vec![0u32; tex_words];
        for id in res.bitmap.iter() {
            let wi = (id >> 5) as usize;
            if wi < words.len() {
                words[wi] |= 1u32 << (id & 31);
            }
        }
        let dto = ImpactSummaryDto {
            node_id,
            total_affected: res.total_affected,
            direct_count: res.direct.len(),
            via_cycle_count: res.via_cycle.len(),
            query_us: t0.elapsed().as_micros(),
            from_cache: res.from_cache,
        };
        Some((cast_slice(&words).to_vec(), dto))
    }

    pub fn node_detail(&self, node_id: u32) -> Option<NodeDetailDto> {
        let proj = self.snapshot.load_full()?;
        let i = node_id as usize;
        if i >= proj.file_paths.len() {
            return None;
        }
        let m = proj.metrics[i];
        let comp = proj.scc.node_to_comp[i] as usize;
        let in_cycle = proj.scc.components[comp].len() > 1;
        let cid = proj.node_to_cluster[i] as usize;
        let cluster_name = proj
            .cluster_names
            .get(cid)
            .cloned()
            .unwrap_or_else(|| "(root)".to_string());
        let [x, y] = proj.positions[i];

        let direct_dependents: Vec<(u32, String)> = proj
            .graph
            .rev
            .neighbors(node_id)
            .iter()
            .take(80)
            .map(|&d| (d, proj.file_paths[d as usize].clone()))
            .collect();

        let symbols: Vec<String> = proj
            .symbols
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| f.as_u32() == node_id)
            .take(50)
            .map(|(si, _)| proj.symbols.names[si].clone())
            .collect();

        Some(NodeDetailDto {
            node_id,
            path: proj.file_paths[i].clone(),
            language: proj.file_langs[i].as_str().to_string(),
            cluster_name,
            x,
            y,
            fan_in: m.fan_in,
            fan_out: m.fan_out,
            instability: m.instability,
            centrality: proj.centrality[i],
            in_cycle,
            direct_dependents,
            symbols,
        })
    }

    pub fn search_files(&self, query: &str, limit: usize) -> Vec<FileSearchDto> {
        let Some(proj) = self.snapshot.load_full() else {
            return Vec::new();
        };
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (i, p) in proj.file_paths.iter().enumerate() {
            if p.to_ascii_lowercase().contains(&q) {
                let [x, y] = proj.positions[i];
                out.push(FileSearchDto {
                    node_id: i as u32,
                    path: p.clone(),
                    language: proj.file_langs[i].as_str().to_string(),
                    x,
                    y,
                });
                if out.len() >= limit {
                    break;
                }
            }
        }
        out
    }

    pub fn search_symbols(&self, query: &str, limit: usize) -> Vec<SearchHitDto> {
        let Some(proj) = self.snapshot.load_full() else {
            return Vec::new();
        };
        proj.search
            .query(&proj.symbols, query, limit)
            .into_iter()
            .map(|h| {
                let fi = h.record.file.as_usize();
                let [x, y] = proj.positions[fi];
                SearchHitDto {
                    name: h.record.name,
                    file_id: h.record.file.as_u32(),
                    path: proj.file_paths[fi].clone(),
                    x,
                    y,
                    line: h.record.start_line + 1,
                    match_kind: format!("{:?}", h.match_kind),
                    xref_count: proj.xrefs.references(h.record.id).len(),
                }
            })
            .collect()
    }

    /// Compute file-path-resolved edge diff between the previous scan and current scan.
    pub fn scan_diff(&self) -> Option<ScanDiffDto> {
        let curr = self.snapshot.load_full()?;
        let prev = self.previous.load_full()?;

        let collect_edge_set = |p: &ProjectState| -> std::collections::BTreeSet<(String, String)> {
            let mut set = std::collections::BTreeSet::new();
            for u in 0..p.graph.node_count {
                let src = &p.file_paths[u as usize];
                for &v in p.graph.fwd.neighbors(u) {
                    set.insert((src.clone(), p.file_paths[v as usize].clone()));
                }
            }
            set
        };

        let old_edges = collect_edge_set(&prev);
        let new_edges = collect_edge_set(&curr);

        let added_edges = new_edges
            .difference(&old_edges)
            .take(100)
            .map(|(s, d)| EdgeDiffItem { src: s.clone(), dst: d.clone() })
            .collect();
        let removed_edges = old_edges
            .difference(&new_edges)
            .take(100)
            .map(|(s, d)| EdgeDiffItem { src: s.clone(), dst: d.clone() })
            .collect();

        let old_files: std::collections::BTreeSet<&String> = prev.file_paths.iter().collect();
        let new_files: std::collections::BTreeSet<&String> = curr.file_paths.iter().collect();

        let added_files = new_files.difference(&old_files).take(100).map(|s| (*s).clone()).collect();
        let removed_files = old_files.difference(&new_files).take(100).map(|s| (*s).clone()).collect();

        Some(ScanDiffDto {
            added_edges,
            removed_edges,
            added_files,
            removed_files,
        })
    }
}
