//! Distance-based graph characteristics.
//!
//! Ported from [graphalgs](https://github.com/starovoid/graphalgs) (MIT).

use std::collections::{HashSet, VecDeque};

use crate::shortest_path::{floyd_warshall, shortest_distances};
use petgraph::algo::FloatMeasure;
use petgraph::visit::{
    GraphProp, IntoEdgeReferences, IntoEdges, IntoNeighbors, IntoNodeIdentifiers, NodeCount,
    NodeIndexable, Visitable,
};

/// Vertex eccentricity.
///
/// The maximum shortest-path distance from `node` to any other vertex.
/// Returns `f32::INFINITY` if the graph is not strongly connected from `node`.
///
/// # Examples
///
/// ```
/// use petgraph_live::metrics::eccentricity;
/// use petgraph::Graph;
///
/// let graph = Graph::<(), ()>::from_edges(&[(0, 1), (1, 0), (1, 2)]);
///
/// assert_eq!(eccentricity(&graph, 0.into()), 2.0);
/// assert_eq!(eccentricity(&graph, 1.into()), 1.0);
/// assert_eq!(eccentricity(&graph, 2.into()), f32::INFINITY);
/// ```
pub fn eccentricity<G>(graph: G, node: G::NodeId) -> f32
where
    G: Visitable + NodeIndexable + IntoEdges + IntoNeighbors,
{
    *shortest_distances(graph, node)
        .iter()
        .max_by(|x, y| x.partial_cmp(y).unwrap())
        .unwrap()
}

/// Graph radius.
///
/// The minimum eccentricity over all vertices. Returns `None` for empty graph.
///
/// # Examples
///
/// ```
/// use petgraph_live::metrics::radius;
/// use petgraph::Graph;
///
/// let graph = Graph::<(), ()>::from_edges(&[(0, 1), (1, 0), (1, 2)]);
///
/// assert_eq!(radius(&graph), Some(1.0));
/// ```
pub fn radius<G>(graph: G) -> Option<f32>
where
    G: Visitable + NodeIndexable + IntoEdges + IntoNeighbors + IntoNodeIdentifiers + NodeCount,
{
    if graph.node_count() == 0 {
        return None;
    }
    graph
        .node_identifiers()
        .map(|i| eccentricity(graph, i))
        .min_by(|x, y| x.partial_cmp(y).unwrap())
}

/// Graph diameter.
///
/// The maximum eccentricity over all vertices. Returns `None` for empty graph.
///
/// # Examples
///
/// ```
/// use petgraph_live::metrics::diameter;
/// use petgraph::Graph;
///
/// let graph = Graph::<(), ()>::from_edges(&[(0, 1), (1, 0), (1, 2)]);
///
/// assert_eq!(diameter(&graph), Some(f32::INFINITY));
/// ```
pub fn diameter<G>(graph: G) -> Option<f32>
where
    G: Visitable + NodeIndexable + IntoEdges + IntoNeighbors + IntoNodeIdentifiers + NodeCount,
{
    if graph.node_count() == 0 {
        return None;
    }
    let mut diam = 0f32;
    for i in graph.node_identifiers() {
        diam = diam.max(eccentricity(graph, i));
        if diam == f32::INFINITY {
            break;
        }
    }
    Some(diam)
}

/// Central vertices of the graph.
///
/// Returns vertices with minimum eccentricity. Empty vec for empty graph.
///
/// # Examples
///
/// ```
/// use petgraph_live::metrics::center;
/// use petgraph::Graph;
///
/// let graph = Graph::<(), ()>::from_edges(&[(0, 1), (1, 0), (1, 2)]);
///
/// assert_eq!(center(&graph), vec![1.into()]);
/// ```
pub fn center<G>(graph: G) -> Vec<G::NodeId>
where
    G: Visitable + NodeIndexable + IntoEdges + IntoNodeIdentifiers,
{
    let ecc = graph
        .node_identifiers()
        .map(|i| eccentricity(graph, i))
        .collect::<Vec<f32>>();

    match ecc.iter().min_by(|x, y| x.partial_cmp(y).unwrap()) {
        None => vec![],
        Some(&r) => graph
            .node_identifiers()
            .enumerate()
            .filter(|(i, _)| ecc[*i] == r)
            .map(|(_, node_id)| node_id)
            .collect(),
    }
}

