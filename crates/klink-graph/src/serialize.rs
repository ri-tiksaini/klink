use crate::csr::CsrHalf;
use bytemuck::{cast_slice, Pod, Zeroable};
use klink_common::{EdgeMeta, KlinkError, Result};
use memmap2::Mmap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

const MAGIC: [u8; 8] = *b"KLINKCSR";
const VERSION: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CsrHeader {
    magic: [u8; 8],
    version: u32,
    node_count: u32,
    edge_count: u32,
    _reserved: u32,
}

/// Write an aligned flat binary CSR blob suitable for O(1) zero-copy mmap reads.
pub fn write_csr_file(path: &Path, half: &CsrHalf) -> Result<()> {
    let header = CsrHeader {
        magic: MAGIC,
        version: VERSION,
        node_count: half.node_count() as u32,
        edge_count: half.edge_count() as u32,
        _reserved: 0,
    };

    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    w.write_all(bytemuck::bytes_of(&header))?;
    w.write_all(cast_slice(&half.offsets))?;
    w.write_all(cast_slice(&half.targets))?;
    w.write_all(cast_slice(&half.meta))?;
    w.flush()?;
    Ok(())
}

/// Zero-copy memory-mapped CSR view (no deserialization pass on project open).
pub struct MmapCsrHalf {
    mmap: Mmap,
    node_count: usize,
    edge_count: usize,
}

impl MmapCsrHalf {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        // SAFETY: Klink writes `.klink/*.bin` atomically via temp-file + rename and treats blobs as immutable.
        let mmap = unsafe { Mmap::map(&file)? };
        let hdr_size = std::mem::size_of::<CsrHeader>();
        if mmap.len() < hdr_size {
            return Err(KlinkError::InvalidFormat("CSR file shorter than header".into()));
        }
        let hdr: &CsrHeader = bytemuck::from_bytes(&mmap[..hdr_size]);
        if hdr.magic != MAGIC || hdr.version != VERSION {
            return Err(KlinkError::InvalidFormat("Invalid CSR magic or version".into()));
        }
        let node_count = hdr.node_count as usize;
        let edge_count = hdr.edge_count as usize;
        let expected = hdr_size + (node_count + 1) * 4 + edge_count * 4 + edge_count * 4;
        if mmap.len() != expected {
            return Err(KlinkError::InvalidFormat(format!(
                "CSR length mismatch: got {}, expected {}",
                mmap.len(),
                expected
            )));
        }
        Ok(Self {
            mmap,
            node_count,
            edge_count,
        })
    }

    #[inline]
    pub fn offsets(&self) -> &[u32] {
        let start = std::mem::size_of::<CsrHeader>();
        let end = start + (self.node_count + 1) * 4;
        cast_slice(&self.mmap[start..end])
    }

    #[inline]
    pub fn targets(&self) -> &[u32] {
        let start = std::mem::size_of::<CsrHeader>() + (self.node_count + 1) * 4;
        let end = start + self.edge_count * 4;
        cast_slice(&self.mmap[start..end])
    }

    #[inline]
    pub fn meta(&self) -> &[EdgeMeta] {
        let start = std::mem::size_of::<CsrHeader>() + (self.node_count + 1) * 4 + self.edge_count * 4;
        let end = start + self.edge_count * 4;
        cast_slice(&self.mmap[start..end])
    }

    #[inline]
    pub fn neighbors(&self, u: u32) -> &[u32] {
        let off = self.offsets();
        let idx = u as usize;
        &self.targets()[off[idx] as usize..off[idx + 1] as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::GraphBuilder;
    use crate::facts::RawEdge;
    use klink_common::{Confidence, EdgeKind};

    #[test]
    fn zero_copy_mmap_roundtrip() {
        let meta = EdgeMeta::new(Confidence::Static, EdgeKind::Import);
        let g = GraphBuilder::build(
            4,
            vec![
                RawEdge { src: 0, dst: 1, meta },
                RawEdge { src: 0, dst: 2, meta },
                RawEdge { src: 2, dst: 3, meta },
            ],
        );
        let tmp = std::env::temp_dir().join("klink_test_csr_roundtrip.bin");
        write_csr_file(&tmp, &g.fwd).unwrap();
        let mapped = MmapCsrHalf::open(&tmp).unwrap();
        assert_eq!(mapped.neighbors(0), &[1, 2]);
        assert_eq!(mapped.neighbors(2), &[3]);
        assert_eq!(mapped.meta()[0].confidence(), Confidence::Static);
        let _ = std::fs::remove_file(&tmp);
    }
}
