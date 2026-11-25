use hashbrown::HashMap;
use klink_common::NodeBitmap;

struct CacheEntry {
    bitmap: NodeBitmap,
    approx_bytes: usize,
    last_touch: u64,
}

/// Memory-bounded LRU cache of materialized Roaring bitmaps for high-fan-in hub nodes.
pub struct ImpactCache {
    entries: HashMap<u32, CacheEntry>,
    used_bytes: usize,
    max_bytes: usize,
    clock: u64,
}

impl ImpactCache {
    pub fn with_max_bytes(max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            used_bytes: 0,
            max_bytes: max_bytes.max(1024),
            clock: 0,
        }
    }

    pub fn default_128mb() -> Self {
        Self::with_max_bytes(128 * 1024 * 1024)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[inline]
    pub fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    pub fn get(&mut self, node_id: u32) -> Option<&NodeBitmap> {
        self.clock += 1;
        let tick = self.clock;
        if let Some(entry) = self.entries.get_mut(&node_id) {
            entry.last_touch = tick;
            return Some(&entry.bitmap);
        }
        None
    }

    pub fn insert(&mut self, node_id: u32, bitmap: NodeBitmap) {
        let approx_bytes = ((bitmap.len() as usize) / 2).max(64) + 64;
        if approx_bytes > self.max_bytes {
            return;
        }
        if let Some(old) = self.entries.remove(&node_id) {
            self.used_bytes = self.used_bytes.saturating_sub(old.approx_bytes);
        }

        while self.used_bytes + approx_bytes > self.max_bytes && !self.entries.is_empty() {
            let victim = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.last_touch)
                .map(|(&k, _)| k);
            if let Some(k) = victim {
                if let Some(rem) = self.entries.remove(&k) {
                    self.used_bytes = self.used_bytes.saturating_sub(rem.approx_bytes);
                }
            } else {
                break;
            }
        }

        self.clock += 1;
        self.used_bytes += approx_bytes;
        self.entries.insert(
            node_id,
            CacheEntry {
                bitmap,
                approx_bytes,
                last_touch: self.clock,
            },
        );
    }
}
