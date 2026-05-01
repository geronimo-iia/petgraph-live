use petgraph::visit::{IntoNeighbors, IntoNodeIdentifiers, NodeIndexable};

pub fn articulation_points<G>(_graph: G) -> Vec<G::NodeId>
where
    G: IntoNodeIdentifiers + IntoNeighbors + NodeIndexable,
{
    todo!()
}
