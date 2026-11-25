use hashbrown::HashMap;
use klink_common::FileId;

#[derive(Default, Debug)]
struct TrieNode {
    children: HashMap<String, TrieNode>,
    file_ids: Vec<FileId>,
}

/// Reversed path-component suffix trie for fast right-hand path resolution.
#[derive(Default, Debug)]
pub struct PathTrie {
    root: TrieNode,
}

impl PathTrie {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, norm_path: &str, id: FileId) {
        let mut cur = &mut self.root;
        for comp in norm_path.split('/').rev().filter(|c| !c.is_empty()) {
            cur = cur.children.entry(comp.to_string()).or_default();
            cur.file_ids.push(id);
        }
    }

    /// Resolve a partial suffix path (e.g. `utils/math.rs` or `math.rs`).
    /// Returns `Some(FileId)` when a unique match exists (or first deterministic match if ambiguous).
    pub fn resolve_suffix(&self, suffix: &str) -> Option<(FileId, bool)> {
        let mut cur = &self.root;
        let mut matched_any = false;
        for comp in suffix.split('/').rev().filter(|c| !c.is_empty()) {
            cur = cur.children.get(comp)?;
            matched_any = true;
        }
        if !matched_any || cur.file_ids.is_empty() {
            return None;
        }
        let unique = cur.file_ids.len() == 1;
        Some((cur.file_ids[0], unique))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_trie_resolves_unique_and_flags_ambiguous() {
        let mut trie = PathTrie::new();
        trie.insert("src/core/engine.rs", FileId(1));
        trie.insert("src/net/client.rs", FileId(2));
        trie.insert("tests/net/client.rs", FileId(3));

        assert_eq!(trie.resolve_suffix("engine.rs"), Some((FileId(1), true)));
        assert_eq!(trie.resolve_suffix("core/engine.rs"), Some((FileId(1), true)));
        assert_eq!(trie.resolve_suffix("client.rs"), Some((FileId(2), false)));
        assert_eq!(trie.resolve_suffix("src/net/client.rs"), Some((FileId(2), true)));
    }
}
