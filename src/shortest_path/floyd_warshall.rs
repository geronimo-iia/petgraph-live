use petgraph::algo::{FloatMeasure, NegativeCycle};
use petgraph::visit::{EdgeRef, GraphProp, IntoEdgeReferences, IntoNodeIdentifiers, NodeIndexable};
use std::collections::HashMap;
use std::hash::Hash;

/// [Floyd–Warshall algorithm](https://en.wikipedia.org/wiki/Floyd%E2%80%93Warshall_algorithm)
/// for all pairs shortest path problem.
///
/// Computes shortest paths in a weighted graph with positive or negative edge weights,
/// but with no negative cycles. Multiple edges and self-loops allowed.
///
/// # Examples
///
/// ```
/// use petgraph_live::shortest_path::floyd_warshall;
/// use petgraph::Graph;
///
/// let inf = f32::INFINITY;
///
/// let graph = Graph::<(), f32>::from_edges(&[
///     (0, 1, 2.0), (1, 2, 10.0), (1, 3, -5.0),
///     (3, 2, 2.0), (2, 3, 20.0),
/// ]);
///
/// assert_eq!(
///     floyd_warshall(&graph, |edge| *edge.weight()),
///     Ok(vec![vec![0.0, 2.0, -1.0, -3.0],
///             vec![f32::INFINITY, 0.0, -3.0, -5.0],
///             vec![f32::INFINITY, f32::INFINITY,  0.0, 20.0],
///             vec![f32::INFINITY, f32::INFINITY,  2.0,  0.0]])
/// );
/// ```
pub fn floyd_warshall<G, F, K>(graph: G, mut edge_cost: F) -> Result<Vec<Vec<K>>, NegativeCycle>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + GraphProp,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    let n = graph.node_bound();
    let mut dist = vec![vec![K::infinite(); n]; n];

    for i in graph.node_identifiers() {
        dist[graph.to_index(i)][graph.to_index(i)] = K::zero();
    }

    for edge in graph.edge_references() {
        let s = graph.to_index(edge.source());
        let t = graph.to_index(edge.target());
        let c = edge_cost(edge);
        if c < dist[s][t] {
            dist[s][t] = c;
            if !graph.is_directed() {
                dist[t][s] = c;
            }
        }
    }

    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                if dist[i][k] + dist[k][j] < dist[i][j] {
                    dist[i][j] = dist[i][k] + dist[k][j];
                }
            }
        }
    }

    for (i, row) in dist.iter().enumerate() {
        if row[i] < K::zero() {
            return Err(NegativeCycle(()));
        }
    }

    Ok(dist)
}

/// Convert a graph and edge-cost closure into an all-pairs distance hashmap.
///
/// Runs Floyd–Warshall internally and maps each `(NodeId, NodeId)` pair to its
/// shortest distance.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use petgraph_live::shortest_path::distance_map;
/// use petgraph::prelude::NodeIndex;
/// use petgraph::Graph;
///
/// let graph = Graph::<(), f32>::from_edges(&[
///     (0, 1, 2.0), (1, 2, 10.0), (1, 3, -5.0),
///     (3, 2, 2.0), (2, 3, 20.0),
/// ]);
///
/// let dm = distance_map(&graph, |edge| *edge.weight()).unwrap();
/// assert_eq!(dm[&(NodeIndex::new(0), NodeIndex::new(1))], 2.0);
/// assert_eq!(dm[&(NodeIndex::new(1), NodeIndex::new(3))], -5.0);
/// ```
type DistMap<N, K> = HashMap<(N, N), K>;

