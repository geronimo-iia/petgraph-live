use petgraph::visit::{IntoEdges, IntoNeighbors, NodeIndexable, Visitable};

pub fn shortest_distances<G>(_graph: G, _start: G::NodeId) -> Vec<f32>
where
    G: Visitable + NodeIndexable + IntoEdges + IntoNeighbors,
{
    todo!()
}
