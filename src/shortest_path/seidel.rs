use petgraph::visit::{
    EdgeRef, GraphProp, IntoEdges, IntoNodeIdentifiers, NodeCount, NodeIndexable,
};

/// [Seidel's algorithm (APD)](https://en.wikipedia.org/wiki/Seidel%27s_algorithm)
/// for all pairs shortest path problem.
///
/// Computes the distance matrix of an **unweighted**, **undirected**, **connected** graph.
/// Distances are in units of edge hops. Diagonal is always 0.
///
/// # Examples
///
/// ```
/// use petgraph_live::shortest_path::seidel;
/// use petgraph::{Graph, Undirected};
///
/// let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
/// let n0 = graph.add_node(());
/// let n1 = graph.add_node(());
/// let n2 = graph.add_node(());
/// let n3 = graph.add_node(());
/// let n4 = graph.add_node(());
/// let n5 = graph.add_node(());
/// graph.extend_with_edges(&[(0, 1), (0, 3), (1, 2), (1, 5), (2, 4), (3, 4), (4, 5)]);
///
/// assert_eq!(
///     seidel(&graph),
///     vec![vec![0, 1, 2, 1, 2, 2],
///          vec![1, 0, 1, 2, 2, 1],
///          vec![2, 1, 0, 2, 1, 2],
///          vec![1, 2, 2, 0, 1, 2],
///          vec![2, 2, 1, 1, 0, 1],
///          vec![2, 1, 2, 2, 1, 0]]
/// );
/// ```
pub fn seidel<G>(graph: G) -> Vec<Vec<u32>>
where
    G: IntoEdges + IntoNodeIdentifiers + NodeCount + NodeIndexable + GraphProp,
{
    let n = graph.node_count();
    if n == 0 {
        return vec![];
    }

    // Build symmetric adjacency matrix
    let mut a = vec![vec![0u32; n]; n];
    for node in graph.node_identifiers() {
        let i = graph.to_index(node);
        for edge in graph.edges(node) {
            let j = graph.to_index(edge.target());
            if i != j {
                a[i][j] = 1;
                a[j][i] = 1;
            }
        }
    }

    let mut d = apd(&a);
    #[allow(clippy::needless_range_loop)]
    for i in 0..n {
        d[i][i] = 0;
    }
    d
}

