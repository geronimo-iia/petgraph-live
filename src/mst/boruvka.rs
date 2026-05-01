use petgraph::algo::FloatMeasure;
use petgraph::unionfind::UnionFind;
use petgraph::visit::{
    EdgeRef, IntoEdgeReferences, IntoEdges, IntoNodeIdentifiers, NodeCount, NodeIndexable,
};

/// [Borůvka's algorithm](https://en.wikipedia.org/wiki/Bor%C5%AFvka%27s_algorithm)
/// for computing a minimum spanning tree (or forest).
///
/// The input graph is treated as undirected. Returns edges as a sorted `Vec<(NodeId, NodeId)>`
/// for determinism.
///
/// # Examples
///
/// ```
/// use petgraph_live::mst::boruvka;
/// use petgraph::graph::UnGraph;
///
/// let mut graph: UnGraph<(), f64> = UnGraph::new_undirected();
/// let n0 = graph.add_node(()); let n1 = graph.add_node(());
/// let n2 = graph.add_node(()); let n3 = graph.add_node(());
/// let n4 = graph.add_node(()); let n5 = graph.add_node(());
///
/// graph.add_edge(n0, n1, 10.0); graph.add_edge(n1, n3, 4.0);
/// graph.add_edge(n2, n3, -5.0); graph.add_edge(n2, n0, -2.0);
/// graph.add_edge(n2, n5, 6.0); graph.add_edge(n5, n4, 2.0);
/// graph.add_edge(n3, n4, 10.0);
///
/// let mst = boruvka(&graph, |edge| *edge.weight());
/// assert_eq!(mst.len(), 5);
/// ```
pub fn boruvka<G, F, K>(graph: G, mut edge_cost: F) -> Vec<(G::NodeId, G::NodeId)>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + NodeCount + IntoEdges,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    let n = graph.node_count();
    let mut components = UnionFind::<usize>::new(n);
    // Use (usize, usize) internally, convert at end
    let mut msf_raw: Vec<(usize, usize)> = Vec::new();

    loop {
        let mut min_edge_cost: Vec<K> = vec![K::infinite(); n];
        let mut min_edge: Vec<Option<(usize, usize)>> = vec![None; n];

        for edge in graph.edge_references() {
            let s = graph.to_index(edge.source());
            let t = graph.to_index(edge.target());
            let s_comp = components.find(s);
            let t_comp = components.find(t);
            let c = edge_cost(edge);

            if s_comp != t_comp {
                if c < min_edge_cost[s_comp] {
                    min_edge[s_comp] = Some((s, t));
                    min_edge_cost[s_comp] = c;
                }
                if c < min_edge_cost[t_comp] {
                    min_edge[t_comp] = Some((s, t));
                    min_edge_cost[t_comp] = c;
                }
            }
        }

        let mut union_occurred = false;
        for k in 0..n {
            if let Some((s, t)) = min_edge[components.find(k)]
                && components.union(s, t)
            {
                union_occurred = true;
                msf_raw.push((s, t));
            }
        }

        if !union_occurred {
            break;
        }
    }

    // Sort for determinism and convert to NodeId
    msf_raw.sort();
    msf_raw
        .into_iter()
        .map(|(s, t)| (graph.from_index(s), graph.from_index(t)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::{Graph, NodeIndex, UnGraph};

    fn graph1() -> Graph<(), f32> {
        let mut graph = Graph::<(), f32>::new();
        let n0 = graph.add_node(());
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());
        let n4 = graph.add_node(());

        graph.add_edge(n0, n1, 40.0);
        graph.add_edge(n0, n4, 18.0);
        graph.add_edge(n1, n0, 40.0);
        graph.add_edge(n1, n4, 15.0);
        graph.add_edge(n1, n2, 22.0);
        graph.add_edge(n1, n3, 6.0);
        graph.add_edge(n2, n1, 22.0);
        graph.add_edge(n2, n3, 14.0);
        graph.add_edge(n3, n4, 20.0);
        graph.add_edge(n3, n1, 6.0);
        graph.add_edge(n3, n2, 14.0);
        graph.add_edge(n4, n0, 18.0);
        graph.add_edge(n4, n1, 15.0);
        graph.add_edge(n4, n3, 20.0);

        graph
    }

    fn graph2() -> UnGraph<i8, f64> {
        let mut graph = UnGraph::<i8, f64>::new_undirected();
        let n0 = graph.add_node(0);
        let n1 = graph.add_node(1);
        let n2 = graph.add_node(2);
        let n3 = graph.add_node(3);
        let n4 = graph.add_node(4);
        let n5 = graph.add_node(5);
        let n6 = graph.add_node(6);

        graph.add_edge(n0, n2, 5.0);
        graph.add_edge(n1, n0, 1.0);
        graph.add_edge(n1, n2, 2.0);
        graph.add_edge(n3, n4, 2.0);
        graph.add_edge(n5, n4, 3.0);
        graph.add_edge(n4, n6, 6.0);
        graph.add_edge(n6, n5, 3.0);

        graph
    }

    fn ni(i: usize) -> NodeIndex {
        NodeIndex::new(i)
    }

    fn sorted(mut v: Vec<(NodeIndex, NodeIndex)>) -> Vec<(NodeIndex, NodeIndex)> {
        v.sort_by_key(|&(a, b)| (a.index().min(b.index()), a.index().max(b.index())));
        v
    }

    #[test]
    fn test_boruvka() {
        // Same MST as prim (same edge set, different order) — compare as sorted sets
        assert_eq!(
            sorted(boruvka(&graph1(), |edge| *edge.weight())),
            sorted(vec![(ni(0), ni(4)), (ni(1), ni(4)), (ni(1), ni(3)), (ni(2), ni(3))])
        );
        assert_eq!(
            sorted(boruvka(&graph2(), |edge| *edge.weight())),
            sorted(vec![(ni(1), ni(0)), (ni(1), ni(2)), (ni(3), ni(4)), (ni(5), ni(4)), (ni(6), ni(5))])
        );
        assert_eq!(
            sorted(boruvka(&graph1(), |edge| -*edge.weight())),
            sorted(vec![(ni(0), ni(1)), (ni(1), ni(2)), (ni(0), ni(4)), (ni(3), ni(4))])
        );
        assert_eq!(
            sorted(boruvka(&graph2(), |edge| -*edge.weight())),
            sorted(vec![(ni(0), ni(2)), (ni(1), ni(2)), (ni(3), ni(4)), (ni(4), ni(6)), (ni(5), ni(4))])
        );
    }
}
