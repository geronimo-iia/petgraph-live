use petgraph::algo::{FloatMeasure, NegativeCycle};
use petgraph::visit::{GraphProp, IntoEdgeReferences, IntoNodeIdentifiers, NodeIndexable};
use std::collections::HashMap;
use std::hash::Hash;

pub fn floyd_warshall<G, F, K>(_graph: G, _edge_cost: F) -> Result<Vec<Vec<K>>, NegativeCycle>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + GraphProp,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    todo!()
}

pub fn distance_map<G, F, K>(_graph: G, _edge_cost: F) -> HashMap<(G::NodeId, G::NodeId), K>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + GraphProp,
    G::NodeId: Eq + Hash,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    todo!()
}
