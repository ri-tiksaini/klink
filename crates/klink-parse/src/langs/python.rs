use crate::fact::{Fact, Span, SymbolDef, SymbolKind};
use klink_common::{Confidence, EdgeKind, FileId};

pub fn extract(file_id: FileId, content: &str) -> (Vec<Fact>, Vec<SymbolDef>) {
    let mut facts = Vec::new();
    let mut syms = Vec::new();

    for (line_idx, raw_line) in content.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let span = Span { start: line_idx as u32, end: line_idx as u32 };

        if let Some(rest) = line.strip_prefix("from ") {
            if let Some((module, _)) = rest.split_once(" import ") {
                let m = module.trim();
                if !m.is_empty() {
                    facts.push(Fact {
                        source: file_id,
                        kind: EdgeKind::Import,
                        raw_target: m.to_string(),
                        span,
                        confidence: Confidence::Static,
                    });
                }
            }
        } else if let Some(rest) = line.strip_prefix("import ") {
            for item in rest.split(',') {
                let mod_name = item.trim().split_whitespace().next().unwrap_or("");
                if !mod_name.is_empty() {
                    facts.push(Fact {
                        source: file_id,
                        kind: EdgeKind::Import,
                        raw_target: mod_name.to_string(),
                        span,
                        confidence: Confidence::Static,
                    });
                }
            }
        }

        // Dynamic / reflection constructs flagged honestly as Unresolved
        if line.contains("importlib.import_module(")
            || line.contains("__import__(")
            || line.contains("eval(")
            || line.contains("exec(")
        {
            facts.push(Fact {
                source: file_id,
                kind: EdgeKind::DynamicRef,
                raw_target: "<python-dynamic-import>".to_string(),
                span,
                confidence: Confidence::Unresolved,
            });
        }

        if let Some(rest) = line.strip_prefix("def ").or_else(|| line.strip_prefix("async def ")) {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                syms.push(SymbolDef { file: file_id, name, kind: SymbolKind::Function, span });
            }
        } else if let Some(rest) = line.strip_prefix("class ") {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                syms.push(SymbolDef { file: file_id, name, kind: SymbolKind::Type, span });
            }
        }
    }

    (facts, syms)
}