pub fn distance_map<G, F, K>(
    graph: G,
    edge_cost: F,
) -> Result<DistMap<G::NodeId, K>, NegativeCycle>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + GraphProp,
    G::NodeId: Eq + Hash,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    let dist_matrix = floyd_warshall(graph, edge_cost)?;
    let mut map = HashMap::new();
    for (i, distances) in dist_matrix.iter().enumerate() {
        for (j, &d) in distances.iter().enumerate() {
            map.insert((graph.from_index(i), graph.from_index(j)), d);
        }
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::{Graph, NodeIndex};
    use petgraph::Undirected;

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

    fn graph2() -> Graph<(), f32, Undirected> {
        let mut graph = Graph::<(), f32, Undirected>::new_undirected();
        let n0 = graph.add_node(());
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());
        let n4 = graph.add_node(());
        let n5 = graph.add_node(());
        let n6 = graph.add_node(());
        let n7 = graph.add_node(());

        graph.add_edge(n0, n1, 1.0);
        graph.add_edge(n1, n4, 5.0);
        graph.add_edge(n4, n1, 5.0);
        graph.add_edge(n2, n1, 8.0);
        graph.add_edge(n4, n3, 10.0);
        graph.add_edge(n3, n2, 0.0);
        graph.add_edge(n5, n6, 5.0);
        graph.add_edge(n5, n7, 44.0);
        graph.add_edge(n6, n7, 1.0);

        graph
    }

    fn graph3() -> Graph<(), f64> {
        let mut graph = Graph::<(), f64>::new();
        let n0 = graph.add_node(());
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());

        graph.add_edge(n0, n1, 10.0);
        graph.add_edge(n0, n2, 5.0);
        graph.add_edge(n1, n2, 2.0);
        graph.add_edge(n2, n3, -10.0);
        graph.add_edge(n3, n1, -1.0);
        graph.add_edge(n1, n3, 16.0);

        graph
    }

    fn graph4() -> Graph<(), f32> {
        let mut graph = Graph::<(), f32>::new();
        graph.add_node(());
        graph
    }

    fn graph6() -> Graph<(), f32> {
        let mut graph = Graph::<(), f32>::new();
        let n0 = graph.add_node(());
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());

        graph.add_edge(n0, n1, 1.0);
        graph.add_edge(n1, n0, -10.0);
        graph.add_edge(n2, n2, 5.0);
        graph
    }

    #[test]
    fn test_floyd_warshall_empty_graph() {
        let graph = Graph::<(), f32>::new();
        assert_eq!(floyd_warshall(&graph, |edge| *edge.weight()), Ok(vec![]));
    }

    #[test]
    fn test_floyd_warshall_single_node() {
        assert_eq!(
            floyd_warshall(&graph4(), |edge| *edge.weight()),
            Ok(vec![vec![0.0]])
        );
    }

    #[test]
    fn test_floyd_warshall_one_component() {
        assert_eq!(
            floyd_warshall(&graph1(), |edge| *edge.weight()),
            Ok(vec![
                vec![0.0, 33.0, 52.0, 38.0, 18.0],
                vec![33.0, 0.0, 20.0, 6.0, 15.0],
                vec![52.0, 20.0, 0.0, 14.0, 34.0],
                vec![38.0, 6.0, 14.0, 0.0, 20.0],
                vec![18.0, 15.0, 34.0, 20.0, 0.0]
            ])
        );
    }

    #[test]
    fn test_floyd_warshall_two_components() {
        let inf = f32::INFINITY;
        assert_eq!(
            floyd_warshall(&graph2(), |edge| *edge.weight()),
            Ok(vec![
                vec![0.0, 1.0, 9.0, 9.0, 6.0, inf, inf, inf],
                vec![1.0, 0.0, 8.0, 8.0, 5.0, inf, inf, inf],
                vec![9.0, 8.0, 0.0, 0.0, 10.0, inf, inf, inf],
                vec![9.0, 8.0, 0.0, 0.0, 10.0, inf, inf, inf],
                vec![6.0, 5.0, 10.0, 10.0, 0.0, inf, inf, inf],
                vec![inf, inf, inf, inf, inf, 0.0, 5.0, 6.0],
                vec![inf, inf, inf, inf, inf, 5.0, 0.0, 1.0],
                vec![inf, inf, inf, inf, inf, 6.0, 1.0, 0.0],
            ])
        );
    }

    #[test]
    fn test_floyd_warshall_negative_cycle() {
        assert_eq!(
            floyd_warshall(&graph3(), |edge| *edge.weight()),
            Err(NegativeCycle(()))
        );
        assert_eq!(
            floyd_warshall(&graph6(), |edge| *edge.weight()),
            Err(NegativeCycle(()))
        );
        let mut graph = graph1();
        graph.add_edge(3.into(), 3.into(), -5.0);
        assert_eq!(
            floyd_warshall(&graph, |edge| *edge.weight()),
            Err(NegativeCycle(()))
        );
    }

    #[test]
    fn test_distance_map_empty() {
        let graph = Graph::<(), f32>::new();
        assert_eq!(
            distance_map(&graph, |edge| *edge.weight()),
            Ok(HashMap::new())
        );
    }

    #[test]
    fn test_distance_map_single_node() {
        let graph = graph4();
        let mut expected = HashMap::new();
        expected.insert((graph.from_index(0), graph.from_index(0)), 0.0f32);
        assert_eq!(
            distance_map(&graph, |edge| *edge.weight()),
            Ok(expected)
        );
    }

    #[test]
    fn test_distance_map() {
        let graph = graph1();
        let expected: HashMap<(NodeIndex, NodeIndex), f32> = [
            ((0.into(), 0.into()), 0.0),
            ((0.into(), 1.into()), 33.0),
            ((0.into(), 2.into()), 52.0),
            ((0.into(), 3.into()), 38.0),
            ((0.into(), 4.into()), 18.0),
            ((1.into(), 0.into()), 33.0),
            ((1.into(), 1.into()), 0.0),
            ((1.into(), 2.into()), 20.0),
            ((1.into(), 3.into()), 6.0),
            ((1.into(), 4.into()), 15.0),
            ((2.into(), 0.into()), 52.0),
            ((2.into(), 1.into()), 20.0),
            ((2.into(), 2.into()), 0.0),
            ((2.into(), 3.into()), 14.0),
            ((2.into(), 4.into()), 34.0),
            ((3.into(), 0.into()), 38.0),
            ((3.into(), 1.into()), 6.0),
            ((3.into(), 2.into()), 14.0),
            ((3.into(), 3.into()), 0.0),
            ((3.into(), 4.into()), 20.0),
            ((4.into(), 0.into()), 18.0),
            ((4.into(), 1.into()), 15.0),
            ((4.into(), 2.into()), 34.0),
            ((4.into(), 3.into()), 20.0),
            ((4.into(), 4.into()), 0.0),
        ]
        .iter()
        .cloned()
        .collect();

        assert_eq!(distance_map(&graph, |edge| *edge.weight()), Ok(expected));
    }
}
