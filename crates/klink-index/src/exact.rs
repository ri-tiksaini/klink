use crate::symbols::SymbolTable;
use hashbrown::HashMap;
use klink_common::SymbolId;

/// O(1) SwissTable exact symbol lookup (case-insensitive normalized key).
#[derive(Default)]
pub struct ExactIndex {
    map: HashMap<String, Vec<SymbolId>>,
}

impl ExactIndex {
    pub fn build(symbols: &SymbolTable) -> Self {
        let mut map: HashMap<String, Vec<SymbolId>> = HashMap::with_capacity(symbols.len());
        for (idx, name) in symbols.names.iter().enumerate() {
            map.entry(name.to_ascii_lowercase())
                .or_default()
                .push(SymbolId(idx as u32));
        }
        Self { map }
    }

    pub fn lookup(&self, query: &str) -> &[SymbolId] {
        self.map
            .get(&query.to_ascii_lowercase())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}
