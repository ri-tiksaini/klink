use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_id {
    ($name:ident) => {
        #[repr(transparent)]
        #[derive(
            Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
            Serialize, Deserialize, Pod, Zeroable
        )]
        pub struct $name(pub u32);

        impl $name {
            #[inline]
            pub const fn new(raw: u32) -> Self {
                Self(raw)
            }

            #[inline]
            pub const fn as_u32(self) -> u32 {
                self.0
            }

            #[inline]
            pub const fn as_usize(self) -> usize {
                self.0 as usize
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

define_id!(FileId);
define_id!(SymbolId);
define_id!(ClusterId);
define_id!(EdgeIdx);
