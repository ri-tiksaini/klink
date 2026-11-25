use bytemuck::{Pod, Zeroable};
use klink_graph::{CsrGraph, SccResult};
use std::f32::consts::TAU;

/// Exact 32-byte aligned GPU vertex instance ready for direct WebGL2 VBO upload.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuNodeInstance {
    pub pos_x: f32,
    pub pos_y: f32,
    pub radius: f32,
    pub rgba8: u32,
    pub phase: f32,
    pub amplitude: f32,
    pub meta: u32,       // bits 0..24 = nodeId, 24..28 = langId, 28 = inCycle
    pub cluster_id: u32,
}

/// Exact 28-byte aligned GPU edge instance ready for WebGL2 instanced line/strip rendering.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuEdgeInstance {
    pub src_x: f32,
    pub src_y: f32,
    pub dst_x: f32,
    pub dst_y: f32,
    pub src_id: u32,
    pub dst_id: u32,
    pub conf: u32,
}

pub fn lang_rgba8(lang_id: u8) -> u32 {
    let (r, g, b) = match lang_id {
        0 => (249u32, 115, 22),  // Rust
        1 => (96, 165, 250),     // C
        2 => (129, 140, 248),    // C++
        3 => (52, 211, 153),     // Python
        4 => (56, 189, 248),     // TypeScript
        5 => (250, 204, 21),     // JavaScript
        6 => (45, 212, 191),     // Go
        7 => (251, 113, 133),    // Java
        _ => (148, 163, 184),    // Build/Config
    };
    r | (g << 8) | (b << 16) | (255 << 24)
}

pub fn pack_nodes(
    pos: &[[f32; 2]],
    centrality: &[f32],
    lang_ids: &[u8],
    clusters: &[u32],
    scc: &SccResult,
) -> Vec<GpuNodeInstance> {
    let mut out = Vec::with_capacity(pos.len());
    for i in 0..pos.len() {
        let c = centrality.get(i).copied().unwrap_or(0.0);
        let l = lang_ids.get(i).copied().unwrap_or(8);
        let comp = scc.node_to_comp.get(i).copied().unwrap_or(0) as usize;
        let in_cycle = u32::from(scc.components.get(comp).map(|v| v.len() > 1).unwrap_or(false));
        let phase = ((i as u32).wrapping_mul(2654435761) as f32 / u32::MAX as f32) * TAU;
        let amp = 0.8 + (((i as u32).wrapping_mul(1597334677) & 0xFFFF) as f32 / 65535.0) * 1.6;
        let meta = ((i as u32) & 0xFF_FFFF) | (((l as u32) & 0xF) << 24) | (in_cycle << 28);

        out.push(GpuNodeInstance {
            pos_x: pos[i][0],
            pos_y: pos[i][1],
            radius: 3.5 + c * 12.0,
            rgba8: lang_rgba8(l),
            phase,
            amplitude: amp,
            meta,
            cluster_id: clusters.get(i).copied().unwrap_or(0),
        });
    }
    out
}

pub fn pack_edges(graph: &CsrGraph, pos: &[[f32; 2]], max_edges: usize) -> Vec<GpuEdgeInstance> {
    let cap = graph.fwd.edge_count().min(max_edges);
    let mut out = Vec::with_capacity(cap);
    'outer: for u in 0..graph.node_count {
        let tgts = graph.fwd.neighbors(u);
        let metas = graph.fwd.neighbor_meta(u);
        let pu = pos[u as usize];
        for (&v, &m) in tgts.iter().zip(metas.iter()) {
            if out.len() >= cap {
                break 'outer;
            }
            let pv = pos[v as usize];
            out.push(GpuEdgeInstance {
                src_x: pu[0],
                src_y: pu[1],
                dst_x: pv[0],
                dst_y: pv[1],
                src_id: u,
                dst_id: v,
                conf: m.confidence() as u32,
            });
        }
    }
    out
}
