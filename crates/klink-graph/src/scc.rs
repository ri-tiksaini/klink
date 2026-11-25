use crate::csr::CsrGraph;

#[derive(Clone, Debug)]
pub struct SccResult {
    /// Component ID for each node in `0..node_count`.
    pub node_to_comp: Vec<u32>,
    /// Members of each strongly connected component.
    pub components: Vec<Vec<u32>>,
}

impl SccResult {
    /// Number of non-trivial cycles (SCCs with >= 2 nodes).
    pub fn cycle_count(&self) -> usize {
        self.components.iter().filter(|c| c.len() > 1).count()
    }
}

/// Iterative Tarjan SCC with explicit call stack (safe for 1,000+ deep dependency chains).
pub fn tarjan_scc_iterative(graph: &CsrGraph) -> SccResult {
    let n = graph.node_count as usize;
    let mut index_counter: u32 = 0;
    let mut indices = vec![u32::MAX; n];
    let mut lowlink = vec![0u32; n];
    let mut on_stack = vec![false; n];
    let mut tarjan_stack: Vec<u32> = Vec::with_capacity(n.min(1024));
    let mut call_stack: Vec<(u32, usize)> = Vec::with_capacity(n.min(1024));

    let mut node_to_comp = vec![0u32; n];
    let mut components: Vec<Vec<u32>> = Vec::new();

    for start in 0..n {
        if indices[start] != u32::MAX {
            continue;
        }

        indices[start] = index_counter;
        lowlink[start] = index_counter;
        index_counter += 1;
        tarjan_stack.push(start as u32);
        on_stack[start] = true;
        call_stack.push((start as u32, 0));

        while let Some((v, next_edge)) = call_stack.last_mut() {
            let v_val = *v;
            let neighbors = graph.fwd.neighbors(v_val);

            if *next_edge < neighbors.len() {
                let w = neighbors[*next_edge];
                *next_edge += 1;
                let wi = w as usize;

                if indices[wi] == u32::MAX {
                    indices[wi] = index_counter;
                    lowlink[wi] = index_counter;
                    index_counter += 1;
                    tarjan_stack.push(w);
                    on_stack[wi] = true;
                    call_stack.push((w, 0));
                } else if on_stack[wi] {
                    let vi = v_val as usize;
                    if indices[wi] < lowlink[vi] {
                        lowlink[vi] = indices[wi];
                    }
                }
            } else {
                let vi = v_val as usize;
                if lowlink[vi] == indices[vi] {
                    let comp_id = components.len() as u32;
                    let mut comp = Vec::new();
                    while let Some(w) = tarjan_stack.pop() {
                        on_stack[w as usize] = false;
                        node_to_comp[w as usize] = comp_id;
                        comp.push(w);
                        if w == v_val {
                            break;
                        }
                    }
                    components.push(comp);
                }
                let finished_low = lowlink[vi];
                call_stack.pop();
                if let Some((parent, _)) = call_stack.last() {
                    let pi = *parent as usize;
                    if finished_low < lowlink[pi] {
                        lowlink[pi] = finished_low;
                    }
                }
            }
        }
    }

    SccResult {
        node_to_comp,
        components,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::GraphBuilder;
    use crate::facts::RawEdge;
    use klink_common::{Confidence, EdgeKind, EdgeMeta};

    #[test]
    fn handles_1500_node_deep_cycle_without_stack_overflow() {
        let n: u32 = 1500;
        let meta = EdgeMeta::new(Confidence::Static, EdgeKind::Import);
        let edges: Vec<RawEdge> = (0..n)
            .map(|i| RawEdge {
                src: i,
                dst: (i + 1) % n,
                meta,
            })
            .collect();
        let g = GraphBuilder::build(n, edges);
        let scc = tarjan_scc_iterative(&g);
        assert_eq!(scc.components.len(), 1);
        assert_eq!(scc.components[0].len(), 1500);
        assert_eq!(scc.cycle_count(), 1);
    }
}