/// Peripheral graph vertices.
///
/// Returns vertices with maximum eccentricity. Empty vec for empty graph.
///
/// # Examples
///
/// ```
/// use petgraph_live::metrics::periphery;
/// use petgraph::Graph;
///
/// let graph = Graph::<(), ()>::from_edges(&[(0, 1), (1, 0), (1, 2)]);
///
/// assert_eq!(periphery(&graph), vec![2.into()]);
/// ```
pub fn periphery<G>(graph: G) -> Vec<G::NodeId>
where
    G: Visitable + NodeIndexable + IntoEdges + IntoNodeIdentifiers,
{
    let ecc = graph
        .node_identifiers()
        .map(|i| eccentricity(graph, i))
        .collect::<Vec<f32>>();

    match ecc.iter().max_by(|x, y| x.partial_cmp(y).unwrap()) {
        None => vec![],
        Some(&d) => graph
            .node_identifiers()
            .enumerate()
            .filter(|(i, _)| ecc[*i] == d)
            .map(|(_, node_id)| node_id)
            .collect(),
    }
}

/// Girth of a simple graph.
///
/// Returns `None` if the graph is acyclic, otherwise `Some(length)` of the shortest cycle.
///
/// # Examples
///
/// ```
/// use petgraph_live::metrics::girth;
/// use petgraph::graph::UnGraph;
///
/// let mut g = UnGraph::new_undirected();
/// let n0 = g.add_node(());
/// let n1 = g.add_node(());
/// let n2 = g.add_node(());
/// let n3 = g.add_node(());
/// g.add_edge(n0, n1, ());
/// g.add_edge(n1, n2, ());
/// g.add_edge(n2, n3, ());
///
/// assert_eq!(girth(&g), None); // acyclic
///
/// g.add_edge(n3, n0, ());
/// assert_eq!(girth(&g), Some(4));
/// ```
pub fn girth<G>(graph: G) -> Option<u32>
where
    G: Visitable + NodeIndexable + IntoEdges + IntoNodeIdentifiers + GraphProp,
{
    let mut best: Option<u32> = None;

    if graph.is_directed() {
        let mut stack = Vec::<usize>::new();
        let mut used = vec![false; graph.node_bound()];

        for start in 0..graph.node_bound() {
            if used[start] {
                continue;
            }
            stack.push(start);
            let mut depth = vec![0usize; graph.node_bound()];
            let mut predecessors = (0..graph.node_bound())
                .map(|_| HashSet::<usize>::new())
                .collect::<Vec<HashSet<usize>>>();

            while let Some(current) = stack.pop() {
                if !used[current] {
                    used[current] = true;
                    let d = depth[current];

                    for nb in graph.neighbors(graph.from_index(current)) {
                        let v = graph.to_index(nb);
                        if used[v] {
                            if predecessors[current].contains(&v) {
                                let candidate = (depth[current] - depth[v] + 1) as u32;
                                best = Some(best.map_or(candidate, |b| b.min(candidate)));
                            }
                        } else {
                            depth[v] = d + 1;
                            stack.push(v);
                            predecessors[v] = predecessors[v]
                                .union(&predecessors[current])
                                .cloned()
                                .collect();
                            predecessors[v].insert(current);
                        }
                    }
                }
                if best == Some(2) {
                    return best;
                }
            }
        }
    } else {
        for start in 0..graph.node_bound() {
            let mut queue = VecDeque::<usize>::new();
            queue.push_back(start);

            let mut used = vec![false; graph.node_bound()];
            let mut depth = vec![0usize; graph.node_bound()];
            let mut inp = vec![None; graph.node_bound()];

            while !queue.is_empty() {
                let current = queue.pop_front().unwrap();
                let d = depth[current] + 1;

                for nb in graph.neighbors(graph.from_index(current)) {
                    let v = graph.to_index(nb);
                    if used[v] {
                        if inp[current] == Some(v) {
                            continue;
                        }
                        let candidate = if depth[v] == d - 1 {
                            (d * 2 - 1) as u32
                        } else if depth[v] == d {
                            (d * 2) as u32
                        } else {
                            continue;
                        };
                        best = Some(best.map_or(candidate, |b| b.min(candidate)));
                    } else {
                        used[v] = true;
                        queue.push_back(v);
                        depth[v] = d;
                        inp[v] = Some(current);
                    }
                }
            }

            if best == Some(3) {
                return best;
            }
        }
    }

    best
}

