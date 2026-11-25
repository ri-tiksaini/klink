use crate::symbols::SymbolTable;
use hashbrown::HashMap;
use klink_common::{FileId, SymbolId};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct XrefSite {
    pub file: FileId,
    pub line: u32,
}

/// CSR-shaped posting lists mapping each SymbolId to all call/reference sites.
#[derive(Clone, Debug, Default)]
pub struct XrefIndex {
    pub offsets: Vec<u32>,
    pub sites: Vec<XrefSite>,
}

impl XrefIndex {
    pub fn build(symbol_count: usize, pairs: Vec<(SymbolId, XrefSite)>) -> Self {
        let mut counts = vec![0u32; symbol_count + 1];
        for (sid, _) in &pairs {
            let idx = sid.as_usize();
            if idx < symbol_count {
                counts[idx + 1] += 1;
            }
        }
        for i in 0..symbol_count {
            counts[i + 1] += counts[i];
        }

        let total = counts[symbol_count] as usize;
        let mut sites = vec![
            XrefSite {
                file: FileId(0),
                line: 0,
            };
            total
        ];
        let mut cur = counts.clone();
        for (sid, site) in pairs {
            let idx = sid.as_usize();
            if idx < symbol_count {
                let pos = cur[idx] as usize;
                cur[idx] += 1;
                sites[pos] = site;
            }
        }

        Self {
            offsets: counts,
            sites,
        }
    }

    #[inline]
    pub fn references(&self, sid: SymbolId) -> &[XrefSite] {
        let idx = sid.as_usize();
        if idx + 1 >= self.offsets.len() {
            return &[];
        }
        let s = self.offsets[idx] as usize;
        let e = self.offsets[idx + 1] as usize;
        &self.sites[s..e]
    }
}

/// Fast identifier scan matching source tokens against defined symbol names.
pub fn extract_symbol_refs(
    symbols: &SymbolTable,
    files: &[(FileId, String)],
) -> XrefIndex {
    let mut name_to_ids: HashMap<&str, Vec<SymbolId>> = HashMap::with_capacity(symbols.len());
    for (idx, name) in symbols.names.iter().enumerate() {
        if name.len() >= 3 {
            name_to_ids
                .entry(name.as_str())
                .or_default()
                .push(SymbolId(idx as u32));
        }
    }

    let mut pairs = Vec::new();
    for (file_id, content) in files {
        for (line_idx, line) in content.lines().enumerate() {
            let mut start = None;
            for (bi, ch) in line.char_indices() {
                let is_ident = ch.is_alphanumeric() || ch == '_';
                if is_ident && start.is_none() {
                    start = Some(bi);
                } else if !is_ident {
                    if let Some(s) = start.take() {
                        let tok = &line[s..bi];
                        if let Some(ids) = name_to_ids.get(tok) {
                            for &sid in ids {
                                if symbols.files[sid.as_usize()] != *file_id
                                    || symbols.spans[sid.as_usize()].0 != line_idx as u32
                                {
                                    pairs.push((
                                        sid,
                                        XrefSite {
                                            file: *file_id,
                                            line: line_idx as u32,
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            if let Some(s) = start {
                let tok = &line[s..];
                if let Some(ids) = name_to_ids.get(tok) {
                    for &sid in ids {
                        if symbols.files[sid.as_usize()] != *file_id
                            || symbols.spans[sid.as_usize()].0 != line_idx as u32
                        {
                            pairs.push((
                                sid,
                                XrefSite {
                                    file: *file_id,
                                    line: line_idx as u32,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }

    XrefIndex::build(symbols.len(), pairs)
}
