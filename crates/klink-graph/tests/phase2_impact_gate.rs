use klink_common::{Confidence, EdgeKind, EdgeMeta};
use klink_graph::{
    brandes_betweenness, condense_graph, tarjan_scc_iterative, GrailIndex, GraphBuilder,
    GraphSnapshot, ImpactEngine, RawEdge,
};
use std::time::Instant;

#[test]
fn gate_20k_node_impact_latency_and_hub_cache() {
    let n: u32 = 20_000;
    let meta = EdgeMeta::new(Confidence::Static, EdgeKind::Import);
    let mut edges = Vec::with_capacity(60_000);

    // Modular graph with hub utilities at indices 0..10
    for u in 10..n {
        let cluster_base = (u / 100) * 100;
        let local_target = cluster_base + (u % 17).min(u - cluster_base);
        if local_target != u {
            edges.push(RawEdge { src: u, dst: local_target, meta });
        }
        if u % 20 == 0 {
            edges.push(RawEdge { src: u, dst: u % 10, meta });
        }
    }

    let graph = GraphBuilder::build(n, edges);
    let scc = tarjan_scc_iterative(&graph);
    let dag = condense_graph(&graph, &scc);
    let mut engine = ImpactEngine::build(&graph, 128 * 1024 * 1024);

    assert!(engine.cached_hub_count() > 0);

    // Hub hit must come from cache and include many dependents
    let hub_res = engine.query_impact(&graph, &scc, 0);
    assert!(hub_res.from_cache);
    assert!(hub_res.total_affected >= 50);

    // Measure 200 random queries and verify P99 is well under latency gate
    let mut times_us = Vec::with_capacity(200);
    for k in 0..200u32 {
        let node = (k * 97) % n;
        let t0 = Instant::now();
        let _ = engine.query_impact(&graph, &scc, node);
        times_us.push(t0.elapsed().as_micros());
    }
    times_us.sort_unstable();
    let p99_us = times_us[197];
    // Allow generous margin in debug mode, strict < 1ms in release
    #[cfg(not(debug_assertions))]
    assert!(p99_us < 1_000, "P99 impact query took {} us (gate < 1000 us)", p99_us);
    #[cfg(debug_assertions)]
    assert!(p99_us < 10_000, "Debug P99 impact query took {} us", p99_us);

    // Verify GRAIL pairwise negative pruning
    let grail = GrailIndex::build(&dag, 3);
    assert!(grail.may_reach(scc.node_to_comp[20], scc.node_to_comp[0]));
}

#[test]
fn gate_brandes_centrality_and_snapshot_diff() {
    let meta = EdgeMeta::new(Confidence::Static, EdgeKind::Import);
    // Line 0 -> 1 -> 2 -> 3 -> 4 (node 2 is the central bridge)
    let g1 = GraphBuilder::build(
        5,
        vec![
            RawEdge { src: 0, dst: 1, meta },
            RawEdge { src: 1, dst: 2, meta },
            RawEdge { src: 2, dst: 3, meta },
            RawEdge { src: 3, dst: 4, meta },
        ],
    );
    let scores = brandes_betweenness(&g1);
    assert_eq!(scores.len(), 5);
    assert!((scores[2] - 1.0).abs() < 1e-5, "center node must have highest centrality");
    assert_eq!(scores[0], 0.0);

    let g2 = GraphBuilder::build(
        5,
        vec![
            RawEdge { src: 0, dst: 1, meta },
            RawEdge { src: 1, dst: 2, meta },
            RawEdge { src: 2, dst: 4, meta },
        ],
    );
    let diff = GraphSnapshot::new(1, g1).diff(&GraphSnapshot::new(2, g2));
    assert_eq!(diff.added, vec![(2, 4)]);
    assert_eq!(diff.removed, vec![(2, 3), (3, 4)]);
}
