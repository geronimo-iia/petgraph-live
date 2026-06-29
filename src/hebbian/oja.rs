//! Oja's rule — normalized Hebbian learning.
//!
//! Self-normalizing: weights converge without manual capping.
//! Δw_ij = η × y_i × (x_j − w_ij × y_i)

use petgraph::EdgeType;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph;

/// Oja's rule configuration.
#[derive(Debug, Clone, Copy)]
pub struct OjaConfig {
    /// Learning rate (default: 0.01).
    pub learning_rate: f64,
}

impl Default for OjaConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
        }
    }
}

/// Apply Oja's normalized Hebbian rule.
///
/// For each edge (pre → post) where pre is in `pre_activations` and post is in
/// `post_activations`, updates weight:
///   Δw = η × y_post × (x_pre − w × y_post)
///
/// Self-normalizing — weights converge to principal component direction.
/// Returns number of edges modified.
///
/// # Examples
///
/// ```
/// use petgraph::stable_graph::StableDiGraph;
/// use petgraph_live::hebbian::{oja_update, OjaConfig};
///
/// let mut g = StableDiGraph::<(), f64>::new();
/// let a = g.add_node(());
/// let b = g.add_node(());
/// g.add_edge(a, b, 0.5);
///
/// let pre = vec![(a, 1.0)];
/// let post = vec![(b, 0.8)];
/// let modified = oja_update(&mut g, &pre, &post, &OjaConfig::default());
/// assert_eq!(modified, 1);
/// ```
pub fn oja_update<N, Ty: EdgeType>(
    graph: &mut StableGraph<N, f64, Ty>,
    pre_activations: &[(NodeIndex, f64)],
    post_activations: &[(NodeIndex, f64)],
    config: &OjaConfig,
) -> usize {
    let mut modified = 0;
    for &(post_node, y) in post_activations {
        for &(pre_node, x) in pre_activations {
            if pre_node == post_node {
                continue;
            }
            if let Some(idx) = graph.find_edge(pre_node, post_node)
                && let Some(w) = graph.edge_weight_mut(idx)
            {
                // Δw = η × y × (x − w × y)
                *w += config.learning_rate * y * (x - *w * y);
                modified += 1;
            }
        }
    }
    modified
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::stable_graph::StableDiGraph;

    #[test]
    fn oja_converges_toward_normalization() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);

        let config = OjaConfig { learning_rate: 0.1 };
        // Repeated application with constant activations converges
        for _ in 0..100 {
            oja_update(&mut g, &[(a, 1.0)], &[(b, 1.0)], &config);
        }
        // With x=1, y=1: converges to w where η*y*(x - w*y) = 0 → w = x/y = 1.0
        let w = *g.edge_weight(0.into()).unwrap();
        assert!((w - 1.0).abs() < 0.01);
    }

    #[test]
    fn oja_no_edge_no_effect() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());

        let modified = oja_update(&mut g, &[(a, 1.0)], &[(b, 1.0)], &OjaConfig::default());
        assert_eq!(modified, 0);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn oja_self_loop_skipped() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        g.add_edge(a, a, 0.5);

        let modified = oja_update(&mut g, &[(a, 1.0)], &[(a, 1.0)], &OjaConfig::default());
        assert_eq!(modified, 0);
        assert_eq!(*g.edge_weight(0.into()).unwrap(), 0.5);
    }

    #[test]
    fn oja_high_weight_decreases() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 2.0); // w > x/y → should decrease

        let config = OjaConfig { learning_rate: 0.1 };
        oja_update(&mut g, &[(a, 1.0)], &[(b, 1.0)], &config);
        // Δw = 0.1 * 1.0 * (1.0 - 2.0*1.0) = -0.1
        assert!(*g.edge_weight(0.into()).unwrap() < 2.0);
    }

    #[test]
    fn oja_low_weight_increases() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.1); // w < x/y → should increase

        let config = OjaConfig { learning_rate: 0.1 };
        oja_update(&mut g, &[(a, 1.0)], &[(b, 1.0)], &config);
        assert!(*g.edge_weight(0.into()).unwrap() > 0.1);
    }
}
