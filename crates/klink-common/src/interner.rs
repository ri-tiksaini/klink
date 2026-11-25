use ahash::RandomState;
use hashbrown::HashTable;
use serde::{Deserialize, Serialize};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct InternedStr(pub u32);

#[derive(Copy, Clone, Debug)]
struct Span {
    start: u32,
    len: u32,
}

/// Contiguous byte-arena string interner using hashbrown + ahash.
#[derive(Debug)]
pub struct StringInterner {
    bytes: Vec<u8>,
    spans: Vec<Span>,
    table: HashTable<u32>,
    hasher: RandomState,
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(64 * 1024),
            spans: Vec::with_capacity(4096),
            table: HashTable::with_capacity(4096),
            hasher: RandomState::with_seeds(0x4B4C_494E, 0x1234_5678, 0x9E37_79B9, 0xDEAD_BEEF),
        }
    }

    #[inline]
    fn hash_str(&self, s: &str) -> u64 {
        self.hasher.hash_one(s.as_bytes())
    }

    pub fn intern(&mut self, s: &str) -> InternedStr {
        let hash = self.hash_str(s);
        let bytes = &self.bytes;
        let spans = &self.spans;

        if let Some(&id) = self.table.find(hash, |&idx| {
            let sp = spans[idx as usize];
            let slice = &bytes[sp.start as usize..(sp.start + sp.len) as usize];
            slice == s.as_bytes()
        }) {
            return InternedStr(id);
        }

        let id = self.spans.len() as u32;
        let start = self.bytes.len() as u32;
        let len = s.len() as u32;
        self.bytes.extend_from_slice(s.as_bytes());
        self.spans.push(Span { start, len });

        let hasher = &self.hasher;
        let bytes_ref = &self.bytes;
        let spans_ref = &self.spans;
        self.table.insert_unique(hash, id, |&existing_id| {
            let sp = spans_ref[existing_id as usize];
            let slice = &bytes_ref[sp.start as usize..(sp.start + sp.len) as usize];
            hasher.hash_one(slice)
        });

        InternedStr(id)
    }

    #[inline]
    pub fn resolve(&self, id: InternedStr) -> Option<&str> {
        let sp = *self.spans.get(id.0 as usize)?;
        let slice = &self.bytes[sp.start as usize..(sp.start + sp.len) as usize];
        std::str::from_utf8(slice).ok()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_and_dedup() {
        let mut interner = StringInterner::new();
        let a = interner.intern("src/core/engine.rs");
        let b = interner.intern("src/core/engine.rs");
        let c = interner.intern("src/graph/csr.rs");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(interner.resolve(a), Some("src/core/engine.rs"));
        assert_eq!(interner.resolve(c), Some("src/graph/csr.rs"));
        assert_eq!(interner.len(), 2);
    }
}
