use klink_graph::CsrGraph;

#[derive(Clone, Debug)]
pub struct ForceConfig {
    pub theta: f32,
    pub charge: f32,
    pub link_distance: f32,
    pub alpha_decay: f32,
    pub min_alpha: f32,
    pub max_ticks: usize,
}

impl Default for ForceConfig {
    fn default() -> Self {
        Self {
            theta: 0.9,
            charge: -300.0,
            link_distance: 80.0,
            alpha_decay: 0.02,
            min_alpha: 1e-3,
            max_ticks: 200,
        }
    }
}

#[derive(Clone, Debug)]
struct QuadNode {
    cx: f32,
    cy: f32,
    half: f32,
    mass: f32,
    com_x: f32,
    com_y: f32,
    body: i32,          // -1 = empty, -2 = internal, >=0 = leaf node index
    children: [i32; 4], // NW, NE, SW, SE
}

impl QuadNode {
    fn empty(cx: f32, cy: f32, half: f32) -> Self {
        Self {
            cx,
            cy,
            half,
            mass: 0.0,
            com_x: 0.0,
            com_y: 0.0,
            body: -1,
            children: [-1; 4],
        }
    }
}

struct QuadTree {
    nodes: Vec<QuadNode>,
}

impl QuadTree {
    fn build(pos: &[[f32; 2]]) -> Self {
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &[x, y] in pos {
            if x < min_x { min_x = x; }
            if x > max_x { max_x = x; }
            if y < min_y { min_y = y; }
            if y > max_y { max_y = y; }
        }
        let cx = (min_x + max_x) * 0.5;
        let cy = (min_y + max_y) * 0.5;
        let half = ((max_x - min_x).max(max_y - min_y) * 0.5 + 1.0).max(10.0);

        let mut qt = Self {
            nodes: Vec::with_capacity(pos.len() * 2 + 16),
        };
        qt.nodes.push(QuadNode::empty(cx, cy, half));
        for (i, &[x, y]) in pos.iter().enumerate() {
            qt.insert(0, i as i32, x, y, 0);
        }
        qt
    }

    fn quadrant(node: &QuadNode, x: f32, y: f32) -> usize {
        let east = usize::from(x >= node.cx);
        let south = usize::from(y >= node.cy);
        (south << 1) | east
    }

    fn child_bounds(cx: f32, cy: f32, half: f32, q: usize) -> (f32, f32, f32) {
        let h2 = half * 0.5;
        let nx = if (q & 1) == 1 { cx + h2 } else { cx - h2 };
        let ny = if (q & 2) == 2 { cy + h2 } else { cy - h2 };
        (nx, ny, h2)
    }

    fn insert(&mut self, idx: usize, body: i32, x: f32, y: f32, depth: u8) {
        let old_mass = self.nodes[idx].mass;
        let new_mass = old_mass + 1.0;
        self.nodes[idx].com_x = (self.nodes[idx].com_x * old_mass + x) / new_mass;
        self.nodes[idx].com_y = (self.nodes[idx].com_y * old_mass + y) / new_mass;
        self.nodes[idx].mass = new_mass;

        let cur_body = self.nodes[idx].body;
        if cur_body == -1 {
            self.nodes[idx].body = body;
            return;
        }
        if depth >= 20 {
            return;
        }

        if cur_body >= 0 {
            let ox = self.nodes[idx].com_x;
            let oy = self.nodes[idx].com_y;
            self.nodes[idx].body = -2;
            let q_old = Self::quadrant(&self.nodes[idx], ox, oy);
            let child_old = self.ensure_child(idx, q_old);
            self.insert(child_old, cur_body, ox, oy, depth + 1);
        }

        let q_new = Self::quadrant(&self.nodes[idx], x, y);
        let child_new = self.ensure_child(idx, q_new);
        self.insert(child_new, body, x, y, depth + 1);
    }

    fn ensure_child(&mut self, parent: usize, q: usize) -> usize {
        let existing = self.nodes[parent].children[q];
        if existing >= 0 {
            return existing as usize;
        }
        let (cx, cy, half) = Self::child_bounds(
            self.nodes[parent].cx,
            self.nodes[parent].cy,
            self.nodes[parent].half,
            q,
        );
        let new_idx = self.nodes.len();
        self.nodes.push(QuadNode::empty(cx, cy, half));
        self.nodes[parent].children[q] = new_idx as i32;
        new_idx
    }

    fn repel(&self, idx: usize, body: i32, x: f32, y: f32, theta: f32, charge: f32, out: &mut [f32; 2]) {
        let node = &self.nodes[idx];
        if node.mass == 0.0 || node.body == body {
            return;
        }
        let dx = x - node.com_x;
        let dy = y - node.com_y;
        let dist2 = dx * dx + dy * dy + 16.0;
        let dist = dist2.sqrt();

        if node.body >= 0 || (node.half * 2.0) / dist < theta {
            let f = (-charge * node.mass) / dist2;
            out[0] += (dx / dist) * f;
            out[1] += (dy / dist) * f;
        } else {
            for &c in &node.children {
                if c >= 0 {
                    self.repel(c as usize, body, x, y, theta, charge, out);
                }
            }
        }
    }
}

/// Converge 2D positions via Fruchterman-Reingold + Barnes-Hut quadtree (O(V log V + E) per tick).
pub fn run_barnes_hut(graph: &CsrGraph, pos: &mut [[f32; 2]], cfg: &ForceConfig) -> usize {
    let n = pos.len();
    if n <= 1 {
        return 0;
    }

    let mut disp = vec![[0.0f32; 2]; n];
    let mut alpha = 1.0f32;
    let mut ticks = 0usize;

    for tick in 0..cfg.max_ticks {
        if alpha < cfg.min_alpha {
            break;
        }
        ticks = tick + 1;
        for d in &mut disp {
            *d = [0.0, 0.0];
        }

        let qt = QuadTree::build(pos);
        for i in 0..n {
            let mut f = [0.0f32; 2];
            qt.repel(0, i as i32, pos[i][0], pos[i][1], cfg.theta, cfg.charge, &mut f);
            // Gentle centering gravity
            disp[i][0] += f[0] - pos[i][0] * 0.015;
            disp[i][1] += f[1] - pos[i][1] * 0.015;
        }

        // Attractive spring forces along edges
        for u in 0..n {
            let pu = pos[u];
            for &v in graph.fwd.neighbors(u as u32) {
                let vi = v as usize;
                let dx = pos[vi][0] - pu[0];
                let dy = pos[vi][1] - pu[1];
                let dist = (dx * dx + dy * dy).sqrt().max(0.01);
                let pull = (dist - cfg.link_distance) * 0.08;
                let fx = (dx / dist) * pull;
                let fy = (dy / dist) * pull;
                disp[u][0] += fx;
                disp[u][1] += fy;
                disp[vi][0] -= fx;
                disp[vi][1] -= fy;
            }
        }

        let mut max_step = 0.0f32;
        for i in 0..n {
            let dx = disp[i][0] * alpha * 0.12;
            let dy = disp[i][1] * alpha * 0.12;
            let mag = (dx * dx + dy * dy).sqrt();
            let cap = 40.0 * alpha;
            let (sx, sy) = if mag > cap && mag > 0.0 {
                (dx / mag * cap, dy / mag * cap)
            } else {
                (dx, dy)
            };
            pos[i][0] += sx;
            pos[i][1] += sy;
            if mag > max_step {
                max_step = mag;
            }
        }

        alpha *= 1.0 - cfg.alpha_decay;
        if max_step < 0.01 {
            break;
        }
    }

    ticks
}
