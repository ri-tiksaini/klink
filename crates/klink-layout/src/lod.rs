use crate::cluster::ClusterResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterLod {
    pub id: u32,
    pub name: String,
    pub cx: f32,
    pub cy: f32,
    pub radius: f32,
    pub member_count: u32,
}

/// Build folder-cluster LOD envelopes (centroid + bounding radius) for canvas overlays and quick-jump.
pub fn build_lod_clusters(pos: &[[f32; 2]], clusters: &ClusterResult) -> Vec<ClusterLod> {
    let c_count = clusters.cluster_count as usize;
    let mut sums = vec![[0.0f32; 2]; c_count];
    let mut counts = vec![0u32; c_count];

    for (u, &[x, y]) in pos.iter().enumerate() {
        let c = clusters.node_to_cluster[u] as usize;
        sums[c][0] += x;
        sums[c][1] += y;
        counts[c] += 1;
    }

    let mut centers = vec![[0.0f32; 2]; c_count];
    for c in 0..c_count {
        let cnt = counts[c].max(1) as f32;
        centers[c] = [sums[c][0] / cnt, sums[c][1] / cnt];
    }

    let mut max_r = vec![38.0f32; c_count];
    for (u, &[x, y]) in pos.iter().enumerate() {
        let c = clusters.node_to_cluster[u] as usize;
        let d = ((x - centers[c][0]).powi(2) + (y - centers[c][1]).powi(2)).sqrt() + 26.0;
        if d > max_r[c] {
            max_r[c] = d;
        }
    }

    (0..c_count)
        .map(|c| ClusterLod {
            id: c as u32,
            name: clusters
                .cluster_names
                .get(c)
                .cloned()
                .unwrap_or_else(|| format!("cluster_{c}")),
            cx: centers[c][0],
            cy: centers[c][1],
            radius: max_r[c],
            member_count: counts[c],
        })
        .collect()
}
