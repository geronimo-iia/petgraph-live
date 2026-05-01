use petgraph::visit::{IntoNeighbors, IntoNodeIdentifiers, NodeIndexable};

pub fn find_bridges<G>(_graph: G) -> Vec<(G::NodeId, G::NodeId)>
where
    G: IntoNodeIdentifiers + IntoNeighbors + NodeIndexable,
{
    todo!()
}
