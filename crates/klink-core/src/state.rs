use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanState {
    Idle = 0,
    Walking = 1,
    Parsing = 2,
    Resolving = 3,
    Building = 4,
    Indexing = 5,
    Layout = 6,
    Ready = 7,
    Cancelled = 8,
}

impl ScanState {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Walking,
            2 => Self::Parsing,
            3 => Self::Resolving,
            4 => Self::Building,
            5 => Self::Indexing,
            6 => Self::Layout,
            7 => Self::Ready,
            8 => Self::Cancelled,
            _ => Self::Idle,
        }
    }
}
