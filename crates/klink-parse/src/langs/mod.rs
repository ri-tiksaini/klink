pub mod c_cpp;
pub mod go;
pub mod java;
pub mod js_ts;
pub mod python;
pub mod rust;

use crate::fact::{Fact, SymbolDef};
use crate::plugin::Language;
use klink_common::FileId;

/// Dispatch to the static language extractor (zero dynamic vtables).
pub fn extract_source_facts(
    lang: Language,
    file_id: FileId,
    content: &str,
) -> (Vec<Fact>, Vec<SymbolDef>) {
    match lang {
        Language::C | Language::Cpp => c_cpp::extract(file_id, content),
        Language::Rust => rust::extract(file_id, content),
        Language::Python => python::extract(file_id, content),
        Language::JavaScript | Language::TypeScript => js_ts::extract(file_id, content),
        Language::Go => go::extract(file_id, content),
        Language::Java => java::extract(file_id, content),
        Language::BuildFile | Language::ConfigText | Language::Binary => (Vec::new(), Vec::new()),
    }
}

/// Strip C-style `//` and `/* ... */` comments while preserving line structure.
pub(crate) fn strip_c_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut in_str: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_str {
            out.push(b as char);
            if b == b'\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if b == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        if b == b'"' || b == b'\'' {
            in_str = Some(b);
            out.push(b as char);
            i += 1;
        } else if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                if bytes[i] == b'\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

pub(crate) fn extract_quoted(s: &str) -> Option<&str> {
    for q in ['"', '\''] {
        if let Some(start) = s.find(q) {
            let rest = &s[start + 1..];
            if let Some(end) = rest.find(q) {
                return Some(&rest[..end]);
            }
        }
    }
    None
}
