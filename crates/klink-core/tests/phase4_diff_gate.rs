use klink_common::ScanConfig;
use klink_core::KlinkEngine;
use std::fs;

#[test]
fn gate_rescan_diff_detects_added_and_removed_edges() {
    let dir = std::env::temp_dir().join(format!("klink_p4_diff_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/a.rs"), "pub fn a() {}\n").unwrap();
    fs::write(dir.join("src/b.rs"), "pub fn b() {}\n").unwrap();
    fs::write(dir.join("src/main.rs"), "use crate::src::a;\nfn main() {}\n").unwrap();

    let engine = KlinkEngine::new();
    let cfg = ScanConfig::default();

    engine.scan_folder(&dir, &cfg).unwrap();
    assert!(engine.scan_diff().is_none(), "no diff before second scan");

    // Refactor main.rs: replace `a` with `b`
    fs::write(dir.join("src/main.rs"), "use crate::src::b;\nfn main() {}\n").unwrap();
    engine.scan_folder(&dir, &cfg).unwrap();

    let diff = engine.scan_diff().expect("diff must exist after rescan");
    assert_eq!(diff.added_edges.len(), 1);
    assert_eq!(diff.added_edges[0].dst, "src/b.rs");
    assert_eq!(diff.removed_edges.len(), 1);
    assert_eq!(diff.removed_edges[0].dst, "src/a.rs");

    let _ = fs::remove_dir_all(&dir);
}
