/// Reusable chunked bump-allocated byte scratch arena reset between files.
#[derive(Debug)]
pub struct ScratchArena {
    buf: Vec<u8>,
    cursor: usize,
}

impl ScratchArena {
    pub fn with_capacity(bytes: usize) -> Self {
        Self {
            buf: vec![0u8; bytes.max(4096)],
            cursor: 0,
        }
    }

    /// Allocate aligned bytes inside the arena, growing if needed.
    pub fn alloc_slice(&mut self, data: &[u8], align: usize) -> &[u8] {
        let align = align.max(1);
        let aligned = (self.cursor + align - 1) & !(align - 1);
        let end = aligned + data.len();
        if end > self.buf.len() {
            let new_cap = end.next_power_of_two().max(self.buf.len() * 2);
            self.buf.resize(new_cap, 0);
        }
        self.buf[aligned..end].copy_from_slice(data);
        self.cursor = end;
        &self.buf[aligned..end]
    }

    #[inline]
    pub fn used(&self) -> usize {
        self.cursor
    }

    /// O(1) reset between files without freeing the backing buffer.
    #[inline]
    pub fn reset(&mut self) {
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_reset_reuses_memory() {
        let mut arena = ScratchArena::with_capacity(8192);
        let s = arena.alloc_slice(b"hello klink", 8);
        assert_eq!(s, b"hello klink");
        assert!(arena.used() >= 11);
        arena.reset();
        assert_eq!(arena.used(), 0);
    }
}
