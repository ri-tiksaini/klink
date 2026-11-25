use crate::plugin::Language;
use std::path::Path;

fn has_binary_magic(head: &[u8]) -> bool {
    if head.len() >= 4 {
        // ELF
        if &head[..4] == b"\x7fELF" {
            return true;
        }
        // Mach-O 32/64 & fat binaries
        let m = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
        if matches!(m, 0xFEED_FACE | 0xCEFA_EDFE | 0xFEED_FACF | 0xCFFA_EDFE | 0xCAFE_BABE) {
            return true;
        }
    }
    // Windows PE/MZ header (require at least 64 bytes so text files starting with "MZ" don't false-positive)
    if head.len() >= 64 && &head[..2] == b"MZ" && head.contains(&0u8) {
        return true;
    }
    false
}

/// Map file name, extension, magic bytes, or shebang line to a static Language enum.
pub fn detect_language(path: &Path, head: &[u8]) -> Option<Language> {
    if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
        match fname {
            "Cargo.toml" | "package.json" | "Makefile" | "GNUmakefile" | "CMakeLists.txt" => {
                return Some(Language::BuildFile);
            }
            _ => {}
        }
    }

    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let lower = ext.to_ascii_lowercase();
        let lang = match lower.as_str() {
            "c" | "h" => Some(Language::C),
            "cc" | "cpp" | "cxx" | "hh" | "hpp" | "hxx" => Some(Language::Cpp),
            "rs" => Some(Language::Rust),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            "js" | "mjs" | "cjs" | "jsx" => Some(Language::JavaScript),
            "ts" | "mts" | "cts" | "tsx" => Some(Language::TypeScript),
            "py" | "pyi" => Some(Language::Python),
            "exe" | "dll" | "so" | "dylib" => Some(Language::Binary),
            "json" | "toml" | "yaml" | "yml" | "ini" | "cfg" | "conf" | "xml" => {
                Some(Language::ConfigText)
            }
            _ => None,
        };
        if lang.is_some() {
            return lang;
        }
    }

    if has_binary_magic(head) {
        return Some(Language::Binary);
    }

    if head.starts_with(b"#!") {
        let first_line_end = head.iter().position(|&b| b == b'\n').unwrap_or(head.len());
        if let Ok(line) = std::str::from_utf8(&head[..first_line_end]) {
            if line.contains("python") {
                return Some(Language::Python);
            }
            if line.contains("node") || line.contains("deno") || line.contains("bun") {
                return Some(Language::JavaScript);
            }
        }
    }

    None
}
