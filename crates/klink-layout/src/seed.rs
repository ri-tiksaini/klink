use crate::cluster::ClusterResult;
use klink_graph::CsrGraph;

fn xorshift32(state: &mut u32) -> f32 {
    let mut x = *state;
    if x == 0 {
        x = 0x9E37_79B9;
    }
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x as f32) / (u32::MAX as f32)
}

pub fn graph_hash_seed(graph: &CsrGraph) -> u32 {
    let mut h: u32 = 0x811C_9DC5 ^ graph.node_count;
    h = h.wrapping_mul(0x0100_0193) ^ (graph.fwd.edge_count() as u32);
    for &t in graph.fwd.targets.iter().take(256) {
        h ^= t;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// Deterministic non-overlapping folder-island placement: cluster area scales with file count.
pub fn deterministic_seed_positions(graph: &CsrGraph, clusters: &ClusterResult) -> Vec<[f32; 2]> {
    let n = graph.node_count as usize;
    if n == 0 {
        return Vec::new();
    }

    let mut rng = graph_hash_seed(graph);
    let c_count = clusters.cluster_count.max(1) as usize;

    // Area-proportional radius per folder (uncapped so large folders never squeeze nodes)
    let mut radii = vec![44.0f32; c_count];
    for c in 0..c_count {
        let sz = clusters.cluster_sizes.get(c).copied().unwrap_or(1) as f32;
        radii[c] = 36.0 + sz.sqrt() * 26.0;
    }

    // Place largest folders first on a deterministic spiral and push outward until zero overlap
    let mut order: Vec<usize> = (0..c_count).collect();
    order.sort_by(|&a, &b| radii[b].total_cmp(&radii[a]).then_with(|| a.cmp(&b)));

    let mut centers = vec![[0.0f32; 2]; c_count];
    let mut placed: Vec<usize> = Vec::with_capacity(c_count);
    let golden = 2.3999632f32;
    let pad = 75.0f32;

    for (step, &c) in order.iter().enumerate() {
        if step == 0 {
            centers[c] = [0.0, 0.0];
            placed.push(c);
            continue;
        }
        let ang = (step as f32) * golden;
        let dir_x = ang.cos();
        let dir_y = ang.sin();
        let mut dist = radii[order[0]] + radii[c] + pad;

        // Spiral outward until guaranteed non-overlapping with all placed folder clusters
        for _ in 0..256 {
            let cx = dir_x * dist;
            let cy = dir_y * dist;
            let mut overlap = false;
            for &p in &placed {
                let dx = cx - centers[p][0];
                let dy = cy - centers[p][1];
                let min_d = radii[c] + radii[p] + pad;
                if dx * dx + dy * dy < min_d * min_d {
                    overlap = true;
                    break;
                }
            }
            if !overlap {
                centers[c] = [cx, cy];
                break;
            }
            dist += 32.0;
        }
        placed.push(c);
    }

    // Uniform-density sunflower phyllotaxis inside each folder cluster
    let mut local_idx = vec![0u32; c_count];
    let mut pos = vec![[0.0f32; 2]; n];
    for u in 0..n {
        let c = clusters.node_to_cluster[u] as usize;
        let k = local_idx[c];
        local_idx[c] += 1;
        let total = clusters.cluster_sizes.get(c).copied().unwrap_or(1).max(1) as f32;
        let max_r = (radii[c] - 18.0).max(12.0);
        let r = if total <= 1.0 {
            0.0
        } else {
            max_r * (((k as f32) + 0.5) / total).sqrt()
        };
        let ang = (k as f32) * golden + xorshift32(&mut rng) * 0.08;
        pos[u] = [
            centers[c][0] + ang.cos() * r,
            centers[c][1] + ang.sin() * r,
        ];
    }
    pos
}

/// Gentle intra-cluster refinement that preserves uniform minimum spacing between nodes.
pub fn refine_folder_clusters(graph: &CsrGraph, clusters: &ClusterResult, pos: &mut [[f32; 2]], iters: usize) {
    let n = pos.len();
    if n <= 1 {
        return;
    }
    let anchor = pos.to_vec();
    let mut disp = vec![[0.0f32; 2]; n];

    for _ in 0..iters.min(20) {
        for d in &mut disp {
            *d = [0.0, 0.0];
        }
        for u in 0..n {
            let cu = clusters.node_to_cluster[u];
            for &v in graph.fwd.neighbors(u as u32) {
                let vi = v as usize;
                if clusters.node_to_cluster[vi] != cu {
                    continue;
                }
                let dx = pos[vi][0] - pos[u][0];
                let dy = pos[vi][1] - pos[u][1];
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                // Keep a 24-unit minimum distance so connected nodes never collapse onto each other
                let pull = (dist - 28.0) * 0.025;
                disp[u][0] += (dx / dist) * pull;
                disp[u][1] += (dy / dist) * pull;
                disp[vi][0] -= (dx / dist) * pull;
                disp[vi][1] -= (dy / dist) * pull;
            }
            // Spring back toward uniform phyllotaxis slot so density stays even
            disp[u][0] += (anchor[u][0] - pos[u][0]) * 0.18;
            disp[u][1] += (anchor[u][1] - pos[u][1]) * 0.18;
        }
        for u in 0..n {
            pos[u][0] += disp[u][0].clamp(-4.0, 4.0);
            pos[u][1] += disp[u][1].clamp(-4.0, 4.0);
        }
    }
}
