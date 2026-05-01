//! Distance-based graph characteristics.
//!
//! Ported from [graphalgs](https://github.com/starovoid/graphalgs) (MIT).

pub fn eccentricity<G>(_graph: G, _node: G::NodeId) -> f32
where
    G: petgraph::visit::Visitable
        + petgraph::visit::NodeIndexable
        + petgraph::visit::IntoEdges
        + petgraph::visit::IntoNeighbors,
{
    todo!()
}

pub fn radius<G>(_graph: G) -> Option<f32>
where
    G: petgraph::visit::Visitable
        + petgraph::visit::NodeIndexable
        + petgraph::visit::IntoEdges
        + petgraph::visit::IntoNeighbors
        + petgraph::visit::IntoNodeIdentifiers
        + petgraph::visit::NodeCount,
{
    todo!()
}

pub fn diameter<G>(_graph: G) -> Option<f32>
where
    G: petgraph::visit::Visitable
        + petgraph::visit::NodeIndexable
        + petgraph::visit::IntoEdges
        + petgraph::visit::IntoNeighbors
        + petgraph::visit::IntoNodeIdentifiers
        + petgraph::visit::NodeCount,
{
    todo!()
}

pub fn center<G>(_graph: G) -> Vec<G::NodeId>
where
    G: petgraph::visit::Visitable
        + petgraph::visit::NodeIndexable
        + petgraph::visit::IntoEdges
        + petgraph::visit::IntoNodeIdentifiers,
{
    todo!()
}

pub fn periphery<G>(_graph: G) -> Vec<G::NodeId>
where
    G: petgraph::visit::Visitable
        + petgraph::visit::NodeIndexable
        + petgraph::visit::IntoEdges
        + petgraph::visit::IntoNodeIdentifiers,
{
    todo!()
}

pub fn girth<G>(_graph: G) -> Option<u32>
where
    G: petgraph::visit::Visitable
        + petgraph::visit::NodeIndexable
        + petgraph::visit::IntoEdges
        + petgraph::visit::IntoNodeIdentifiers
        + petgraph::visit::GraphProp,
{
    todo!()
}

pub fn weighted_eccentricity<G, F, K>(_graph: G, _node: G::NodeId, _edge_cost: F) -> Option<K>
where
    G: petgraph::visit::NodeCount
        + petgraph::visit::IntoNodeIdentifiers
        + petgraph::visit::IntoEdges
        + petgraph::visit::NodeIndexable,
    F: FnMut(G::EdgeRef) -> K,
    K: petgraph::algo::FloatMeasure,
{
    todo!()
}

pub fn weighted_radius<G, F, K>(_graph: G, _edge_cost: F) -> Option<K>
where
    G: petgraph::visit::IntoEdgeReferences
        + petgraph::visit::IntoNodeIdentifiers
        + petgraph::visit::NodeIndexable
        + petgraph::visit::NodeCount
        + petgraph::visit::GraphProp,
    F: FnMut(G::EdgeRef) -> K,
    K: petgraph::algo::FloatMeasure,
{
    todo!()
}

pub fn weighted_diameter<G, F, K>(_graph: G, _edge_cost: F) -> Option<K>
where
    G: petgraph::visit::IntoEdgeReferences
        + petgraph::visit::IntoNodeIdentifiers
        + petgraph::visit::NodeIndexable
        + petgraph::visit::NodeCount
        + petgraph::visit::GraphProp,
    F: FnMut(G::EdgeRef) -> K,
    K: petgraph::algo::FloatMeasure,
{
    todo!()
}

pub fn weighted_center<G, F, K>(_graph: G, _edge_cost: F) -> Vec<G::NodeId>
where
    G: petgraph::visit::IntoEdgeReferences
        + petgraph::visit::IntoNodeIdentifiers
        + petgraph::visit::NodeIndexable
        + petgraph::visit::NodeCount
        + petgraph::visit::GraphProp,
    F: FnMut(G::EdgeRef) -> K,
    K: petgraph::algo::FloatMeasure,
{
    todo!()
}

pub fn weighted_periphery<G, F, K>(_graph: G, _edge_cost: F) -> Vec<G::NodeId>
where
    G: petgraph::visit::IntoEdgeReferences
        + petgraph::visit::IntoNodeIdentifiers
        + petgraph::visit::NodeIndexable
        + petgraph::visit::NodeCount
        + petgraph::visit::GraphProp,
    F: FnMut(G::EdgeRef) -> K,
    K: petgraph::algo::FloatMeasure,
{
    todo!()
}