/// APD recursive helper for Seidel's algorithm.
///
/// # Safety (by convention)
/// Caller must ensure `a` is a symmetric (undirected) adjacency matrix with 0 on diagonal.
/// Violating this may cause infinite recursion.
#[allow(dead_code)]
pub(crate) fn apd(a: &[Vec<u32>]) -> Vec<Vec<u32>> {
    let n = a.len();
    if n == 0 {
        return vec![];
    }

    // Base case: all off-diagonal entries nonzero → distances are exactly A
    if (0..n).all(|i| (0..n).all(|j| i == j || a[i][j] != 0)) {
        return a.to_vec();
    }

    // Z = A * A (matrix square)
    let mut z = vec![vec![0u32; n]; n];
    #[allow(clippy::needless_range_loop)]
    for i in 0..n {
        for k in 0..n {
            if a[i][k] == 0 {
                continue;
            }
            for j in 0..n {
                z[i][j] += a[i][k] * a[k][j];
            }
        }
    }

    // B[i][j] = 1 if i!=j && (A[i][j]==1 || Z[i][j]>0), else 0
    let mut b = vec![vec![0u32; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i != j && (a[i][j] == 1 || z[i][j] > 0) {
                b[i][j] = 1;
            }
        }
    }

    let t = apd(&b);

    // X = T * A
    let mut x = vec![vec![0u32; n]; n];
    for i in 0..n {
        for k in 0..n {
            if t[i][k] == 0 {
                continue;
            }
            for j in 0..n {
                x[i][j] += t[i][k] * a[k][j];
            }
        }
    }

    // degree[j] = sum_i A[i][j] (column sum = row sum since symmetric)
    let degree: Vec<u32> = (0..n).map(|j| (0..n).map(|i| a[i][j]).sum()).collect();

    // D[i][j] = 2*T[i][j] if X[i][j] >= T[i][j]*degree[j], else 2*T[i][j]-1
    let mut d = vec![vec![0u32; n]; n];
    for i in 0..n {
        for j in 0..n {
            if x[i][j] >= t[i][j] * degree[j] {
                d[i][j] = 2 * t[i][j];
            } else {
                d[i][j] = 2 * t[i][j] - 1;
            }
        }
    }

    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::Graph;

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

    fn graph2() -> Graph<(), ()> {
        let mut graph = Graph::<(), ()>::new();
        let n0 = graph.add_node(());
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());
        graph.add_edge(n0, n1, ());
        graph.add_edge(n1, n0, ());
        graph.add_edge(n1, n2, ());
        graph.add_edge(n2, n1, ());
        graph.add_edge(n1, n3, ());
        graph.add_edge(n3, n1, ());
        graph.add_edge(n2, n3, ());
        graph.add_edge(n3, n2, ());

        graph
    }

    fn graph3() -> Graph<(), f64> {
        let mut graph = Graph::<(), f64>::new();
        let n0 = graph.add_node(());
        let n1 = graph.add_node(());
        graph.add_edge(n0, n1, 10.0);
        graph.add_edge(n1, n0, 5.0);
        graph
    }

    #[test]
    fn test_apd() {
        // graph1 adjacency (symmetric, treating directed as undirected):
        // n0-n1, n0-n4, n1-n2, n1-n3, n2-n3, n3-n4
        let a = vec![
            vec![0, 1, 0, 0, 1],
            vec![1, 0, 1, 1, 1],
            vec![0, 1, 0, 1, 0],
            vec![0, 1, 1, 0, 1],
            vec![1, 1, 0, 1, 0],
        ];
        assert_eq!(
            apd(&a),
            vec![
                vec![0, 1, 2, 2, 1],
                vec![1, 0, 1, 1, 1],
                vec![2, 1, 0, 1, 2],
                vec![2, 1, 1, 0, 1],
                vec![1, 1, 2, 1, 0]
            ]
        );

        // graph2 adjacency (bidirectional edges → symmetric)
        let b = vec![
            vec![0, 1, 0, 0],
            vec![1, 0, 1, 1],
            vec![0, 1, 0, 1],
            vec![0, 1, 1, 0],
        ];
        assert_eq!(
            apd(&b),
            vec![
                vec![0, 1, 2, 2],
                vec![1, 0, 1, 1],
                vec![2, 1, 0, 1],
                vec![2, 1, 1, 0],
            ]
        );
    }

    #[test]
    fn test_apd_empty_graph() {
        assert_eq!(apd(&[]), Vec::<Vec<u32>>::new());
    }

    #[test]
    fn test_apd_single_edge() {
        let a = vec![vec![0, 1], vec![1, 0]];
        assert_eq!(apd(&a), vec![vec![0, 1], vec![1, 0]]);
    }

    #[test]
    fn test_seidel() {
        assert_eq!(
            seidel(&graph1()),
            vec![
                vec![0, 1, 2, 2, 1],
                vec![1, 0, 1, 1, 1],
                vec![2, 1, 0, 1, 2],
                vec![2, 1, 1, 0, 1],
                vec![1, 1, 2, 1, 0]
            ]
        );

        assert_eq!(
            seidel(&graph2()),
            vec![
                vec![0, 1, 2, 2],
                vec![1, 0, 1, 1],
                vec![2, 1, 0, 1],
                vec![2, 1, 1, 0],
            ]
        );
    }

    #[test]
    fn test_seidel_single_edge() {
        assert_eq!(seidel(&graph3()), vec![vec![0, 1], vec![1, 0]]);
    }

    #[test]
    fn test_seidel_empty_graph() {
        assert_eq!(seidel(&Graph::<(), f32>::new()), Vec::<Vec<u32>>::new());
    }
}
