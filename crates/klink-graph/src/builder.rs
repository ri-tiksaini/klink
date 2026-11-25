use crate::csr::{CsrGraph, CsrHalf};
use crate::facts::{dedup_edges, RawEdge};
use klink_common::EdgeMeta;

pub struct GraphBuilder;

impl GraphBuilder {
    /// Two-pass counting sort + prefix sum + scatter into forward and reverse CSR with zero reallocation.
    pub fn build(node_count: u32, mut edges: Vec<RawEdge>) -> CsrGraph {
        edges.retain(|e| e.src < node_count && e.dst < node_count && e.src != e.dst);
        dedup_edges(&mut edges);

        let n = node_count as usize;
        let m = edges.len();

        let mut fwd_offsets = vec![0u32; n + 1];
        let mut rev_offsets = vec![0u32; n + 1];
        for e in &edges {
            fwd_offsets[e.src as usize + 1] += 1;
            rev_offsets[e.dst as usize + 1] += 1;
        }
        for i in 0..n {
            fwd_offsets[i + 1] += fwd_offsets[i];
            rev_offsets[i + 1] += rev_offsets[i];
        }

        let mut fwd_targets = vec![0u32; m];
        let mut fwd_meta = vec![EdgeMeta::default(); m];
        let mut rev_targets = vec![0u32; m];
        let mut rev_meta = vec![EdgeMeta::default(); m];

        let mut fwd_cur = fwd_offsets.clone();
        let mut rev_cur = rev_offsets.clone();

        for e in &edges {
            let fi = fwd_cur[e.src as usize] as usize;
            fwd_cur[e.src as usize] += 1;
            fwd_targets[fi] = e.dst;
            fwd_meta[fi] = e.meta;

            let ri = rev_cur[e.dst as usize] as usize;
            rev_cur[e.dst as usize] += 1;
            rev_targets[ri] = e.src;
            rev_meta[ri] = e.meta;
        }

        CsrGraph {
            node_count,
            fwd: CsrHalf {
                offsets: fwd_offsets,
                targets: fwd_targets,
                meta: fwd_meta,
            },
            rev: CsrHalf {
                offsets: rev_offsets,
                targets: rev_targets,
                meta: rev_meta,
            },
        }
    }
}