/// Weighted eccentricity.
///
/// Distance to the farthest node from `node`, given edge weights.
/// Returns `None` if the graph contains a negative cycle.
///
/// # Examples
///
/// ```
/// use petgraph_live::metrics::weighted_eccentricity;
/// use petgraph::Graph;
///
/// let graph = Graph::<(), f32>::from_edges(&[
///     (0, 1, 2.0), (1, 2, 10.0), (1, 3, -5.0),
///     (3, 2, 2.0), (2, 3, 20.0),
/// ]);
///
/// assert_eq!(weighted_eccentricity(&graph, 0.into(), |e| *e.weight()), Some(2.0));
/// assert_eq!(weighted_eccentricity(&graph, 1.into(), |e| *e.weight()), Some(f32::INFINITY));
/// ```
pub fn weighted_eccentricity<G, F, K>(graph: G, node: G::NodeId, edge_cost: F) -> Option<K>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + NodeCount + GraphProp,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    let idx = graph.to_index(node);
    match floyd_warshall(graph, edge_cost) {
        Err(_) => None,
        Ok(dist) => dist[idx]
            .iter()
            .copied()
            .max_by(|x, y| x.partial_cmp(y).unwrap()),
    }
}

/// Weighted graph radius.
///
/// # Examples
///
/// ```
/// use petgraph_live::metrics::weighted_radius;
/// use petgraph::Graph;
///
/// let graph = Graph::<(), f32>::from_edges(&[
///     (0, 1, 2.0), (1, 2, 10.0), (1, 3, -5.0),
///     (3, 2, 2.0), (2, 3, 20.0),
/// ]);
///
/// assert_eq!(weighted_radius(&graph, |edge| *edge.weight()), Some(2.0));
/// ```
pub fn weighted_radius<G, F, K>(graph: G, edge_cost: F) -> Option<K>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + NodeCount + GraphProp,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    if graph.node_count() == 0 {
        return None;
    }
    match floyd_warshall(graph, edge_cost) {
        Err(_) => None,
        Ok(dist) => dist
            .iter()
            .map(|row| *row.iter().max_by(|x, y| x.partial_cmp(y).unwrap()).unwrap())
            .min_by(|x, y| x.partial_cmp(y).unwrap()),
    }
}

/// Weighted graph diameter.
///
/// # Examples
///
/// ```
/// use petgraph_live::metrics::weighted_diameter;
/// use petgraph::Graph;
///
/// let graph = Graph::<(), f32>::from_edges(&[
///     (0, 1, 2.0), (1, 2, 10.0), (1, 3, -5.0),
///     (3, 2, 2.0), (2, 3, 20.0),
/// ]);
///
/// assert_eq!(weighted_diameter(&graph, |edge| *edge.weight()), Some(f32::INFINITY));
/// ```
pub fn weighted_diameter<G, F, K>(graph: G, edge_cost: F) -> Option<K>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + NodeCount + GraphProp,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    if graph.node_count() == 0 {
        return None;
    }
    match floyd_warshall(graph, edge_cost) {
        Err(_) => None,
        Ok(dist) => {
            let mut diam = K::zero();
            for row in &dist {
                for &d in row {
                    if d == K::infinite() {
                        return Some(d);
                    } else if d > diam {
                        diam = d;
                    }
                }
            }
            Some(diam)
        }
    }
}

/// Center of a weighted graph.
///
/// # Examples
///
/// ```
/// use petgraph_live::metrics::weighted_center;
/// use petgraph::Graph;
///
/// let graph = Graph::<(), f32>::from_edges(&[
///     (0, 1, 2.0), (1, 2, 10.0), (1, 3, -5.0),
///     (3, 2, 2.0), (2, 3, 20.0), (3, 0, 3.0),
/// ]);
///
/// assert_eq!(weighted_center(&graph, |edge| *edge.weight()), vec![1.into()]);
/// ```
pub fn weighted_center<G, F, K>(graph: G, edge_cost: F) -> Vec<G::NodeId>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + NodeCount + GraphProp,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    if graph.node_count() == 0 {
        return vec![];
    }
    match floyd_warshall(graph, edge_cost) {
        Err(_) => vec![],
        Ok(dist) => {
            let ecc: Vec<K> = dist
                .iter()
                .map(|row| *row.iter().max_by(|x, y| x.partial_cmp(y).unwrap()).unwrap())
                .collect();
            let rad = *ecc.iter().min_by(|x, y| x.partial_cmp(y).unwrap()).unwrap();
            (0..graph.node_bound())
                .filter(|i| ecc[*i] == rad)
                .map(|i| graph.from_index(i))
                .collect()
        }
    }
}

