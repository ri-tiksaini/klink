use crate::csr::CsrGraph;

/// Brandes betweenness centrality normalized to `[0.0, 1.0]`.
/// Exact when `node_count <= 5_000`; deterministic `sqrt(V)` source sampling above 5,000 nodes.
pub fn brandes_betweenness(graph: &CsrGraph) -> Vec<f32> {
    let n = graph.node_count as usize;
    if n == 0 {
        return Vec::new();
    }

    let sources: Vec<u32> = if n <= 5_000 {
        (0..n as u32).collect()
    } else {
        let k = (n as f64).sqrt().ceil() as usize;
        let step = (n / k.max(1)).max(1);
        (0..n as u32).step_by(step).take(k).collect()
    };

    let mut cb = vec![0.0f64; n];
    let mut stack: Vec<u32> = Vec::with_capacity(n);
    let mut preds: Vec<Vec<u32>> = vec![Vec::new(); n];
    let mut sigma = vec![0.0f64; n];
    let mut dist = vec![-1i32; n];
    let mut delta = vec![0.0f64; n];
    let mut queue: Vec<u32> = Vec::with_capacity(n);

    for s in sources {
        stack.clear();
        queue.clear();
        for i in 0..n {
            preds[i].clear();
            sigma[i] = 0.0;
            dist[i] = -1;
            delta[i] = 0.0;
        }

        let si = s as usize;
        sigma[si] = 1.0;
        dist[si] = 0;
        queue.push(s);

        let mut head = 0;
        while head < queue.len() {
            let v = queue[head];
            head += 1;
            stack.push(v);
            let vi = v as usize;
            let dv = dist[vi];

            for &w in graph.fwd.neighbors(v) {
                let wi = w as usize;
                if dist[wi] < 0 {
                    queue.push(w);
                    dist[wi] = dv + 1;
                }
                if dist[wi] == dv + 1 {
                    sigma[wi] += sigma[vi];
                    preds[wi].push(v);
                }
            }
        }

        while let Some(w) = stack.pop() {
            let wi = w as usize;
            let sw = sigma[wi];
            if sw > 0.0 {
                let factor = (1.0 + delta[wi]) / sw;
                for &v in &preds[wi] {
                    let vi = v as usize;
                    delta[vi] += sigma[vi] * factor;
                }
            }
            if w != s {
                cb[wi] += delta[wi];
            }
        }
    }

    let max_val = cb.iter().copied().fold(0.0f64, f64::max);
    if max_val <= 0.0 {
        return vec![0.0f32; n];
    }
    cb.into_iter().map(|v| (v / max_val) as f32).collect()
}
