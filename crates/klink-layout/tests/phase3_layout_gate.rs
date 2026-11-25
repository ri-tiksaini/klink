use klink_common::{Confidence, EdgeKind, EdgeMeta};
use klink_graph::{condense_graph, tarjan_scc_iterative, GraphBuilder, RawEdge};
use klink_layout::{
    build_lod_clusters, deterministic_seed_positions, louvain_clusters, pack_nodes, run_barnes_hut,
    stress_majorization, sugiyama_layered, ForceConfig, GpuEdgeInstance, GpuNodeInstance,
};

#[test]
fn gate_gpu_struct_sizes_and_deterministic_layout() {
    assert_eq!(std::mem::size_of::<GpuNodeInstance>(), 32);
    assert_eq!(std::mem::size_of::<GpuEdgeInstance>(), 28);

    let n = 400u32;
    let meta = EdgeMeta::new(Confidence::Static, EdgeKind::Import);
    let mut edges = Vec::new();
    for i in 1..n {
        edges.push(RawEdge { src: i, dst: i / 2, meta });
    }
    let g = GraphBuilder::build(n, edges);
    let scc = tarjan_scc_iterative(&g);
    let dag = condense_graph(&g, &scc);

    let clusters = louvain_clusters(&g, 50);
    assert!(clusters.cluster_count >= 1);

    let mut p1 = deterministic_seed_positions(&g, &clusters);
    let mut p2 = deterministic_seed_positions(&g, &clusters);
    assert_eq!(p1, p2, "seeded layout must be bit-identical");

    let cfg = ForceConfig { max_ticks: 40, ..Default::default() };
    run_barnes_hut(&g, &mut p1, &cfg);
    run_barnes_hut(&g, &mut p2, &cfg);
    assert_eq!(p1, p2, "Barnes-Hut layout must be deterministic");

    // Stress majorization respects pinned nodes
    let mut pinned = vec![false; n as usize];
    pinned[0] = true;
    let anchor = p1[0];
    stress_majorization(&g, &mut p1, &pinned, 80.0, 20);
    assert_eq!(p1[0], anchor);

    let lod = build_lod_clusters(&p1, &clusters);
    assert_eq!(lod.len(), clusters.cluster_count as usize);

    let layered = sugiyama_layered(&dag, 120.0, 40.0);
    assert_eq!(layered.len(), dag.graph.node_count as usize);

    let packed = pack_nodes(&p1, &vec![0.5; n as usize], &vec![0u8; n as usize], &clusters.node_to_cluster, &scc);
    assert_eq!(packed.len(), n as usize);
}
