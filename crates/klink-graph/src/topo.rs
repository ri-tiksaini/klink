use crate::condense::CondensedDag;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Deterministic Kahn topological sort with min-heap tiebreak.
pub fn kahn_topo_sort(dag: &CondensedDag) -> Vec<u32> {
    let n = dag.graph.node_count as usize;
    let mut in_deg = vec![0u32; n];
    let mut heap = BinaryHeap::new();

    for u in 0..n {
        let d = dag.graph.rev.degree(u as u32);
        in_deg[u] = d;
        if d == 0 {
            heap.push(Reverse(u as u32));
        }
    }

    let mut order = Vec::with_capacity(n);
    while let Some(Reverse(u)) = heap.pop() {
        order.push(u);
        for &v in dag.graph.fwd.neighbors(u) {
            let vi = v as usize;
            in_deg[vi] -= 1;
            if in_deg[vi] == 0 {
                heap.push(Reverse(v));
            }
        }
    }

    order
}
