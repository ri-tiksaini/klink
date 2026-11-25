use klink_cli::scan_repository_full;
use klink_common::ScanConfig;
use std::fs;

#[test]
fn gate_ground_truth_accuracy_across_all_8_tier1_languages() {
    let dir = std::env::temp_dir().join(format!("klink_p5_acc_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);

    // Construct 8 clean single-edge ground-truth pairs (1 per Tier-1 language = 8 expected static edges)
    fs::create_dir_all(dir.join("c")).unwrap();
    fs::write(dir.join("c/a.h"), "int f(void);\n").unwrap();
    fs::write(dir.join("c/a.c"), "#include \"a.h\"\nint f(void) { return 1; }\n").unwrap();

    fs::create_dir_all(dir.join("cpp")).unwrap();
    fs::write(dir.join("cpp/b.hpp"), "void g();\n").unwrap();
    fs::write(dir.join("cpp/b.cpp"), "#include \"b.hpp\"\nvoid g() {}\n").unwrap();

    fs::create_dir_all(dir.join("rs")).unwrap();
    fs::write(dir.join("rs/dep.rs"), "pub fn h() {}\n").unwrap();
    fs::write(dir.join("rs/lib.rs"), "mod dep;\nfn main() { dep::h(); }\n").unwrap();

    fs::create_dir_all(dir.join("go/pkg")).unwrap();
    fs::write(dir.join("go/pkg/util.go"), "package pkg\nfunc U() {}\n").unwrap();
    fs::write(dir.join("go/main.go"), "package main\nimport \"go/pkg/util\"\nfunc main() {}\n").unwrap();

    fs::create_dir_all(dir.join("java/app")).unwrap();
    fs::write(dir.join("java/app/Lib.java"), "package app;\npublic class Lib {}\n").unwrap();
    fs::write(dir.join("java/app/Main.java"), "package app;\nimport app.Lib;\npublic class Main {}\n").unwrap();

    fs::create_dir_all(dir.join("js")).unwrap();
    fs::write(dir.join("js/lib.js"), "function x() {}\n").unwrap();
    fs::write(dir.join("js/app.js"), "const l = require('./lib');\n").unwrap();

    fs::create_dir_all(dir.join("ts")).unwrap();
    fs::write(dir.join("ts/lib.ts"), "export function y() {}\n").unwrap();
    fs::write(dir.join("ts/app.ts"), "import { y } from './lib';\n").unwrap();

    fs::create_dir_all(dir.join("py")).unwrap();
    fs::write(dir.join("py/lib.py"), "def z():\n    return 1\n").unwrap();
    fs::write(dir.join("py/app.py"), "from .lib import z\n").unwrap();

    let art = scan_repository_full(&dir, &ScanConfig::default(), false).unwrap();
    assert_eq!(art.report.file_count, 16);
    assert_eq!(
        art.report.static_edges, 8,
        "Expected 100% recall (8/8 static edges) on ground-truth Tier-1 fixture, got {}",
        art.report.static_edges
    );

    let _ = fs::remove_dir_all(&dir);
}
