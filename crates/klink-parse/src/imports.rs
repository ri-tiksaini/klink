use crate::path_trie::PathTrie;
use hashbrown::HashMap;
use klink_common::{Confidence, FileId};

/// Single-owner import and path resolver across all scanned files.
#[derive(Debug)]
pub struct ImportResolver {
    exact: HashMap<String, FileId>,
    trie: PathTrie,
    paths: Vec<String>,
}

const CANDIDATE_EXTS: &[&str] = &[
    "",
    ".rs",
    "/mod.rs",
    ".ts",
    ".tsx",
    ".mts",
    "/index.ts",
    "/index.tsx",
    ".js",
    ".jsx",
    ".mjs",
    "/index.js",
    ".py",
    "/__init__.py",
    ".h",
    ".hpp",
    ".c",
    ".cpp",
    ".go",
    ".java",
];

impl ImportResolver {
    pub fn new(normalized_paths: Vec<String>) -> Self {
        let mut exact = HashMap::with_capacity(normalized_paths.len());
        let mut trie = PathTrie::new();
        for (idx, p) in normalized_paths.iter().enumerate() {
            let id = FileId(idx as u32);
            exact.insert(p.clone(), id);
            trie.insert(p, id);
        }
        Self {
            exact,
            trie,
            paths: normalized_paths,
        }
    }

    /// Normalize a relative path against a source file's directory.
    pub fn normalize_relative(source_path: &str, raw: &str) -> String {
        let cleaned = raw.replace('\\', "/");
        let base_dir = source_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let mut parts: Vec<&str> = if cleaned.starts_with("./") || cleaned.starts_with("../") {
            base_dir.split('/').filter(|s| !s.is_empty()).collect()
        } else {
            Vec::new()
        };

        for comp in cleaned.split('/') {
            match comp {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                c => parts.push(c),
            }
        }
        parts.join("/")
    }

    /// Resolve a raw target string from `source` to a `(FileId, Confidence)`.
    pub fn resolve(
        &self,
        source: FileId,
        raw_target: &str,
        base_confidence: Confidence,
    ) -> Option<(FileId, Confidence)> {
        let src_path = self.paths.get(source.as_usize())?;
        let trimmed = raw_target.trim().trim_matches(|c| c == '"' || c == '\'' || c == '<' || c == '>');
        if trimmed.is_empty() {
            return None;
        }

        // Convert Rust `::` or Java/Python `.` module specifiers into slash candidates when appropriate
        let slash_target = if trimmed.contains("::") {
            let t = trimmed
                .trim_start_matches("crate::")
                .trim_start_matches("self::")
                .trim_start_matches("super::");
            t.replace("::", "/")
        } else if !trimmed.contains('/') && trimmed.contains('.') && !trimmed.starts_with('.') {
            // Keep filenames like `foo.h` intact, but convert `pkg.sub.Mod` -> `pkg/sub/Mod`
            let looks_like_ext = trimmed
                .rsplit_once('.')
                .map(|(_, ext)| matches!(ext, "c" | "h" | "hpp" | "cpp" | "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "java" | "json" | "toml" | "proto"))
                .unwrap_or(false);
            if looks_like_ext {
                trimmed.to_string()
            } else {
                trimmed.replace('.', "/")
            }
        } else if trimmed.starts_with('.') && !trimmed.starts_with("./") && !trimmed.starts_with("../") {
            // Python relative import like `.utils` or `..core.db`
            let dots = trimmed.bytes().take_while(|&b| b == b'.').count();
            let rest = &trimmed[dots..];
            let mut prefix = String::new();
            if dots == 1 {
                prefix.push_str("./");
            } else {
                for _ in 1..dots {
                    prefix.push_str("../");
                }
            }
            prefix.push_str(&rest.replace('.', "/"));
            prefix
        } else {
            trimmed.to_string()
        };

        let norm = Self::normalize_relative(src_path, &slash_target);

        // 1. Exact path match with candidate extensions
        for ext in CANDIDATE_EXTS {
            let cand = if ext.is_empty() {
                norm.clone()
            } else {
                format!("{norm}{ext}")
            };
            if let Some(&id) = self.exact.get(&cand) {
                if id != source {
                    return Some((id, base_confidence));
                }
            }
        }

        // 2. Reversed-component suffix trie lookup (handles crate/package prefixes)
        for ext in CANDIDATE_EXTS {
            let cand = if ext.is_empty() {
                norm.clone()
            } else {
                format!("{norm}{ext}")
            };
            if let Some((id, unique)) = self.trie.resolve_suffix(&cand) {
                if id != source {
                    let conf = if unique {
                        base_confidence
                    } else {
                        Confidence::Heuristic
                    };
                    return Some((id, conf));
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_rust_python_and_ts_imports() {
        let resolver = ImportResolver::new(vec![
            "src/main.rs".into(),
            "src/graph/csr.rs".into(),
            "web/app.ts".into(),
            "web/utils/math.ts".into(),
            "py/pkg/main.py".into(),
            "py/pkg/helper.py".into(),
        ]);

        assert_eq!(
            resolver.resolve(FileId(0), "crate::graph::csr", Confidence::Static),
            Some((FileId(1), Confidence::Static))
        );
        assert_eq!(
            resolver.resolve(FileId(2), "./utils/math", Confidence::Static),
            Some((FileId(3), Confidence::Static))
        );
        assert_eq!(
            resolver.resolve(FileId(4), ".helper", Confidence::Static),
            Some((FileId(5), Confidence::Static))
        );
    }
}
