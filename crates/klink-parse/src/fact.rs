use klink_common::{Confidence, EdgeKind, FileId};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

/// Raw dependency fact emitted by a parser before single-owner resolution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    pub source: FileId,
    pub kind: EdgeKind,
    pub raw_target: String,
    pub span: Span,
    pub confidence: Confidence,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Type,
    Module,
    Constant,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolDef {
    pub file: FileId,
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
}
