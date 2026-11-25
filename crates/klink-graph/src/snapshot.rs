use crate::csr::CsrGraph;
use klink_common::EdgeMeta;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct GraphSnapshot {
    pub version: u64,
    pub graph: Arc<CsrGraph>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub added: Vec<(u32, u32)>,
    pub removed: Vec<(u32, u32)>,
    pub changed_meta: Vec<(u32, u32)>,
}

impl GraphSnapshot {
    pub fn new(version: u64, graph: CsrGraph) -> Self {
        Self {
            version,
            graph: Arc::new(graph),
        }
    }

    /// Sorted-key merge diff between two graph snapshots (O(V + E)).
    pub fn diff(&self, newer: &GraphSnapshot) -> SnapshotDiff {
        let mut old_edges: Vec<(u64, EdgeMeta)> = Vec::with_capacity(self.graph.fwd.edge_count());
        for u in 0..self.graph.node_count {
            let tgts = self.graph.fwd.neighbors(u);
            let metas = self.graph.fwd.neighbor_meta(u);
            for (&v, &m) in tgts.iter().zip(metas.iter()) {
                old_edges.push((((u as u64) << 32) | (v as u64), m));
            }
        }

        let mut new_edges: Vec<(u64, EdgeMeta)> = Vec::with_capacity(newer.graph.fwd.edge_count());
        for u in 0..newer.graph.node_count {
            let tgts = newer.graph.fwd.neighbors(u);
            let metas = newer.graph.fwd.neighbor_meta(u);
            for (&v, &m) in tgts.iter().zip(metas.iter()) {
                new_edges.push((((u as u64) << 32) | (v as u64), m));
            }
        }

        old_edges.sort_unstable_by_key(|&(k, _)| k);
        new_edges.sort_unstable_by_key(|&(k, _)| k);

        let mut diff = SnapshotDiff::default();
        let (mut i, mut j) = (0, 0);
        while i < old_edges.len() && j < new_edges.len() {
            let (ok, om) = old_edges[i];
            let (nk, nm) = new_edges[j];
            if ok < nk {
                diff.removed.push(((ok >> 32) as u32, ok as u32));
                i += 1;
            } else if nk < ok {
                diff.added.push(((nk >> 32) as u32, nk as u32));
                j += 1;
            } else {
                if om != nm {
                    diff.changed_meta.push(((nk >> 32) as u32, nk as u32));
                }
                i += 1;
                j += 1;
            }
        }
        while i < old_edges.len() {
            let ok = old_edges[i].0;
            diff.removed.push(((ok >> 32) as u32, ok as u32));
            i += 1;
        }
        while j < new_edges.len() {
            let nk = new_edges[j].0;
            diff.added.push(((nk >> 32) as u32, nk as u32));
            j += 1;
        }
        diff
    }
}
