use klink_cli::scan_repository_full;
use klink_common::ScanConfig;
use std::path::PathBuf;
use std::time::Instant;

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json_mode = args.iter().any(|a| a == "--json");
    let no_write = args.iter().any(|a| a == "--no-write");
    let impact_query = arg_value(&args, "--impact");
    let search_query = arg_value(&args, "--search");

    let mut target = PathBuf::from(".");
    let mut skip_next = false;
    for a in &args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a == "--impact" || a == "--search" {
            skip_next = true;
            continue;
        }
        if !a.starts_with("--") {
            target = PathBuf::from(a);
            break;
        }
    }

    let cfg = ScanConfig::default();
    match scan_repository_full(&target, &cfg, !no_write) {
        Ok(mut art) => {
            if let Some(file_q) = impact_query {
                let norm_q = file_q.replace('\\', "/");
                if let Some(id) = art
                    .file_paths
                    .iter()
                    .position(|p| p == &norm_q || p.ends_with(&norm_q))
                {
                    let t0 = Instant::now();
                    let res = art.impact.query_impact(&art.graph, &art.scc, id as u32);
                    let us = t0.elapsed().as_micros();
                    println!(
                        "Impact for {} ({} us, cached={}):",
                        art.file_paths[id], us, res.from_cache
                    );
                    println!(
                        "  Total affected: {} ({} direct, {} via cycle)",
                        res.total_affected,
                        res.direct.len(),
                        res.via_cycle.len()
                    );
                    for d in res.direct.iter().take(20) {
                        println!("    [direct] {}", art.file_paths[*d as usize]);
                    }
                } else {
                    eprintln!("File not found in scan: {file_q}");
                    std::process::exit(1);
                }
                return;
            }

            if let Some(sym_q) = search_query {
                let t0 = Instant::now();
                let hits = art.search.query(&art.symbols, &sym_q, 20);
                let us = t0.elapsed().as_micros();
                println!("Symbol search '{}' ({} hits in {} us):", sym_q, hits.len(), us);
                for h in hits {
                    let file = &art.file_paths[h.record.file.as_usize()];
                    let refs = art.xrefs.references(h.record.id).len();
                    println!(
                        "  [{:?}] {} ({}:{}) — {} xrefs",
                        h.match_kind, h.record.name, file, h.record.start_line + 1, refs
                    );
                }
                return;
            }

            let report = &art.report;
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!("Klink Phase 2 Scan Complete ({} ms)", report.duration_ms);
                println!("  Root:        {}", report.root);
                println!("  Files:       {} (skipped: {})", report.file_count, report.skipped_count);
                println!(
                    "  Edges:       {} (static: {}, heuristic: {}, unresolved flags: {})",
                    report.edge_count,
                    report.static_edges,
                    report.heuristic_edges,
                    report.unresolved_facts
                );
                println!("  Symbols:     {}", report.symbol_count);
                println!("  SCC Cycles:  {}", report.cycle_count);
                if !report.hotspots.is_empty() {
                    println!("  Top Hotspots:");
                    for h in &report.hotspots {
                        println!(
                            "    - {} [{}] fan-in={} fan-out={} instab={:.2} centrality={:.2}",
                            h.path, h.language, h.fan_in, h.fan_out, h.instability, h.centrality
                        );
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("klink-cli error: {e}");
            std::process::exit(1);
        }
    }
}
