use klink_common::ScanConfig;
use klink_core::{KlinkEngine, ScanState};
use std::fs;

#[test]
fn gate_engine_end_to_end_binary_buffers_and_impact_mask() {
    let dir = std::env::temp_dir().join(format!("klink_p3_core_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/util.rs"), "pub fn helper_fn() {}\n").unwrap();
    fs::write(dir.join("src/main.rs"), "use crate::src::util;\nfn main() { helper_fn(); }\n").unwrap();

    let engine = KlinkEngine::new();
    assert_eq!(engine.scan_state(), ScanState::Idle);

    let summary = engine.scan_folder(&dir, &ScanConfig::default()).unwrap();
    assert_eq!(engine.scan_state(), ScanState::Ready);
    assert_eq!(summary.node_count, 2);
    assert_eq!(summary.edge_count, 1);

    let node_bytes = engine.packed_nodes_bytes();
    assert_eq!(node_bytes.len(), 2 * 32, "32 bytes per GpuNodeInstance");
    let edge_bytes = engine.packed_edges_bytes();
    assert_eq!(edge_bytes.len(), 28, "28 bytes per GpuEdgeInstance");

    let (mask_bytes, impact) = engine.query_impact_mask(1, 256 * 8).unwrap();
    assert_eq!(mask_bytes.len(), 256 * 8 * 4);
    assert_eq!(impact.total_affected, 1);

    let hits = engine.search_symbols("helper", 10);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].name, "helper_fn");

    let _ = fs::remove_dir_all(&dir);
}
