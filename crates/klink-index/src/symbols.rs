use klink_common::{FileId, SymbolId};
use klink_parse::{SymbolDef, SymbolKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolRecord {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub file: FileId,
    pub start_line: u32,
    pub end_line: u32,
}

/// Struct-of-arrays symbol table for O(1) lookup by SymbolId.
#[derive(Clone, Debug, Default)]
pub struct SymbolTable {
    pub names: Vec<String>,
    pub kinds: Vec<SymbolKind>,
    pub files: Vec<FileId>,
    pub spans: Vec<(u32, u32)>,
}

impl SymbolTable {
    pub fn from_defs(defs: Vec<SymbolDef>) -> Self {
        let cap = defs.len();
        let mut tbl = Self {
            names: Vec::with_capacity(cap),
            kinds: Vec::with_capacity(cap),
            files: Vec::with_capacity(cap),
            spans: Vec::with_capacity(cap),
        };
        for d in defs {
            tbl.names.push(d.name);
            tbl.kinds.push(d.kind);
            tbl.files.push(d.file);
            tbl.spans.push((d.span.start, d.span.end));
        }
        tbl
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn get(&self, id: SymbolId) -> Option<SymbolRecord> {
        let i = id.as_usize();
        if i >= self.names.len() {
            return None;
        }
        Some(SymbolRecord {
            id,
            name: self.names[i].clone(),
            kind: self.kinds[i],
            file: self.files[i],
            start_line: self.spans[i].0,
            end_line: self.spans[i].1,
        })
    }
}
