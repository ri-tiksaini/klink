use crate::symbols::SymbolTable;
use hashbrown::HashMap;
use klink_common::SymbolId;

/// Trigram inverted index + rarest-first top-K pruning + zero-alloc bounded edit distance.
#[derive(Default)]
pub struct FuzzyIndex {
    trigrams: HashMap<[u8; 3], Vec<SymbolId>>,
    lower_names: Vec<String>,
}

fn collect_trigrams(s: &str) -> Vec<[u8; 3]> {
    let b = s.as_bytes();
    if b.len() < 3 { return Vec::new(); }
    let mut out = Vec::with_capacity(b.len() - 2);
    for w in b.windows(3) { out.push([w[0], w[1], w[2]]); }
    out.sort_unstable();
    out.dedup();
    out
}

fn edit_distance_bounded(a: &str, b: &str, max_dist: usize) -> usize {
    let (ab, bb) = (a.as_bytes(), b.as_bytes());
    if ab.len().abs_diff(bb.len()) > max_dist || bb.len() >= 128 {
        return max_dist + 1;
    }
    let mut prev = [0usize; 128];
    let mut curr = [0usize; 128];
    for j in 0..=bb.len() { prev[j] = j; }
    for (i, &ca) in ab.iter().enumerate() {
        curr[0] = i + 1;
        let mut row_min = curr[0];
        for (j, &cb) in bb.iter().enumerate() {
            let v = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + usize::from(ca != cb));
            curr[j + 1] = v;
            if v < row_min { row_min = v; }
        }
        if row_min > max_dist { return max_dist + 1; }
        prev[..=bb.len()].copy_from_slice(&curr[..=bb.len()]);
    }
    prev[bb.len()]
}

impl FuzzyIndex {
    pub fn build(symbols: &SymbolTable) -> Self {
        let mut trigrams: HashMap<[u8; 3], Vec<SymbolId>> = HashMap::new();
        let mut lower_names = Vec::with_capacity(symbols.len());
        for (idx, name) in symbols.names.iter().enumerate() {
            let lower = name.to_ascii_lowercase();
            let sid = SymbolId(idx as u32);
            for tri in collect_trigrams(&lower) {
                trigrams.entry(tri).or_default().push(sid);
            }
            lower_names.push(lower);
        }
        Self { trigrams, lower_names }
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<(SymbolId, f32)> {
        let q = query.to_ascii_lowercase();
        if q.len() < 3 || limit == 0 || self.lower_names.is_empty() {
            return Vec::new();
        }
        let mut q_tris = collect_trigrams(&q);
        q_tris.sort_unstable_by_key(|t| self.trigrams.get(t).map(|v| v.len()).unwrap_or(usize::MAX));

        let mut counts = vec![0u16; self.lower_names.len()];
        let mut touched: Vec<u32> = Vec::with_capacity(512);
        let active = &q_tris[..q_tris.len().min(8)];

        for tri in active {
            if let Some(postings) = self.trigrams.get(tri) {
                for &sid in postings {
                    let idx = sid.as_usize();
                    if counts[idx] == 0 { touched.push(sid.as_u32()); }
                    counts[idx] = counts[idx].saturating_add(1);
                }
            }
        }

        if touched.len() > 256 {
            touched.select_nth_unstable_by(255, |&a, &b| counts[b as usize].cmp(&counts[a as usize]));
            touched.truncate(256);
        }

        let max_dist = (q.len() / 3).clamp(1, 3);
        let mut scored: Vec<(SymbolId, f32)> = Vec::new();
        for id in touched {
            let overlap = counts[id as usize];
            let cand = &self.lower_names[id as usize];
            let contains_bonus = if cand.contains(&q) { 2.0 } else { 0.0 };
            let dist = if contains_bonus > 0.0 { 0 } else { edit_distance_bounded(&q, cand, max_dist) };
            if contains_bonus > 0.0 || dist <= max_dist {
                let tri_score = (overlap as f32) / (active.len().max(1) as f32);
                let len_penalty = 1.0 / (1.0 + (cand.len() as f32) * 0.03);
                scored.push((SymbolId(id), (contains_bonus + tri_score * 2.0 - (dist as f32) * 0.35) * len_penalty));
            }
        }

        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        scored.truncate(limit);
        scored
    }
}
