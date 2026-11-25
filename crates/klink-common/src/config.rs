use crate::error::{KlinkError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Hard per-file byte limit (default 10 MB).
    pub max_file_bytes: u64,
    /// Shannon entropy threshold above which a file is skipped as obfuscated/minified.
    pub max_entropy: f64,
    /// Mean line length threshold above which a file is skipped as minified.
    pub max_mean_line_len: usize,
    /// Worker threads (0 = available_parallelism - 1).
    pub threads: usize,
    /// Additional glob patterns to ignore.
    pub ignore_patterns: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_file_bytes: 10 * 1024 * 1024,
            max_entropy: 6.4,
            max_mean_line_len: 320,
            threads: 0,
            ignore_patterns: vec![
                ".klink".to_string(),
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
        }
    }
}

impl ScanConfig {
    pub fn from_toml_str(s: &str) -> Result<Self> {
        let cfg: Self = toml::from_str(s).map_err(|e| KlinkError::Config(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        if self.max_file_bytes == 0 {
            return Err(KlinkError::Config("max_file_bytes must be > 0".into()));
        }
        if !(1.0..=8.0).contains(&self.max_entropy) {
            return Err(KlinkError::Config(
                "max_entropy must be between 1.0 and 8.0".into(),
            ));
        }
        if self.max_mean_line_len < 32 {
            return Err(KlinkError::Config(
                "max_mean_line_len must be >= 32".into(),
            ));
        }
        Ok(())
    }

    pub fn effective_threads(&self) -> usize {
        if self.threads > 0 {
            return self.threads;
        }
        std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(1)
    }
}
