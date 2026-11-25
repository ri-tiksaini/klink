use crate::fact::{Fact, Span, SymbolDef, SymbolKind};
use crate::langs::strip_c_comments;
use klink_common::{Confidence, EdgeKind, FileId};

pub fn extract(file_id: FileId, content: &str) -> (Vec<Fact>, Vec<SymbolDef>) {
    let clean = strip_c_comments(content);
    let mut facts = Vec::new();
    let mut syms = Vec::new();

    for (line_idx, raw_line) in clean.lines().enumerate() {
        let line = raw_line.trim();
        let span = Span { start: line_idx as u32, end: line_idx as u32 };

        if let Some(rest) = line.strip_prefix("#include") {
            let r = rest.trim();
            if let Some(inner) = r.strip_prefix('"').and_then(|s| s.split('"').next()) {
                facts.push(Fact {
                    source: file_id,
                    kind: EdgeKind::Include,
                    raw_target: inner.to_string(),
                    span,
                    confidence: Confidence::Static,
                });
            } else if let Some(inner) = r.strip_prefix('<').and_then(|s| s.split('>').next()) {
                facts.push(Fact {
                    source: file_id,
                    kind: EdgeKind::Include,
                    raw_target: inner.to_string(),
                    span,
                    confidence: Confidence::Heuristic,
                });
            }
        }

        if line.contains("dlopen(") || line.contains("LoadLibrary") {
            facts.push(Fact {
                source: file_id,
                kind: EdgeKind::DynamicRef,
                raw_target: "<dynamic-library-load>".to_string(),
                span,
                confidence: Confidence::Unresolved,
            });
        }

        // Lightweight C/C++ function definition heuristic
        if line.ends_with('{') && line.contains('(') && !line.starts_with("if") && !line.starts_with("for") && !line.starts_with("while") && !line.starts_with("switch") {
            if let Some(before_paren) = line.split('(').next() {
                if let Some(name) = before_paren.split_whitespace().last() {
                    let clean_name = name.trim_start_matches('*').trim_start_matches('&');
                    if !clean_name.is_empty() && clean_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ':') {
                        syms.push(SymbolDef {
                            file: file_id,
                            name: clean_name.to_string(),
                            kind: SymbolKind::Function,
                            span,
                        });
                    }
                }
            }
        }
    }

    (facts, syms)
}
