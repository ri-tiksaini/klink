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

        // ES module `import ... from '...'` or `export ... from '...'`
        if (line.starts_with("import ") || line.starts_with("export ")) && line.contains(" from ") {
            if let Some((_, rhs)) = line.rsplit_once(" from ") {
                if let Some(target) = extract_quoted(rhs) {
                    facts.push(Fact {
                        source: file_id,
                        kind: EdgeKind::Import,
                        raw_target: target.to_string(),
                        span,
                        confidence: Confidence::Static,
                    });
                }
            }
        } else if line.starts_with("import ") {
            // Side-effect `import './polyfill'`
            if let Some(target) = extract_quoted(line) {
                facts.push(Fact {
                    source: file_id,
                    kind: EdgeKind::Import,
                    raw_target: target.to_string(),
                    span,
                    confidence: Confidence::Static,
                });
            }
        }

        // CommonJS `require('...')` or dynamic `require(expr)`
        if let Some(pos) = line.find("require(") {
            let after = &line[pos + 8..];
            if after.starts_with('"') || after.starts_with('\'') {
                if let Some(target) = extract_quoted(after) {
                    facts.push(Fact {
                        source: file_id,
                        kind: EdgeKind::Import,
                        raw_target: target.to_string(),
                        span,
                        confidence: Confidence::Static,
                    });
                }
            } else {
                facts.push(Fact {
                    source: file_id,
                    kind: EdgeKind::DynamicRef,
                    raw_target: "<dynamic-require>".to_string(),
                    span,
                    confidence: Confidence::Unresolved,
                });
            }
        }

        // Dynamic `import(...)` or `eval(...)`
        if let Some(pos) = line.find("import(") {
            let after = &line[pos + 7..];
            if after.starts_with('"') || after.starts_with('\'') {
                if let Some(target) = extract_quoted(after) {
                    facts.push(Fact {
                        source: file_id,
                        kind: EdgeKind::Import,
                        raw_target: target.to_string(),
                        span,
                        confidence: Confidence::Static,
                    });
                }
            } else {
                facts.push(Fact {
                    source: file_id,
                    kind: EdgeKind::DynamicRef,
                    raw_target: "<dynamic-import>".to_string(),
                    span,
                    confidence: Confidence::Unresolved,
                });
            }
        }

        if line.contains("eval(") {
            facts.push(Fact {
                source: file_id,
                kind: EdgeKind::DynamicRef,
                raw_target: "<eval>".to_string(),
                span,
                confidence: Confidence::Unresolved,
            });
        }

        if let Some(pos) = line.find("function ") {
            let after = &line[pos + 9..];
            let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$').collect();
            if !name.is_empty() {
                syms.push(SymbolDef { file: file_id, name, kind: SymbolKind::Function, span });
            }
        }
    }

    (facts, syms)
}
