use klink_common::{Confidence, EdgeKind, EdgeMeta, FileId};
use klink_graph::{
    condense_graph, tarjan_scc_iterative, write_csr_file, GraphBuilder, ImpactEngine, MmapCsrHalf,
    RawEdge,
};
use klink_index::{SymbolTable, UnifiedSearch};
use klink_parse::{Span, SymbolDef, SymbolKind};
use std::time::Instant;

fn main() {
    println!("=== Klink v1.0 Latency-Contract Benchmark Suite (20,000 Nodes) ===");
    let n: u32 = 20_000;
    let meta = EdgeMeta::new(Confidence::Static, EdgeKind::Import);
    let mut edges = Vec::with_capacity(80_000);

    for u in 10..n {
        let cluster = (u / 100) * 100;
        let local = cluster + (u % 19).min(u - cluster);
        if local != u {
            edges.push(RawEdge { src: u, dst: local, meta });
        }
        if u % 15 == 0 {
            edges.push(RawEdge { src: u, dst: u % 10, meta });
        }
    }

    // 1. CSR build + iterative Tarjan SCC + condensation DAG
    let t0 = Instant::now();
    let graph = GraphBuilder::build(n, edges);
    let scc = tarjan_scc_iterative(&graph);
    let _dag = condense_graph(&graph, &scc);
    let build_ms = t0.elapsed().as_secs_f64() * 1000.0;
    println!("[PASS] CSR + Iterative Tarjan SCC + Condensation (20k nodes): {:.2} ms", build_ms);

    // 2. Zero-copy mmap open gate (< 150 ms)
    let tmp = std::env::temp_dir().join(format!("klink_bench_csr_{}.bin", std::process::id()));
    write_csr_file(&tmp, &graph.fwd).unwrap();
    let t_mmap = Instant::now();
    let mapped = MmapCsrHalf::open(&tmp).unwrap();
    let _ = mapped.neighbors(0).len();
    let mmap_ms = t_mmap.elapsed().as_secs_f64() * 1000.0;
    let _ = std::fs::remove_file(&tmp);
    let mmap_ok = mmap_ms < 150.0;
    println!(
        "[{}] Zero-copy mmap CSR open (< 150 ms gate): {:.3} ms",
        if mmap_ok { "PASS" } else { "FAIL" },
        mmap_ms
    );

    // 3. P99 impact query latency (< 1.0 ms gate)
    let mut engine = ImpactEngine::build(&graph, 128 * 1024 * 1024);
    let mut samples_us = Vec::with_capacity(500);
    for k in 0..500u32 {
        let node = (k * 97) % n;
        let ts = Instant::now();
        let _ = engine.query_impact(&graph, &scc, node);
        samples_us.push(ts.elapsed().as_micros());
    }
    samples_us.sort_unstable();
    let p99_us = samples_us[495];
    let impact_ok = p99_us < 1_000;
    println!(
        "[{}] P99 single-source impact query (< 1000 us gate): {} us (hubs cached: {})",
        if impact_ok { "PASS" } else { "FAIL" },
        p99_us,
        engine.cached_hub_count()
    );

    // 4. Unified symbol search latency (< 5.0 ms gate)
    let defs: Vec<SymbolDef> = (0..n)
        .map(|i| SymbolDef {
            file: FileId(i),
            name: format!("klink_symbol_handler_{i}"),
            kind: SymbolKind::Function,
            span: Span { start: 1, end: 1 },
        })
        .collect();
    let sym_table = SymbolTable::from_defs(defs);
    let search = UnifiedSearch::build(&sym_table);
    let t_search = Instant::now();
    let hits = search.query(&sym_table, "klink_symbol_handler_42", 20);
    let search_ms = t_search.elapsed().as_secs_f64() * 1000.0;
    let search_ok = search_ms < 5.0 && !hits.is_empty();
    println!(
        "[{}] Unified symbol search (< 5 ms gate): {:.3} ms ({} hits)",
        if search_ok { "PASS" } else { "FAIL" },
        search_ms,
        hits.len()
    );

    if !(mmap_ok && impact_ok && search_ok) {
        std::process::exit(1);
    }
    println!("=== ALL v1.0 LATENCY GATES PASSED ===");
}
