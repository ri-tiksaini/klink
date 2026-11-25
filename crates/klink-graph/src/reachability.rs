use crate::condense::CondensedDag;
use crate::csr::CsrGraph;
use crate::impact_cache::ImpactCache;
use crate::scc::SccResult;
use klink_common::NodeBitmap;

#[derive(Clone, Debug)]
pub struct ImpactResult {
    pub root: u32,
    pub bitmap: NodeBitmap,
    pub direct: Vec<u32>,
    pub via_cycle: Vec<u32>,
    pub total_affected: u64,
    pub from_cache: bool,
}

/// Single-source impact engine: precomputed Roaring bitmaps for top 1% hubs + zero-alloc reverse-CSR BFS.
pub struct ImpactEngine {
    cache: ImpactCache,
    visited_stamp: Vec<u32>,
    queue: Vec<u32>,
    stamp: u32,
}

impl ImpactEngine {
    pub fn build(graph: &CsrGraph, max_cache_bytes: usize) -> Self {
        let n = graph.node_count as usize;
        let mut engine = Self {
            cache: ImpactCache::with_max_bytes(max_cache_bytes),
            visited_stamp: vec![0u32; n],
            queue: Vec::with_capacity(n.min(4096)),
            stamp: 0,
        };

        // Materialize closures for the top 1% highest fan-in hub nodes
        let hub_count = (n / 100).clamp(1, 256).min(n);
        if hub_count > 0 && n > 0 {
            let mut by_fanin: Vec<(u32, u32)> = (0..graph.node_count)
                .map(|u| (u, graph.rev.degree(u)))
                .filter(|&(_, d)| d > 0)
                .collect();
            by_fanin.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

            for (hub_id, _) in by_fanin.into_iter().take(hub_count) {
                let bm = engine.bfs_reverse_bitmap(graph, hub_id);
                engine.cache.insert(hub_id, bm);
            }
        }

        engine
    }

    fn bfs_reverse_bitmap(&mut self, graph: &CsrGraph, root: u32) -> NodeBitmap {
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp == 0 {
            self.visited_stamp.fill(0);
            self.stamp = 1;
        }
        let stamp = self.stamp;
        self.queue.clear();

        let mut bm = NodeBitmap::new();
        self.visited_stamp[root as usize] = stamp;
        self.queue.push(root);
        bm.insert(root);

        let mut head = 0;
        while head < self.queue.len() {
            let u = self.queue[head];
            head += 1;
            for &v in graph.rev.neighbors(u) {
                let vi = v as usize;
                if self.visited_stamp[vi] != stamp {
                    self.visited_stamp[vi] = stamp;
                    self.queue.push(v);
                    bm.insert(v);
                }
            }
        }
        bm
    }

    /// Answer single-source impact for `node_id` (click-to-glow hot path).
    pub fn query_impact(&mut self, graph: &CsrGraph, scc: &SccResult, node_id: u32) -> ImpactResult {
        let direct: Vec<u32> = graph.rev.neighbors(node_id).to_vec();
        let comp_id = scc.node_to_comp[node_id as usize] as usize;
        let via_cycle: Vec<u32> = if scc.components[comp_id].len() > 1 {
            scc.components[comp_id]
                .iter()
                .copied()
                .filter(|&x| x != node_id)
                .collect()
        } else {
            Vec::new()
        };

        if let Some(cached) = self.cache.get(node_id) {
            let bm = cached.clone();
            let total = bm.len().saturating_sub(1);
            return ImpactResult {
                root: node_id,
                bitmap: bm,
                direct,
                via_cycle,
                total_affected: total,
                from_cache: true,
            };
        }

        let bm = self.bfs_reverse_bitmap(graph, node_id);
        let total = bm.len().saturating_sub(1);
        if total >= 64 {
            self.cache.insert(node_id, bm.clone());
        }

        ImpactResult {
            root: node_id,
            bitmap: bm,
            direct,
            via_cycle,
            total_affected: total,
            from_cache: false,
        }
    }

    pub fn cached_hub_count(&self) -> usize {
        self.cache.len()
    }
}

/// Optional GRAIL-style randomized interval labels on the condensation DAG for O(d) pairwise pruning.
pub struct GrailIndex {
    dim: usize,
    /// `labels[node * dim + k] = (min_rank, post_rank)`
    labels: Vec<(u32, u32)>,
}

impl GrailIndex {
    pub fn build(dag: &CondensedDag, dim: usize) -> Self {
        let n = dag.graph.node_count as usize;
        let dim = dim.clamp(2, 5);
        let mut labels = vec![(u32::MAX, 0u32); n * dim];

        for k in 0..dim {
            let mut rank: u32 = 1;
            let mut visited = vec![false; n];
            let mut order: Vec<u32> = (0..n as u32).collect();
            // Deterministic permutation per dimension
            let mut seed: u32 = 0x9E37_79B9u32.wrapping_mul((k as u32) + 1);
            for i in (1..n).rev() {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                let j = (seed as usize) % (i + 1);
                order.swap(i, j);
            }

            let mut stack: Vec<(u32, usize)> = Vec::new();
            for start in order {
                if visited[start as usize] {
                    continue;
                }
                visited[start as usize] = true;
                stack.push((start, 0));

                while let Some((u, edge_idx)) = stack.last_mut() {
                    let u_val = *u;
                    let neighbors = dag.graph.fwd.neighbors(u_val);
                    if *edge_idx < neighbors.len() {
                        let w = neighbors[*edge_idx];
                        *edge_idx += 1;
                        if !visited[w as usize] {
                            visited[w as usize] = true;
                            stack.push((w, 0));
                        }
                    } else {
                        let mut min_r = rank;
                        for &w in neighbors {
                            let (w_min, _) = labels[w as usize * dim + k];
                            if w_min < min_r {
                                min_r = w_min;
                            }
                        }
                        labels[u_val as usize * dim + k] = (min_r, rank);
                        rank += 1;
                        stack.pop();
                    }
                }
            }
        }

        Self { dim, labels }
    }

    /// Returns `false` if `u_comp` definitely CANNOT reach `v_comp` in O(d); `true` means maybe.
    #[inline]
    pub fn may_reach(&self, u_comp: u32, v_comp: u32) -> bool {
        let ub = u_comp as usize * self.dim;
        let vb = v_comp as usize * self.dim;
        for k in 0..self.dim {
            let (u_min, u_max) = self.labels[ub + k];
            let (v_min, v_max) = self.labels[vb + k];
            if !(u_min <= v_min && v_max <= u_max) {
                return false;
            }
        }
        true
    }
}