/// Peripheral vertices of a weighted graph.
///
/// # Examples
///
/// ```
/// use petgraph_live::metrics::weighted_periphery;
/// use petgraph::Graph;
///
/// let graph = Graph::<(), f32>::from_edges(&[
///     (0, 1, 2.0), (1, 2, 10.0), (1, 3, -5.0),
///     (3, 2, 2.0), (2, 3, 20.0), (3, 0, 3.0),
/// ]);
///
/// assert_eq!(weighted_periphery(&graph, |edge| *edge.weight()), vec![2.into()]);
/// ```
pub fn weighted_periphery<G, F, K>(graph: G, edge_cost: F) -> Vec<G::NodeId>
where
    G: IntoEdgeReferences + IntoNodeIdentifiers + NodeIndexable + NodeCount + GraphProp,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    if graph.node_count() == 0 {
        return vec![];
    }
    match floyd_warshall(graph, edge_cost) {
        Err(_) => vec![],
        Ok(dist) => {
            let ecc: Vec<K> = dist
                .iter()
                .map(|row| *row.iter().max_by(|x, y| x.partial_cmp(y).unwrap()).unwrap())
                .collect();
            let diam = *ecc.iter().max_by(|x, y| x.partial_cmp(y).unwrap()).unwrap();
            (0..graph.node_bound())
                .filter(|i| ecc[*i] == diam)
                .map(|i| graph.from_index(i))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::{Graph, UnGraph};

    fn graph1() -> Graph<(), ()> {
        let mut graph = Graph::<(), ()>::new();
        let n0 = graph.add_node(());
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());
        let n4 = graph.add_node(());
        let n5 = graph.add_node(());
        let n6 = graph.add_node(());
        let n7 = graph.add_node(());
        let n8 = graph.add_node(());
        let n9 = graph.add_node(());
        let n10 = graph.add_node(());
        let n11 = graph.add_node(());

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

    fn graph2() -> Graph<(), ()> {
        let mut graph = Graph::<(), ()>::new();
        let n0 = graph.add_node(());
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());
        let n4 = graph.add_node(());
        let n5 = graph.add_node(());
        let n6 = graph.add_node(());

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

        graph
    }

    fn graph3() -> Graph<(), f32> {
        let mut graph = Graph::<(), f32>::new();
        graph.add_node(());
        graph
    }

    fn graph4() -> Graph<(), f32> {
        Graph::<(), f32>::new()
    }

    fn graph5() -> Graph<(), f32> {
        let mut graph = Graph::new();
        let n0 = graph.add_node(());
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());
        let n4 = graph.add_node(());
        let n5 = graph.add_node(());

        graph.add_edge(n1, n0, 10.0);
        graph.add_edge(n1, n0, 10.0);
        graph.add_edge(n0, n3, 14.0);
        graph.add_edge(n3, n0, 14.0);
        graph.add_edge(n1, n2, 5.0);
        graph.add_edge(n2, n1, -5.0);
        graph.add_edge(n2, n3, 1.0);
        graph.add_edge(n3, n2, 1.0);
        graph.add_edge(n2, n4, 3.0);
        graph.add_edge(n4, n2, 3.0);
        graph.add_edge(n3, n5, -1.0);

        graph
    }

    fn graph6() -> Graph<(), f32> {
        let mut graph = Graph::new();
        graph.add_node(());
        graph.add_node(());
        graph
    }

    fn graph7() -> UnGraph<(), ()> {
        let mut graph = UnGraph::<(), ()>::new_undirected();
        let n0 = graph.add_node(());
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());
        let n4 = graph.add_node(());
        let n5 = graph.add_node(());
        let n6 = graph.add_node(());

        graph.add_edge(n0, n6, ());
        graph.add_edge(n0, n1, ());
        graph.add_edge(n1, n2, ());
        graph.add_edge(n1, n5, ());
        graph.add_edge(n2, n3, ());
        graph.add_edge(n3, n4, ());
        graph.add_edge(n4, n5, ());
        graph.add_edge(n5, n2, ());
        graph.add_edge(n6, n1, ());
        graph.add_edge(n6, n5, ());

        graph
    }

    #[test]
    fn test_eccentricity() {
        let inf = f32::INFINITY;

        let g = graph1();
        assert_eq!(eccentricity(&g, 0.into()), 5.0);
        for i in 1..12 {
            assert_eq!(eccentricity(&g, i.into()), inf);
        }

        let g = graph2();
        assert_eq!(eccentricity(&g, 0.into()), 3.0);
        assert_eq!(eccentricity(&g, 1.into()), 2.0);
        assert_eq!(eccentricity(&g, 2.into()), 2.0);
        assert_eq!(eccentricity(&g, 3.into()), 3.0);
        assert_eq!(eccentricity(&g, 4.into()), 3.0);
        assert_eq!(eccentricity(&g, 5.into()), 2.0);
        assert_eq!(eccentricity(&g, 6.into()), 3.0);

        let g = graph3();
        assert_eq!(eccentricity(&g, 0.into()), 0.0);
    }

    #[test]
    fn test_radius() {
        let inf = f32::INFINITY;

        assert_eq!(radius(&graph1()), Some(5.0));
        assert_eq!(radius(&graph2()), Some(2.0));
        assert_eq!(radius(&graph3()), Some(0.0));
        assert_eq!(radius(&graph4()), None);
        assert_eq!(radius(&graph5()), Some(2.0));
        assert_eq!(radius(&graph6()), Some(inf));
    }

    #[test]
    fn test_diameter() {
        let inf = f32::INFINITY;

        assert_eq!(diameter(&graph1()), Some(inf));
        assert_eq!(diameter(&graph2()), Some(3.0));
        assert_eq!(diameter(&graph3()), Some(0.0));
        assert_eq!(diameter(&graph4()), None);
        assert_eq!(diameter(&graph5()), Some(inf));
        assert_eq!(diameter(&graph6()), Some(inf));
    }

    #[test]
    fn test_center() {
        assert_eq!(center(&graph1()), vec![0.into()]);
        assert_eq!(center(&graph2()), vec![1.into(), 2.into(), 5.into()]);
        assert_eq!(center(&graph3()), vec![0.into()]);
        assert_eq!(center(&graph4()), vec![]);
        assert_eq!(center(&graph5()), vec![2.into(), 3.into()]);
        assert_eq!(center(&graph6()), vec![0.into(), 1.into()]);
    }

    #[test]
    fn test_periphery() {
        assert_eq!(
            periphery(&graph1()),
            vec![
                1.into(),
                2.into(),
                3.into(),
                4.into(),
                5.into(),
                6.into(),
                7.into(),
                8.into(),
                9.into(),
                10.into(),
                11.into()
            ]
        );
        assert_eq!(
            periphery(&graph2()),
            vec![0.into(), 3.into(), 4.into(), 6.into()]
        );
        assert_eq!(periphery(&graph3()), vec![0.into()]);
        assert_eq!(periphery(&graph4()), vec![]);
        assert_eq!(periphery(&graph5()), vec![5.into()]);
        assert_eq!(periphery(&graph6()), vec![0.into(), 1.into()]);
    }

    #[test]
    fn test_weighted_eccentricity() {
        let inf = f32::INFINITY;

        let g = graph3();
        assert_eq!(
            weighted_eccentricity(&g, 0.into(), |e| *e.weight()),
            Some(0.0)
        );

        let graph = graph5();
        assert_eq!(
            weighted_eccentricity(&graph, 0.into(), |e| *e.weight()),
            Some(18.0)
        );
        assert_eq!(
            weighted_eccentricity(&graph, 1.into(), |e| *e.weight()),
            Some(10.0)
        );
        assert_eq!(
            weighted_eccentricity(&graph, 2.into(), |e| *e.weight()),
            Some(5.0)
        );
        assert_eq!(
            weighted_eccentricity(&graph, 3.into(), |e| *e.weight()),
            Some(6.0)
        );
        assert_eq!(
            weighted_eccentricity(&graph, 4.into(), |e| *e.weight()),
            Some(8.0)
        );
        assert_eq!(
            weighted_eccentricity(&graph, 5.into(), |e| *e.weight()),
            Some(inf)
        );
    }

    #[test]
    fn test_weighted_radius() {
        let inf = f32::INFINITY;

        assert_eq!(weighted_radius(&graph1(), |_| 1.0f32), Some(5.0));
        assert_eq!(weighted_radius(&graph2(), |_| 2.0f32), Some(4.0));
        assert_eq!(weighted_radius(&graph3(), |edge| *edge.weight()), Some(0.0));
        assert_eq!(weighted_radius(&graph4(), |edge| *edge.weight()), None);
        assert_eq!(weighted_radius(&graph5(), |edge| *edge.weight()), Some(5.0));
        assert_eq!(weighted_radius(&graph6(), |edge| *edge.weight()), Some(inf));
    }

    #[test]
    fn test_weighted_diameter() {
        let inf = f32::INFINITY;

        assert_eq!(weighted_diameter(&graph1(), |_| 1.0f32), Some(inf));
        assert_eq!(weighted_diameter(&graph2(), |_| 2.0f32), Some(6.0));
        assert_eq!(
            weighted_diameter(&graph3(), |edge| *edge.weight()),
            Some(0.0)
        );
        assert_eq!(weighted_diameter(&graph4(), |edge| *edge.weight()), None);
        assert_eq!(
            weighted_diameter(&graph5(), |edge| *edge.weight()),
            Some(inf)
        );
        assert_eq!(
            weighted_diameter(&graph6(), |edge| *edge.weight()),
            Some(inf)
        );
    }

    #[test]
    fn test_weighted_center() {
        assert_eq!(weighted_center(&graph1(), |_| 1.0f32), vec![0.into()]);
        assert_eq!(
            weighted_center(&graph2(), |_| 2.0f32),
            vec![1.into(), 2.into(), 5.into()]
        );
        assert_eq!(
            weighted_center(&graph3(), |edge| *edge.weight()),
            vec![0.into()]
        );
        assert_eq!(weighted_center(&graph4(), |edge| *edge.weight()), vec![]);
        assert_eq!(
            weighted_center(&graph5(), |edge| *edge.weight()),
            vec![2.into()]
        );
        assert_eq!(
            weighted_center(&graph6(), |edge| *edge.weight()),
            vec![0.into(), 1.into()]
        );
    }

    #[test]
    fn test_weighted_periphery() {
        assert_eq!(
            weighted_periphery(&graph1(), |_| 1.0f32),
            vec![
                1.into(),
                2.into(),
                3.into(),
                4.into(),
                5.into(),
                6.into(),
                7.into(),
                8.into(),
                9.into(),
                10.into(),
                11.into()
            ]
        );
        assert_eq!(
            weighted_periphery(&graph2(), |_| 2.0f32),
            vec![0.into(), 3.into(), 4.into(), 6.into()]
        );
        assert_eq!(
            weighted_periphery(&graph3(), |edge| *edge.weight()),
            vec![0.into()]
        );
        assert_eq!(weighted_periphery(&graph4(), |edge| *edge.weight()), vec![]);
        assert_eq!(
            weighted_periphery(&graph5(), |edge| *edge.weight()),
            vec![5.into()]
        );
        assert_eq!(
            weighted_periphery(&graph6(), |edge| *edge.weight()),
            vec![0.into(), 1.into()]
        );
    }

    #[test]
    fn test_girth() {
        assert_eq!(girth(&Graph::<(), ()>::new()), None);
        assert_eq!(girth(&UnGraph::<(), ()>::new_undirected()), None);
        assert_eq!(girth(&graph1()), Some(2));
        assert_eq!(girth(&graph5()), Some(2));
        assert_eq!(girth(&graph7()), Some(3));

        let mut g = Graph::<i32, ()>::new();
        let n0 = g.add_node(0);
        assert_eq!(girth(&g), None);
        let n1 = g.add_node(1);
        assert_eq!(girth(&g), None);
        g.add_edge(n0, n1, ());
        assert_eq!(girth(&g), None);
        g.add_edge(n0, n1, ());
        assert_eq!(girth(&g), None);
        g.add_edge(n1, n0, ());
        assert_eq!(girth(&g), Some(2));

        let mut g = UnGraph::<i32, ()>::new_undirected();
        let n0 = g.add_node(0);
        assert_eq!(girth(&g), None);
        let n1 = g.add_node(1);
        assert_eq!(girth(&g), None);
        g.add_edge(n0, n1, ());
        assert_eq!(girth(&g), None);
        let n2 = g.add_node(2);
        g.add_edge(n0, n2, ());
        assert_eq!(girth(&g), None);
        g.add_edge(n2, n1, ());
        assert_eq!(girth(&g), Some(3));

        let mut g = Graph::<i32, ()>::new();
        let n0 = g.add_node(0);
        g.add_edge(n0, n0, ());
        assert_eq!(girth(&g), None);
    }
}
