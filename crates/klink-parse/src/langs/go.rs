use crate::fact::{Fact, Span, SymbolDef, SymbolKind};
use crate::langs::{extract_quoted, strip_c_comments};
use klink_common::{Confidence, EdgeKind, FileId};

pub fn extract(file_id: FileId, content: &str) -> (Vec<Fact>, Vec<SymbolDef>) {
    let clean = strip_c_comments(content);
    let mut facts = Vec::new();
    let mut syms = Vec::new();
    let mut in_import_block = false;

    for (line_idx, raw_line) in clean.lines().enumerate() {
        let line = raw_line.trim();
        let span = Span { start: line_idx as u32, end: line_idx as u32 };

        if in_import_block {
            if line.starts_with(')') {
                in_import_block = false;
                continue;
            }
            if let Some(target) = extract_quoted(line) {
                facts.push(Fact {
                    source: file_id,
                    kind: EdgeKind::Import,
                    raw_target: target.to_string(),
                    span,
                    confidence: Confidence::Static,
                });
            }
            continue;
        }

        if line.starts_with("import (") || line == "import(" {
            in_import_block = true;
            continue;
        } else if line.starts_with("import ") {
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

        if let Some(rest) = line.strip_prefix("func ") {
            let after_recv = if rest.starts_with('(') {
                rest.split_once(')').map(|(_, r)| r.trim()).unwrap_or("")
            } else {
                rest
            };
            let name: String = after_recv.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                syms.push(SymbolDef { file: file_id, name, kind: SymbolKind::Function, span });
            }
        }
    }

    (facts, syms)
}
