use klink_graph::{kahn_topo_sort, CondensedDag};

/// Sugiyama-style layered coordinate assignment on the condensation DAG.
pub fn sugiyama_layered(dag: &CondensedDag, layer_spacing: f32, node_sep: f32) -> Vec<[f32; 2]> {
    let n = dag.graph.node_count as usize;
    if n == 0 {
        return Vec::new();
    }

    let topo = kahn_topo_sort(dag);
    let mut layer = vec![0usize; n];
    let mut max_layer = 0usize;

    for &u in &topo {
        let lu = layer[u as usize];
        for &v in dag.graph.fwd.neighbors(u) {
            let vi = v as usize;
            if lu + 1 > layer[vi] {
                layer[vi] = lu + 1;
                if layer[vi] > max_layer {
                    max_layer = layer[vi];
                }
            }
        }
    }

    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); max_layer + 1];
    for (u, &l) in layer.iter().enumerate() {
        buckets[l].push(u as u32);
    }

    // One barycenter sweep to reduce edge crossings
    let mut x_order = vec![0.0f32; n];
    for (idx, &u) in buckets[0].iter().enumerate() {
        x_order[u as usize] = idx as f32;
    }
    for l in 1..=max_layer {
        buckets[l].sort_by(|&a, &b| {
            let avg = |u: u32| -> f32 {
                let preds = dag.graph.rev.neighbors(u);
                if preds.is_empty() {
                    return u as f32;
                }
                let sum: f32 = preds.iter().map(|&p| x_order[p as usize]).sum();
                sum / preds.len() as f32
            };
            avg(a).total_cmp(&avg(b)).then_with(|| a.cmp(&b))
        });
        for (idx, &u) in buckets[l].iter().enumerate() {
            x_order[u as usize] = idx as f32;
        }
    }

    let mut pos = vec![[0.0f32; 2]; n];
    for (l, nodes) in buckets.iter().enumerate() {
        let half_w = (nodes.len().saturating_sub(1) as f32) * node_sep * 0.5;
        let y = (l as f32) * layer_spacing;
        for (idx, &u) in nodes.iter().enumerate() {
            pos[u as usize] = [(idx as f32) * node_sep - half_w, y];
        }
    }
    pos
}
