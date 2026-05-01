use petgraph::visit::{IntoEdges, IntoNeighbors, NodeIndexable, VisitMap, Visitable};

/// The lengths of the shortest paths from the start vertex to all the others.
///
/// Based on BFS. Path length equals number of edges. Returns `Vec<f32>` indexed by
/// `NodeIndexable::to_index`. Unreachable nodes get `f32::INFINITY`.
/// Time complexity: **O(|V| + |E|)**.
///
/// # Examples
///
/// ```
/// use petgraph_live::shortest_path::shortest_distances;
/// use petgraph::Graph;
///
/// let inf = f32::INFINITY;
/// let graph = Graph::<u8, ()>::from_edges(&[(0, 1), (0, 2), (1, 2)]);
///
/// assert_eq!(shortest_distances(&graph, 0.into()), vec![0.0, 1.0, 1.0]);
/// assert_eq!(shortest_distances(&graph, 1.into()), vec![inf, 0.0, 1.0]);
/// ```
pub fn shortest_distances<G>(graph: G, start: G::NodeId) -> Vec<f32>
where
    G: Visitable + NodeIndexable + IntoEdges + IntoNeighbors,
{
    use std::collections::VecDeque;

    let mut visit_map = graph.visit_map();
    visit_map.visit(start);

    let mut dist = vec![f32::INFINITY; graph.node_bound()];
    dist[graph.to_index(start)] = 0.0;

    let mut queue: VecDeque<G::NodeId> = VecDeque::new();
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        for v in graph.neighbors(current) {
            if visit_map.visit(v) {
                queue.push_back(v);
                dist[graph.to_index(v)] = dist[graph.to_index(current)] + 1.0;
            }
        }
    }

    dist
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::Graph;

    fn graph1() -> Graph<u8, ()> {
        let mut graph = Graph::<u8, ()>::new();
        let n0 = graph.add_node(0);
        let n1 = graph.add_node(1);
        let n2 = graph.add_node(2);
        let n3 = graph.add_node(3);
        let n4 = graph.add_node(4);
        let n5 = graph.add_node(5);
        let n6 = graph.add_node(6);
        let n7 = graph.add_node(7);
        let n8 = graph.add_node(8);
        let n9 = graph.add_node(9);
        let n10 = graph.add_node(10);
        let n11 = graph.add_node(11);

        graph.add_edge(n0, n1, ());
        graph.add_edge(n0, n2, ());
        graph.add_edge(n2, n3, ());
        graph.add_edge(n2, n5, ());
        graph.add_edge(n3, n4, ());
        graph.add_edge(n4, n8, ());
        graph.add_edge(n5, n9, ());
        graph.add_edge(n5, n6, ());
        graph.add_edge(n6, n3, ());
        graph.add_edge(n6, n7, ());
        graph.add_edge(n6, n10, ());
        graph.add_edge(n7, n8, ());
        graph.add_edge(n7, n11, ());
        graph.add_edge(n8, n11, ());
        graph.add_edge(n9, n1, ());
        graph.add_edge(n9, n10, ());
        graph.add_edge(n10, n6, ());
        graph.add_edge(n11, n6, ());
        graph.add_edge(n11, n10, ());
        graph.add_edge(n0, n9, ());

        graph
    }

    #[test]
    fn test_shortest_distances() {
        let inf = f32::INFINITY;
        let g = graph1();

        assert_eq!(
            shortest_distances(&g, g.from_index(0)),
            vec![0.0, 1.0, 1.0, 2.0, 3.0, 2.0, 3.0, 4.0, 4.0, 1.0, 2.0, 5.0]
        );
        assert_eq!(
            shortest_distances(&g, g.from_index(1)),
            vec![inf, 0.0, inf, inf, inf, inf, inf, inf, inf, inf, inf, inf]
        );
        assert_eq!(
            shortest_distances(&g, g.from_index(2)),
            vec![inf, 3.0, 0.0, 1.0, 2.0, 1.0, 2.0, 3.0, 3.0, 2.0, 3.0, 4.0]
        );
    }

    #[test]
    fn test_shortest_distances_strongly_connected() {
        let mut graph = Graph::<u8, ()>::new();
        let n0 = graph.add_node(0);
        let n1 = graph.add_node(1);
        let n2 = graph.add_node(2);
        let n3 = graph.add_node(3);
        let n4 = graph.add_node(4);
        let n5 = graph.add_node(5);
        let n6 = graph.add_node(6);

        graph.add_edge(n0, n6, ());
        graph.add_edge(n0, n1, ());
        graph.add_edge(n1, n0, ());
        graph.add_edge(n1, n2, ());
        graph.add_edge(n1, n5, ());
        graph.add_edge(n1, n6, ());
        graph.add_edge(n2, n1, ());
        graph.add_edge(n2, n3, ());
        graph.add_edge(n3, n2, ());
        graph.add_edge(n3, n4, ());
        graph.add_edge(n4, n3, ());
        graph.add_edge(n4, n5, ());
        graph.add_edge(n5, n2, ());
        graph.add_edge(n5, n6, ());
        graph.add_edge(n5, n1, ());
        graph.add_edge(n5, n4, ());
        graph.add_edge(n6, n0, ());
        graph.add_edge(n6, n1, ());
        graph.add_edge(n6, n5, ());
        graph.add_edge(n2, n5, ());

        assert_eq!(
            shortest_distances(&graph, graph.from_index(0)),
            vec![0.0, 1.0, 2.0, 3.0, 3.0, 2.0, 1.0]
        );
        assert_eq!(
            shortest_distances(&graph, graph.from_index(1)),
            vec![1.0, 0.0, 1.0, 2.0, 2.0, 1.0, 1.0]
        );
    }
}
