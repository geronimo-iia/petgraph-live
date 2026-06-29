//! Anti-Hebbian learning — weaken edges between co-activated nodes.
//!
//! Opposite of SOKM strengthen: forces specialization by weakening connections
//! between nodes that fire together (lateral inhibition).

use petgraph::EdgeType;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph;

/// Anti-Hebbian configuration.
#[derive(Debug, Clone, Copy)]
pub struct AntiHebbianConfig {
    /// Weakening rate (default: 0.005).
    pub beta: f64,
}

impl Default for AntiHebbianConfig {
    fn default() -> Self {
        Self { beta: 0.005 }
    }
}

/// Weaken edges between co-activated nodes (lateral inhibition).
///
/// For each pair (a, b) in `activated` that has an existing edge,
/// decrements edge weight by `beta * sa * sb`.
///
/// Does **not** create new edges. Does **not** remove edges (use `prune` after).
/// Returns number of edges weakened.
///
/// # Examples
///
/// ```
/// use petgraph::stable_graph::StableDiGraph;
/// use petgraph_live::hebbian::{anti_hebbian_update, AntiHebbianConfig};
///
/// let mut g = StableDiGraph::<(), f64>::new();
/// let a = g.add_node(());
/// let b = g.add_node(());
/// g.add_edge(a, b, 0.5);
/// g.add_edge(b, a, 0.3);
///
/// let weakened = anti_hebbian_update(&mut g, &[(a, 1.0), (b, 0.8)], &AntiHebbianConfig::default());
/// assert_eq!(weakened, 2);
/// assert!(*g.edge_weight(0.into()).unwrap() < 0.5);
/// ```
pub fn anti_hebbian_update<N, Ty: EdgeType>(
    graph: &mut StableGraph<N, f64, Ty>,
    activated: &[(NodeIndex, f64)],
    config: &AntiHebbianConfig,
) -> usize {
    let mut count = 0;
    for i in 0..activated.len() {
        let j_start = if Ty::is_directed() { 0 } else { i + 1 };
        for j in j_start..activated.len() {
            if i == j {
                continue;
            }
            let (na, sa) = activated[i];
            let (nb, sb) = activated[j];
            let decrement = config.beta * sa * sb;

            if let Some(idx) = graph.find_edge(na, nb) {
                if let Some(w) = graph.edge_weight_mut(idx) {
                    *w -= decrement;
                    count += 1;
                }
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::stable_graph::{StableDiGraph, StableUnGraph};

    #[test]
    fn co_activated_pair_weakened() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);
        g.add_edge(b, a, 0.3);

        let config = AntiHebbianConfig::default();
        anti_hebbian_update(&mut g, &[(a, 1.0), (b, 1.0)], &config);

        // 0.5 - 0.005*1.0*1.0 = 0.495
        assert!((g.edge_weight(0.into()).unwrap() - 0.495).abs() < 1e-10);
        // 0.3 - 0.005 = 0.295
        assert!((g.edge_weight(1.into()).unwrap() - 0.295).abs() < 1e-10);
    }

    #[test]
    fn non_activated_pairs_unchanged() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        let c = g.add_node(());
        g.add_edge(a, b, 0.5);
        g.add_edge(b, c, 0.3);

        let config = AntiHebbianConfig::default();
        // only a activated — no pairs
        let count = anti_hebbian_update(&mut g, &[(a, 1.0)], &config);
        assert_eq!(count, 0);
        assert_eq!(*g.edge_weight(0.into()).unwrap(), 0.5);
        assert_eq!(*g.edge_weight(1.into()).unwrap(), 0.3);
    }

    #[test]
    fn no_edge_no_effect() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        // no edges

        let config = AntiHebbianConfig::default();
        let count = anti_hebbian_update(&mut g, &[(a, 1.0), (b, 1.0)], &config);
        assert_eq!(count, 0);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn undirected_anti_hebbian() {
        let mut g = StableUnGraph::<(), f64>::with_capacity(0, 0);
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);

        let config = AntiHebbianConfig { beta: 0.01 };
        let count = anti_hebbian_update(&mut g, &[(a, 1.0), (b, 0.8)], &config);
        assert_eq!(count, 1);
        // 0.5 - 0.01*1.0*0.8 = 0.492
        assert!((g.edge_weight(0.into()).unwrap() - 0.492).abs() < 1e-10);
    }
}
