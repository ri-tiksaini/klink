use crate::csr::CsrGraph;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub fan_in: u32,
    pub fan_out: u32,
    /// Martin's instability metric: Ce / (Ce + Ca), in [0.0, 1.0].
    pub instability: f32,
}

pub fn compute_metrics(graph: &CsrGraph) -> Vec<NodeMetrics> {
    let n = graph.node_count as usize;
    let mut out = Vec::with_capacity(n);
    for u in 0..graph.node_count {
        let fan_in = graph.rev.degree(u);
        let fan_out = graph.fwd.degree(u);
        let total = fan_in + fan_out;
        let instability = if total == 0 {
            0.0
        } else {
            (fan_out as f32) / (total as f32)
        };
        out.push(NodeMetrics {
            fan_in,
            fan_out,
            instability,
        });
    }
    out
}
