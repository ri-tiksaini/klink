use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// Three-valued confidence tag attached to every edge (packed in 2 bits).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Confidence {
    Static = 0,
    Heuristic = 1,
    Unresolved = 2,
}

impl Confidence {
    #[inline]
    pub const fn from_bits(bits: u32) -> Self {
        match bits & 0b11 {
            0 => Self::Static,
            1 => Self::Heuristic,
            _ => Self::Unresolved,
        }
    }
}

/// Semantic relation kind (packed in 4 bits).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    Import = 0,
    Include = 1,
    BuildDep = 2,
    GeneratedFrom = 3,
    TextRef = 4,
    DynamicRef = 5,
    BinaryImport = 6,
}

impl EdgeKind {
    #[inline]
    pub const fn from_bits(bits: u32) -> Self {
        match bits & 0b1111 {
            0 => Self::Import,
            1 => Self::Include,
            2 => Self::BuildDep,
            3 => Self::GeneratedFrom,
            4 => Self::TextRef,
            5 => Self::DynamicRef,
            6 => Self::BinaryImport,
            _ => Self::Import,
        }
    }
}

/// Packed 32-bit edge metadata word stored inline in CSR arrays.
/// Bits 0..2  = Confidence
/// Bits 2..6  = EdgeKind
/// Bits 6..32 = Reserved flags
#[repr(transparent)]
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
    Serialize, Deserialize, Pod, Zeroable
)]
pub struct EdgeMeta(pub u32);

impl EdgeMeta {
    #[inline]
    pub const fn new(confidence: Confidence, kind: EdgeKind) -> Self {
        Self((confidence as u32 & 0b11) | (((kind as u32) & 0b1111) << 2))
    }

    #[inline]
    pub const fn confidence(self) -> Confidence {
        Confidence::from_bits(self.0)
    }

    #[inline]
    pub const fn kind(self) -> EdgeKind {
        EdgeKind::from_bits(self.0 >> 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpack_roundtrip() {
        for &conf in &[Confidence::Static, Confidence::Heuristic, Confidence::Unresolved] {
            for &kind in &[
                EdgeKind::Import,
                EdgeKind::Include,
                EdgeKind::BuildDep,
                EdgeKind::GeneratedFrom,
                EdgeKind::TextRef,
                EdgeKind::DynamicRef,
                EdgeKind::BinaryImport,
            ] {
                let m = EdgeMeta::new(conf, kind);
                assert_eq!(m.confidence(), conf);
                assert_eq!(m.kind(), kind);
            }
        }
    }
}
