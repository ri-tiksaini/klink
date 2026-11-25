pub mod exact;
pub mod fuzzy;
pub mod prefix;
pub mod search;
pub mod store;
pub mod symbols;
pub mod xref;

pub use exact::ExactIndex;
pub use fuzzy::FuzzyIndex;
pub use prefix::PrefixIndex;
pub use search::{MatchKind, SearchHit, UnifiedSearch};
pub use store::SymbolStore;
pub use symbols::{SymbolRecord, SymbolTable};
pub use xref::{extract_symbol_refs, XrefIndex, XrefSite};
