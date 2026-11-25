use hashbrown::HashMap;
use klink_graph::CsrGraph;

#[derive(Clone, Debug)]
pub struct ClusterResult {
    pub node_to_cluster: Vec<u32>,
    pub cluster_count: u32,
    pub cluster_sizes: Vec<u32>,
    pub cluster_names: Vec<String>,
}

/// Group files into distinct folder-based clusters (one visual cluster per directory).
/// Uses up to 2 leading directory segments (e.g. `crates/klink-graph`, `apps/desktop`, or `(root)`).
pub fn directory_clusters(file_paths: &[String]) -> ClusterResult {
    let mut name_to_id: HashMap<String, u32> = HashMap::new();
    let mut node_to_cluster = Vec::with_capacity(file_paths.len());
    let mut cluster_sizes: Vec<u32> = Vec::new();
    let mut cluster_names: Vec<String> = Vec::new();

    for path in file_paths {
        let dir_key = match path.rsplit_once('/') {
            None => "(root)".to_string(),
            Some((dir, _)) => {
                let parts: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
                match parts.len() {
                    0 => "(root)".to_string(),
                    1 => parts[0].to_string(),
                    _ => format!("{}/{}", parts[0], parts[1]),
                }
            }
        };

        let cid = *name_to_id.entry(dir_key.clone()).or_insert_with(|| {
            let id = cluster_names.len() as u32;
            cluster_names.push(dir_key);
            cluster_sizes.push(0);
            id
        });
        node_to_cluster.push(cid);
        cluster_sizes[cid as usize] += 1;
    }

    ClusterResult {
        node_to_cluster,
        cluster_count: cluster_names.len() as u32,
        cluster_sizes,
        cluster_names,
    }
}

/// Deterministic Louvain-style modularity optimization (resolution 1.0, capped at 100 passes).
pub fn louvain_clusters(graph: &CsrGraph, max_iters: usize) -> ClusterResult {
    let n = graph.node_count as usize;
    if n == 0 {
        return ClusterResult {
            node_to_cluster: Vec::new(),
            cluster_count: 0,
            cluster_sizes: Vec::new(),
            cluster_names: Vec::new(),
        };
    }

    let mut comm: Vec<u32> = (0..n as u32).collect();
    let mut deg = vec![0.0f64; n];
    let mut tot_weight = 0.0f64;

    for u in 0..n {
        let d = (graph.fwd.degree(u as u32) + graph.rev.degree(u as u32)) as f64;
        deg[u] = d;
        tot_weight += d;
    }
    let m2 = tot_weight.max(1.0);
    let mut comm_tot = deg.clone();

    let max_passes = max_iters.clamp(1, 100);
    let mut neigh_weights: HashMap<u32, f64> = HashMap::new();

    for _ in 0..max_passes {
        let mut moved = false;
        for u in 0..n {
            let cur_c = comm[u];
            let ku = deg[u];
            if ku == 0.0 {
                continue;
            }

            neigh_weights.clear();
            for &v in graph.fwd.neighbors(u as u32) {
                *neigh_weights.entry(comm[v as usize]).or_insert(0.0) += 1.0;
            }
            for &v in graph.rev.neighbors(u as u32) {
                *neigh_weights.entry(comm[v as usize]).or_insert(0.0) += 1.0;
            }

            comm_tot[cur_c as usize] -= ku;
            let mut best_c = cur_c;
            let mut best_gain = 0.0f64;

            let mut candidates: Vec<(u32, f64)> = neigh_weights.iter().map(|(&c, &w)| (c, w)).collect();
            candidates.sort_unstable_by_key(|&(c, _)| c);

            for (cand_c, k_in) in candidates {
                let gain = k_in - (comm_tot[cand_c as usize] * ku) / m2;
                if gain > best_gain + 1e-12 {
                    best_gain = gain;
                    best_c = cand_c;
                }
            }

            comm_tot[best_c as usize] += ku;
            if best_c != cur_c {
                comm[u] = best_c;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }

    let mut remap: HashMap<u32, u32> = HashMap::new();
    let mut node_to_cluster = vec![0u32; n];
    let mut cluster_sizes = Vec::new();

    for u in 0..n {
        let raw = comm[u];
        let dense = *remap.entry(raw).or_insert_with(|| {
            let id = cluster_sizes.len() as u32;
            cluster_sizes.push(0);
            id
        });
        node_to_cluster[u] = dense;
        cluster_sizes[dense as usize] += 1;
    }

    let cluster_names = (0..cluster_sizes.len())
        .map(|i| format!("cluster_{i}"))
        .collect();

    ClusterResult {
        node_to_cluster,
        cluster_count: cluster_sizes.len() as u32,
        cluster_sizes,
        cluster_names,
    }
}
