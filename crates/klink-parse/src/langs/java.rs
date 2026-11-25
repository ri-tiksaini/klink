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

        if let Some(rest) = line.strip_prefix("import ") {
            let cleaned = rest
                .strip_prefix("static ")
                .unwrap_or(rest)
                .trim_end_matches(';')
                .trim();
            if !cleaned.is_empty() && !cleaned.ends_with(".*") {
                facts.push(Fact {
                    source: file_id,
                    kind: EdgeKind::Import,
                    raw_target: cleaned.to_string(),
                    span,
                    confidence: Confidence::Static,
                });
            }
        }

        if line.contains("Class.forName(") {
            facts.push(Fact {
                source: file_id,
                kind: EdgeKind::DynamicRef,
                raw_target: "<java-reflection>".to_string(),
                span,
                confidence: Confidence::Unresolved,
            });
        }

        if let Some(pos) = line.find("class ") {
            let after = &line[pos + 6..];
            let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                syms.push(SymbolDef { file: file_id, name, kind: SymbolKind::Type, span });
            }
        }
    }

    (facts, syms)
}
