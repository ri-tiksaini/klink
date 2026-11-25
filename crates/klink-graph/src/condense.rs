use crate::builder::GraphBuilder;
use crate::csr::CsrGraph;
use crate::facts::RawEdge;
use crate::scc::SccResult;
use klink_common::{Confidence, EdgeKind, EdgeMeta};

/// Acyclic condensation DAG where each node is an SCC supernode.
#[derive(Clone, Debug)]
pub struct CondensedDag {
    pub graph: CsrGraph,
}

pub fn condense_graph(graph: &CsrGraph, scc: &SccResult) -> CondensedDag {
    let comp_count = scc.components.len() as u32;
    let mut edges = Vec::new();
    let meta = EdgeMeta::new(Confidence::Static, EdgeKind::Import);

    for u in 0..graph.node_count {
        let cu = scc.node_to_comp[u as usize];
        for &v in graph.fwd.neighbors(u) {
            let cv = scc.node_to_comp[v as usize];
            if cu != cv {
                edges.push(RawEdge { src: cu, dst: cv, meta });
            }
        }
    }

    CondensedDag {
        graph: GraphBuilder::build(comp_count, edges),
    }
}
