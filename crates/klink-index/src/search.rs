use crate::exact::ExactIndex;
use crate::fuzzy::FuzzyIndex;
use crate::prefix::PrefixIndex;
use crate::symbols::{SymbolRecord, SymbolTable};
use hashbrown::HashSet;
use klink_common::SymbolId;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchKind {
    Exact,
    Prefix,
    Fuzzy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub record: SymbolRecord,
    pub match_kind: MatchKind,
    pub score: f32,
}

/// Unified Exact -> Prefix -> Fuzzy symbol search engine.
pub struct UnifiedSearch {
    exact: ExactIndex,
    prefix: PrefixIndex,
    fuzzy: FuzzyIndex,
}

impl UnifiedSearch {
    pub fn build(symbols: &SymbolTable) -> Self {
        Self {
            exact: ExactIndex::build(symbols),
            prefix: PrefixIndex::build(symbols),
            fuzzy: FuzzyIndex::build(symbols),
        }
    }

    pub fn query(&self, symbols: &SymbolTable, q: &str, limit: usize) -> Vec<SearchHit> {
        let mut hits = Vec::new();
        let mut seen: HashSet<SymbolId> = HashSet::new();

        for &sid in self.exact.lookup(q) {
            if hits.len() >= limit {
                return hits;
            }
            if seen.insert(sid) {
                if let Some(record) = symbols.get(sid) {
                    hits.push(SearchHit {
                        record,
                        match_kind: MatchKind::Exact,
                        score: 10.0,
                    });
                }
            }
        }

        for sid in self.prefix.search(q, limit) {
            if hits.len() >= limit {
                return hits;
            }
            if seen.insert(sid) {
                if let Some(record) = symbols.get(sid) {
                    hits.push(SearchHit {
                        record,
                        match_kind: MatchKind::Prefix,
                        score: 5.0,
                    });
                }
            }
        }

        for (sid, score) in self.fuzzy.search(q, limit) {
            if hits.len() >= limit {
                return hits;
            }
            if seen.insert(sid) {
                if let Some(record) = symbols.get(sid) {
                    hits.push(SearchHit {
                        record,
                        match_kind: MatchKind::Fuzzy,
                        score,
                    });
                }
            }
        }

        hits
    }
}
