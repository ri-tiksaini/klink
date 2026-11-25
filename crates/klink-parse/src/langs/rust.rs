use crate::fact::{Fact, Span, SymbolDef, SymbolKind};
use crate::langs::{extract_quoted, strip_c_comments};
use klink_common::{Confidence, EdgeKind, FileId};

pub fn extract(file_id: FileId, content: &str) -> (Vec<Fact>, Vec<SymbolDef>) {
    let clean = strip_c_comments(content);
    let mut facts = Vec::new();
    let mut syms = Vec::new();

    for (line_idx, raw_line) in clean.lines().enumerate() {
        let line = raw_line.trim();
        let span = Span { start: line_idx as u32, end: line_idx as u32 };

        // `mod foo;` or `pub mod foo;`
        let mod_rest = line
            .strip_prefix("pub mod ")
            .or_else(|| line.strip_prefix("pub(crate) mod "))
            .or_else(|| line.strip_prefix("mod "));
        if let Some(rest) = mod_rest {
            if let Some(name) = rest.strip_suffix(';') {
                let name = name.trim();
                if !name.is_empty() {
                    facts.push(Fact {
                        source: file_id,
                        kind: EdgeKind::Import,
                        raw_target: format!("./{name}"),
                        span,
                        confidence: Confidence::Static,
                    });
                }
            }
        }

        // `use crate::...;` / `use super::...;` / `use self::...;`
        let use_rest = line
            .strip_prefix("pub use ")
            .or_else(|| line.strip_prefix("pub(crate) use "))
            .or_else(|| line.strip_prefix("use "));
        if let Some(rest) = use_rest {
            let path_part = rest.trim_end_matches(';').split('{').next().unwrap_or("").trim();
            let path_part = path_part.trim_end_matches("::");
            if path_part.starts_with("crate::") || path_part.starts_with("super::") || path_part.starts_with("self::") {
                facts.push(Fact {
                    source: file_id,
                    kind: EdgeKind::Import,
                    raw_target: path_part.to_string(),
                    span,
                    confidence: Confidence::Static,
                });
            }
        }

        // `include!("file.rs")`
        if let Some(pos) = line.find("include!(") {
            if let Some(target) = extract_quoted(&line[pos..]) {
                facts.push(Fact {
                    source: file_id,
                    kind: EdgeKind::Include,
                    raw_target: format!("./{target}"),
                    span,
                    confidence: Confidence::Static,
                });
            }
        }

        // Function definitions
        if let Some(pos) = line.find("fn ") {
            let after = &line[pos + 3..];
            let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                syms.push(SymbolDef {
                    file: file_id,
                    name,
                    kind: SymbolKind::Function,
                    span,
                });
            }
        }
    }

    (facts, syms)
}
