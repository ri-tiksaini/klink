use klink_common::ScanConfig;

#[derive(Clone, Debug, PartialEq)]
pub enum SkipReason {
    TooLarge(u64),
    BinaryBlob,
    Minified { mean_line_len: usize },
    HighEntropy { entropy: f64 },
}

/// Shannon entropy (bits/byte, 0.0..=8.0) over up to the first 4 KB.
pub fn shannon_entropy(sample: &[u8]) -> f64 {
    if sample.is_empty() {
        return 0.0;
    }
    let slice = &sample[..sample.len().min(4096)];
    let mut counts = [0u32; 256];
    for &b in slice {
        counts[b as usize] += 1;
    }
    let len = slice.len() as f64;
    let mut entropy = 0.0;
    for &c in &counts {
        if c > 0 {
            let p = (c as f64) / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// Detect binary blobs, oversized files, minified bundles, or obfuscated blobs.
pub fn should_skip_file(bytes: &[u8], cfg: &ScanConfig) -> Option<SkipReason> {
    if (bytes.len() as u64) > cfg.max_file_bytes {
        return Some(SkipReason::TooLarge(bytes.len() as u64));
    }
    if bytes.is_empty() {
        return None;
    }

    let sample = &bytes[..bytes.len().min(4096)];
    if sample.contains(&0u8) {
        return Some(SkipReason::BinaryBlob);
    }

    // Only apply line-length / entropy checks to files large enough to avoid short-file noise
    if sample.len() >= 512 {
        let newlines = sample.iter().filter(|&&b| b == b'\n').count().max(1);
        let mean_line_len = sample.len() / newlines;
        if mean_line_len > cfg.max_mean_line_len {
            return Some(SkipReason::Minified { mean_line_len });
        }
        let ent = shannon_entropy(sample);
        if ent > cfg.max_entropy {
            return Some(SkipReason::HighEntropy { entropy: ent });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_minified_js_and_keeps_normal_source() {
        let cfg = ScanConfig::default();
        let minified = "var a=1;".repeat(200);
        assert!(matches!(
            should_skip_file(minified.as_bytes(), &cfg),
            Some(SkipReason::Minified { .. })
        ));

        let normal = "fn main() {\n    println!(\"hello\");\n}\n".repeat(25);
        assert_eq!(should_skip_file(normal.as_bytes(), &cfg), None);
    }
}
