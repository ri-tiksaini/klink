use crate::fact::{Fact, Span};
use klink_common::{Confidence, EdgeKind, FileId};
use std::path::Path;

/// Extract explicit build-file target/path dependencies.
pub fn extract_buildfile_facts(file_id: FileId, path: &Path, content: &str) -> Vec<Fact> {
    let mut facts = Vec::new();
    let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

    match fname {
        "Cargo.toml" => {
            for (idx, line) in content.lines().enumerate() {
                if let Some(pos) = line.find("path") {
                    if let Some(rest) = line[pos..].split_once('=') {
                        let target = rest.1.trim().trim_matches(|c| c == '"' || c == '\'' || c == ',' || c == '}');
                        if !target.is_empty() {
                            facts.push(Fact {
                                source: file_id,
                                kind: EdgeKind::BuildDep,
                                raw_target: format!("{}/Cargo.toml", target.trim_end_matches('/')),
                                span: Span { start: idx as u32, end: idx as u32 },
                                confidence: Confidence::Static,
                            });
                        }
                    }
                }
            }
        }
        "package.json" => {
            for (idx, line) in content.lines().enumerate() {
                if let Some(pos) = line.find("file:") {
                    let rest = &line[pos + 5..];
                    let target: String = rest.chars().take_while(|&c| c != '"' && c != '\'').collect();
                    if !target.is_empty() {
                        facts.push(Fact {
                            source: file_id,
                            kind: EdgeKind::BuildDep,
                            raw_target: format!("{}/package.json", target.trim_end_matches('/')),
                            span: Span { start: idx as u32, end: idx as u32 },
                            confidence: Confidence::Static,
                        });
                    }
                }
            }
        }
        "Makefile" | "GNUmakefile" => {
            for (idx, line) in content.lines().enumerate() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("include ").or_else(|| t.strip_prefix("-include ")) {
                    for dep in rest.split_whitespace() {
                        facts.push(Fact {
                            source: file_id,
                            kind: EdgeKind::BuildDep,
                            raw_target: dep.to_string(),
                            span: Span { start: idx as u32, end: idx as u32 },
                            confidence: Confidence::Static,
                        });
                    }
                }
            }
        }
        "CMakeLists.txt" => {
            for (idx, line) in content.lines().enumerate() {
                let t = line.trim();
                if let Some(inner) = t.strip_prefix("add_subdirectory(").and_then(|s| s.strip_suffix(')')) {
                    let dir = inner.split_whitespace().next().unwrap_or("").trim_matches('"');
                    if !dir.is_empty() {
                        facts.push(Fact {
                            source: file_id,
                            kind: EdgeKind::BuildDep,
                            raw_target: format!("{dir}/CMakeLists.txt"),
                            span: Span { start: idx as u32, end: idx as u32 },
                            confidence: Confidence::Static,
                        });
                    }
                }
            }
        }
        _ => {}
    }

    facts
}
