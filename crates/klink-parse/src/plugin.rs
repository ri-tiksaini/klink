use serde::{Deserialize, Serialize};

/// Static enum dispatch for all 8 Tier-1 languages plus build/config/binary formats.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    C,
    Cpp,
    Rust,
    Go,
    Java,
    JavaScript,
    TypeScript,
    Python,
    BuildFile,
    ConfigText,
    Binary,
}

impl Language {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Python => "Python",
            Self::BuildFile => "BuildFile",
            Self::ConfigText => "ConfigText",
            Self::Binary => "Binary",
        }
    }

    pub const fn is_source(self) -> bool {
        matches!(
            self,
            Self::C
                | Self::Cpp
                | Self::Rust
                | Self::Go
                | Self::Java
                | Self::JavaScript
                | Self::TypeScript
                | Self::Python
        )
    }
}
