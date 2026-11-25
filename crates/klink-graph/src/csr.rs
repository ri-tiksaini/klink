use klink_common::EdgeMeta;

/// Flat compressed sparse row adjacency arrays (cache-linear, zero pointer chasing).
#[derive(Clone, Debug, Default)]
pub struct CsrHalf {
    pub offsets: Vec<u32>,
    pub targets: Vec<u32>,
    pub meta: Vec<EdgeMeta>,
}

impl CsrHalf {
    #[inline]
    pub fn node_count(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    #[inline]
    pub fn edge_count(&self) -> usize {
        self.targets.len()
    }

    #[inline]
    pub fn neighbors(&self, u: u32) -> &[u32] {
        let idx = u as usize;
        let start = self.offsets[idx] as usize;
        let end = self.offsets[idx + 1] as usize;
        &self.targets[start..end]
    }

    #[inline]
    pub fn neighbor_meta(&self, u: u32) -> &[EdgeMeta] {
        let idx = u as usize;
        let start = self.offsets[idx] as usize;
        let end = self.offsets[idx + 1] as usize;
        &self.meta[start..end]
    }

    #[inline]
    pub fn degree(&self, u: u32) -> u32 {
        let idx = u as usize;
        self.offsets[idx + 1] - self.offsets[idx]
    }
}

/// Immutable forward + transposed reverse CSR graph snapshot.
#[derive(Clone, Debug)]
pub struct CsrGraph {
    pub node_count: u32,
    pub fwd: CsrHalf,
    pub rev: CsrHalf,
}
