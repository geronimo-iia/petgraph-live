use petgraph::visit::{GraphProp, IntoEdges, IntoNodeIdentifiers, NodeCount, NodeIndexable};

pub fn seidel<G>(_graph: G) -> Vec<Vec<u32>>
where
    G: IntoEdges + IntoNodeIdentifiers + NodeCount + NodeIndexable + GraphProp,
{
    todo!()
}

#[allow(dead_code)]
pub(crate) fn apd(_a: &[Vec<u32>]) -> Vec<Vec<u32>> {
    todo!()
}
