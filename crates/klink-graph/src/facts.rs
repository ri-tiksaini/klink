use klink_common::EdgeMeta;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RawEdge {
    pub src: u32,
    pub dst: u32,
    pub meta: EdgeMeta,
}

/// Sort-merge deduplicate edges on `(src, dst)`, keeping highest-confidence edge (`Static < Heuristic < Unresolved`).
pub fn dedup_edges(edges: &mut Vec<RawEdge>) {
    edges.sort_unstable_by_key(|e| {
        (
            ((e.src as u64) << 32) | (e.dst as u64),
            e.meta.confidence() as u8,
            e.meta.kind() as u8,
        )
    });
    edges.dedup_by(|a, b| a.src == b.src && a.dst == b.dst);
}
