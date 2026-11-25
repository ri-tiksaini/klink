use klink_graph::CsrGraph;

/// Incremental stress majorization with pinned node constraints (max 50 iters, tol 1e-4).
pub fn stress_majorization(
    graph: &CsrGraph,
    pos: &mut [[f32; 2]],
    pinned: &[bool],
    ideal_len: f32,
    max_iters: usize,
) -> usize {
    let n = pos.len();
    if n <= 1 {
        return 0;
    }

    let iters = max_iters.clamp(1, 50);
    let mut next_pos = pos.to_vec();
    let mut executed = 0;

    for it in 0..iters {
        executed = it + 1;
        let mut max_delta = 0.0f32;

        for u in 0..n {
            if pinned.get(u).copied().unwrap_or(false) {
                next_pos[u] = pos[u];
                continue;
            }

            let mut sx = 0.0f32;
            let mut sy = 0.0f32;
            let mut w_sum = 0.0f32;

            for &v in graph.fwd.neighbors(u as u32).iter().chain(graph.rev.neighbors(u as u32)) {
                let vi = v as usize;
                let dx = pos[u][0] - pos[vi][0];
                let dy = pos[u][1] - pos[vi][1];
                let dist = (dx * dx + dy * dy).sqrt().max(0.01);
                sx += pos[vi][0] + ideal_len * (dx / dist);
                sy += pos[vi][1] + ideal_len * (dy / dist);
                w_sum += 1.0;
            }

            if w_sum > 0.0 {
                let nx = sx / w_sum;
                let ny = sy / w_sum;
                let delta = ((nx - pos[u][0]).powi(2) + (ny - pos[u][1]).powi(2)).sqrt();
                if delta > max_delta {
                    max_delta = delta;
                }
                next_pos[u] = [nx, ny];
            } else {
                next_pos[u] = pos[u];
            }
        }

        pos.copy_from_slice(&next_pos);
        if max_delta < 1e-4 {
            break;
        }
    }

    executed
}
