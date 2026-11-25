use klink_common::FileId;
use klink_parse::{detect_language, extract_binary_facts, Language};
use std::path::Path;

#[test]
fn gate_malformed_and_truncated_binaries_never_panic() {
    // Truncated ELF
    let trunc_elf = b"\x7fELF\x02\x01\x01\x00\x00\x00";
    assert_eq!(detect_language(Path::new("bin_no_ext"), trunc_elf), Some(Language::Binary));
    let (facts, syms, info) = extract_binary_facts(FileId(0), trunc_elf);
    assert!(facts.is_empty());
    assert!(syms.is_empty());
    assert!(info.warning.is_some());

    // Truncated PE
    let mut trunc_pe = vec![0u8; 64];
    trunc_pe[0] = b'M';
    trunc_pe[1] = b'Z';
    assert_eq!(detect_language(Path::new("app.exe"), &trunc_pe), Some(Language::Binary));
    let (_, _, info_pe) = extract_binary_facts(FileId(1), &trunc_pe);
    assert!(info_pe.warning.is_some());

    // Parse current test executable itself as a live native ELF/PE/Mach-O sanity check!
    if let Ok(exe_path) = std::env::current_exe() {
        if let Ok(bytes) = std::fs::read(&exe_path) {
            let (_, _, info_live) = extract_binary_facts(FileId(2), &bytes);
            assert!(info_live.warning.is_none(), "live native test binary must parse cleanly");
            assert!(matches!(info_live.format, "ELF" | "PE" | "Mach-O"));
        }
    }
}
