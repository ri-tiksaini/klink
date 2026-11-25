use crate::error::{KlinkError, Result};
use roaring::RoaringBitmap;

/// Compact set of u32 node IDs backed by Roaring bitmaps.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeBitmap {
    inner: RoaringBitmap,
}

impl NodeBitmap {
    pub fn new() -> Self {
        Self {
            inner: RoaringBitmap::new(),
        }
    }

    #[inline]
    pub fn insert(&mut self, id: u32) -> bool {
        self.inner.insert(id)
    }

    #[inline]
    pub fn contains(&self, id: u32) -> bool {
        self.inner.contains(id)
    }

    #[inline]
    pub fn len(&self) -> u64 {
        self.inner.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.inner.iter()
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(self.inner.serialized_size());
        self.inner
            .serialize_into(&mut out)
            .map_err(|e| KlinkError::Serialization(e.to_string()))?;
        Ok(out)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let inner = RoaringBitmap::deserialize_from(bytes)
            .map_err(|e| KlinkError::Serialization(e.to_string()))?;
        Ok(Self { inner })
    }
}
