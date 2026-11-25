use crate::fact::{Fact, Span};
use aho_corasick::AhoCorasick;
use klink_common::{Confidence, EdgeKind, FileId};

/// Single-pass Aho-Corasick filename reference detector for config/asset text files.
pub struct TextReferenceScanner {
    ac: AhoCorasick,
    patterns: Vec<String>,
}

impl TextReferenceScanner {
    pub fn build(known_paths: &[String]) -> Option<Self> {
        let mut patterns: Vec<String> = known_paths
            .iter()
            .filter_map(|p| p.rsplit('/').next())
            .filter(|name| name.len() >= 5 && name.contains('.'))
            .map(|s| s.to_string())
            .collect();
        patterns.sort();
        patterns.dedup();
        if patterns.is_empty() {
            return None;
        }
        let ac = AhoCorasick::new(&patterns).ok()?;
        Some(Self { ac, patterns })
    }

    pub fn scan(&self, file_id: FileId, content: &str) -> Vec<Fact> {
        let mut facts = Vec::new();
        for mat in self.ac.find_iter(content) {
            let pat = &self.patterns[mat.pattern().as_usize()];
            facts.push(Fact {
                source: file_id,
                kind: EdgeKind::TextRef,
                raw_target: pat.clone(),
                span: Span {
                    start: mat.start() as u32,
                    end: mat.end() as u32,
                },
                confidence: Confidence::Heuristic,
            });
        }
        facts
    }
}
