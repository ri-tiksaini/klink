use crate::symbols::SymbolTable;
use klink_common::SymbolId;

/// Sorted lowercase prefix index with O(log N + k) cache-linear range scan.
#[derive(Default)]
pub struct PrefixIndex {
    entries: Vec<(String, SymbolId)>,
}

impl PrefixIndex {
    pub fn build(symbols: &SymbolTable) -> Self {
        let mut entries: Vec<(String, SymbolId)> = symbols
            .names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.to_ascii_lowercase(), SymbolId(i as u32)))
            .collect();
        entries.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        Self { entries }
    }

    pub fn search(&self, prefix: &str, limit: usize) -> Vec<SymbolId> {
        if prefix.is_empty() || limit == 0 {
            return Vec::new();
        }
        let q = prefix.to_ascii_lowercase();
        let start = self.entries.partition_point(|(k, _)| k.as_str() < q.as_str());
        let mut out = Vec::new();
        for (k, sid) in &self.entries[start..] {
            if !k.starts_with(&q) || out.len() >= limit {
                break;
            }
            out.push(*sid);
        }
        out
    }
}
