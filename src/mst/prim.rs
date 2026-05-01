use petgraph::algo::FloatMeasure;
use petgraph::visit::{IntoEdgeReferences, IntoEdges, IntoNodeIdentifiers, NodeCount, NodeIndexable};

pub fn prim<G, F, K>(_graph: G, _edge_cost: F) -> Vec<(G::NodeId, G::NodeId)>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + NodeCount + IntoEdges,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    todo!()
}
