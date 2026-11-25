use klink_cli::scan_repository;
use klink_common::ScanConfig;
use klink_graph::MmapCsrHalf;
use std::fs;
use std::path::PathBuf;

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("klink_p1_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn scans_all_eight_tier1_languages_plus_adversarial_cases() {
    let root = make_temp_dir("polyglot");

    // 1. Rust (with mutual cycle between a.rs and b.rs)
    fs::create_dir_all(root.join("rust")).unwrap();
    fs::write(root.join("rust/a.rs"), "use crate::rust::b;\npub fn alpha() {}\n").unwrap();
    fs::write(root.join("rust/b.rs"), "use crate::rust::a;\npub fn beta() {}\n").unwrap();

    // 2. C & C++
    fs::create_dir_all(root.join("native")).unwrap();
    fs::write(root.join("native/math.h"), "int add(int a, int b);\n").unwrap();
    fs::write(root.join("native/math.c"), "#include \"math.h\"\nint add(int a, int b) { return a + b; }\n").unwrap();
    fs::write(root.join("native/engine.cpp"), "#include \"math.h\"\nvoid run() {}\n").unwrap();

    // 3. Python (static + dynamic unresolved)
    fs::create_dir_all(root.join("py")).unwrap();
    fs::write(root.join("py/helper.py"), "def help_me():\n    return 1\n").unwrap();
    fs::write(
        root.join("py/main.py"),
        "from .helper import help_me\nimport importlib\nx = importlib.import_module('dyn')\n",
    )
    .unwrap();

    // 4. TypeScript & JavaScript + minified bundle (should be skipped loudly)
    fs::create_dir_all(root.join("web")).unwrap();
    fs::write(root.join("web/util.ts"), "export function sum(a: number, b: number) { return a + b; }\n").unwrap();
    fs::write(root.join("web/app.ts"), "import { sum } from './util';\nfunction boot() { return sum(1, 2); }\n").unwrap();
    fs::write(root.join("web/legacy.js"), "const u = require('./util');\nconst d = require(dynamicPath);\n").unwrap();
    fs::write(root.join("web/bundle.min.js"), "var x=1;".repeat(250)).unwrap();

    // 5. Go
    fs::create_dir_all(root.join("goapp/pkg")).unwrap();
    fs::write(root.join("goapp/pkg/store.go"), "package pkg\nfunc Open() {}\n").unwrap();
    fs::write(root.join("goapp/main.go"), "package main\nimport (\n    \"goapp/pkg/store\"\n)\nfunc main() {}\n").unwrap();

    // 6. Java (static + reflection unresolved)
    fs::create_dir_all(root.join("java/com/klink")).unwrap();
    fs::write(root.join("java/com/klink/Core.java"), "package com.klink;\npublic class Core {}\n").unwrap();
    fs::write(
        root.join("java/com/klink/App.java"),
        "package com.klink;\nimport com.klink.Core;\npublic class App { void f() { Class.forName(\"x\"); } }\n",
    )
    .unwrap();

    // 7. Unicode & spaces filename
    fs::create_dir_all(root.join("unicode dir")).unwrap();
    fs::write(root.join("unicode dir/modül_α.py"), "def greet():\n    return 'ok'\n").unwrap();

    let cfg = ScanConfig::default();
    let (report, graph) = scan_repository(&root, &cfg, true).unwrap();

    assert!(report.file_count >= 13);
    assert_eq!(report.skipped_count, 1, "minified JS bundle must be skipped");
    assert!(report.edge_count >= 7, "expected resolved edges across all Tier-1 languages, got {}", report.edge_count);
    assert!(report.unresolved_facts >= 3, "expected dynamic/reflection flags");
    assert_eq!(report.cycle_count, 1, "expected 1 SCC cycle between rust/a.rs and rust/b.rs");

    // Verify zero-copy mmap CSR blobs written to `.klink/`
    let fwd_mmap = MmapCsrHalf::open(&root.join(".klink/csr.fwd.bin")).unwrap();
    let rev_mmap = MmapCsrHalf::open(&root.join(".klink/csr.rev.bin")).unwrap();
    assert_eq!(fwd_mmap.targets().len(), graph.fwd.edge_count());
    assert_eq!(rev_mmap.targets().len(), graph.rev.edge_count());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn handles_empty_and_single_file_repos() {
    let empty = make_temp_dir("empty");
    let cfg = ScanConfig::default();
    let (rep, g) = scan_repository(&empty, &cfg, false).unwrap();
    assert_eq!(rep.file_count, 0);
    assert_eq!(g.fwd.edge_count(), 0);
    let _ = fs::remove_dir_all(&empty);

    let single = make_temp_dir("single");
    fs::write(single.join("only.rs"), "fn main() {}\n").unwrap();
    let (rep2, g2) = scan_repository(&single, &cfg, false).unwrap();
    assert_eq!(rep2.file_count, 1);
    assert_eq!(g2.fwd.edge_count(), 0);
    let _ = fs::remove_dir_all(&single);
}
