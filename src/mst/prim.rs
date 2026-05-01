use petgraph::algo::FloatMeasure;
use petgraph::visit::{
    EdgeRef, IntoEdgeReferences, IntoEdges, IntoNodeIdentifiers, NodeCount, NodeIndexable,
};

/// [Prim's algorithm](https://en.wikipedia.org/wiki/Prim%27s_algorithm)
/// for computing a minimum spanning tree (or forest).
///
/// The input graph is treated as undirected. Returns edges as `(NodeId, NodeId)` pairs.
///
/// # Examples
///
/// ```
/// use petgraph_live::mst::prim;
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
/// let mst = prim(&graph, |edge| *edge.weight());
/// assert_eq!(mst.len(), 5);
/// ```
pub fn prim<G, F, K>(graph: G, mut edge_cost: F) -> Vec<(G::NodeId, G::NodeId)>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + NodeCount + IntoEdges,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    let n = graph.node_count();
    let mut msf: Vec<(G::NodeId, G::NodeId)> = Vec::new();
    let mut used = vec![false; n];
    let mut used_count = 0usize;
    let mut sel_e = vec![n; n];
    let mut min_e = vec![K::infinite(); n];

    while used_count < n {
        let mut next_node = n;
        for node in 0..n {
            if !used[node] && (next_node == n || min_e[node] < min_e[next_node]) {
                next_node = node;
            }
        }

        if min_e[next_node] == K::infinite() {
            min_e[next_node] = K::zero();
            continue;
        }

        used[next_node] = true;
        used_count += 1;

        if sel_e[next_node] != graph.node_bound() {
            let pred = sel_e[next_node];
            msf.push((graph.from_index(next_node), graph.from_index(pred)));
        }

        for edge in graph.edges(graph.from_index(next_node)) {
            let to = graph.to_index(edge.target());
            if edge_cost(edge) < min_e[to] {
                min_e[to] = edge_cost(edge);
                sel_e[to] = next_node;
            }
        }
    }

    msf
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

    #[test]
    fn test_prim() {
        assert_eq!(
            prim(&graph1(), |edge| *edge.weight()),
            vec![
                (ni(4), ni(0)),
                (ni(1), ni(4)),
                (ni(3), ni(1)),
                (ni(2), ni(3))
            ]
        );
        assert_eq!(
            prim(&graph2(), |edge| *edge.weight()),
            vec![
                (ni(1), ni(0)),
                (ni(2), ni(1)),
                (ni(4), ni(3)),
                (ni(5), ni(4)),
                (ni(6), ni(5))
            ]
        );
        assert_eq!(
            prim(&graph1(), |edge| -*edge.weight()),
            vec![
                (ni(1), ni(0)),
                (ni(2), ni(1)),
                (ni(4), ni(0)),
                (ni(3), ni(4))
            ]
        );
        assert_eq!(
            prim(&graph2(), |edge| -*edge.weight()),
            vec![
                (ni(2), ni(0)),
                (ni(1), ni(2)),
                (ni(4), ni(3)),
                (ni(6), ni(4)),
                (ni(5), ni(4))
            ]
        );
    }
}
