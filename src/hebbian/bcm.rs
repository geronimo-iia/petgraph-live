//! BCM (Bienenstock-Cooper-Munro) rule — homeostatic plasticity.
//!
//! Activity-dependent threshold: highly active nodes become harder to strengthen.
//! Prevents runaway strengthening and stabilizes the network.

use petgraph::EdgeType;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph;

/// BCM configuration.
#[derive(Debug, Clone, Copy)]
pub struct BcmConfig {
    /// Learning rate (default: 0.01).
    pub learning_rate: f64,
    /// Threshold sliding rate (default: 0.001).
    pub threshold_rate: f64,
}

impl Default for BcmConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            threshold_rate: 0.001,
        }
    }
}

/// BCM state — tracks per-node sliding threshold.
#[derive(Debug, Clone)]
pub struct BcmState {
    /// Per-node activity threshold (indexed by `NodeIndexable::to_index`).
    pub thresholds: Vec<f64>,
}

impl BcmState {
    /// Create state for a graph with `n` nodes, initialized to `initial_threshold`.
    pub fn new(n: usize, initial_threshold: f64) -> Self {
        Self {
            thresholds: vec![initial_threshold; n],
        }
    }
}

/// Apply BCM rule: strengthen if activation > threshold, weaken if below.
///
/// For each edge (pre → post) where both are in `activated`:
///   Δw = η × y_post × (y_post − θ_post) × x_pre
///
/// Thresholds slide toward recent activity:
///   Δθ = threshold_rate × (y² − θ)
///
/// Returns number of edges modified.
///
/// # Examples
///
/// ```
/// use petgraph::stable_graph::StableDiGraph;
/// use petgraph_live::hebbian::{bcm_update, BcmConfig, BcmState};
///
/// let mut g = StableDiGraph::<(), f64>::new();
/// let a = g.add_node(());
/// let b = g.add_node(());
/// g.add_edge(a, b, 0.5);
///
/// let mut state = BcmState::new(2, 0.5);
/// let modified = bcm_update(&mut g, &[(a, 0.8), (b, 0.9)], &mut state, &BcmConfig::default());
/// assert_eq!(modified, 1);
/// ```
pub fn bcm_update<N, Ty: EdgeType>(
    graph: &mut StableGraph<N, f64, Ty>,
    activated: &[(NodeIndex, f64)],
    state: &mut BcmState,
    config: &BcmConfig,
) -> usize {
    // Update thresholds for all activated nodes
    for &(node, y) in activated {
        let idx = node.index();
        if idx < state.thresholds.len() {
            let theta = &mut state.thresholds[idx];
            *theta += config.threshold_rate * (y * y - *theta);
        }
    }

    // Modify edges between activated pairs
    let mut modified = 0;
    for i in 0..activated.len() {
        let j_start = if Ty::is_directed() { 0 } else { i + 1 };
        for j in j_start..activated.len() {
            if i == j {
                continue;
            }
            let (pre_node, x) = activated[i];
            let (post_node, y) = activated[j];
            let post_idx = post_node.index();
            if post_idx >= state.thresholds.len() {
                continue;
            }
            let theta = state.thresholds[post_idx];
            // Δw = η × y × (y − θ) × x
            let delta_w = config.learning_rate * y * (y - theta) * x;

            if let Some(edge_idx) = graph.find_edge(pre_node, post_node) {
                if let Some(w) = graph.edge_weight_mut(edge_idx) {
                    *w += delta_w;
                    modified += 1;
                }
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
    fn above_threshold_strengthens() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);

        // threshold=0.3, activation=0.9 → above threshold → strengthen
        let mut state = BcmState::new(2, 0.3);
        let config = BcmConfig::default();
        bcm_update(&mut g, &[(a, 0.9), (b, 0.9)], &mut state, &config);
        assert!(*g.edge_weight(0.into()).unwrap() > 0.5);
    }

    #[test]
    fn below_threshold_weakens() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);

        // threshold=0.9, activation=0.3 → below threshold → weaken
        let mut state = BcmState::new(2, 0.9);
        let config = BcmConfig::default();
        bcm_update(&mut g, &[(a, 0.3), (b, 0.3)], &mut state, &config);
        assert!(*g.edge_weight(0.into()).unwrap() < 0.5);
    }

    #[test]
    fn threshold_slides_toward_activity() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let _b = g.add_node(());

        let mut state = BcmState::new(2, 0.5);
        let config = BcmConfig {
            threshold_rate: 0.1,
            ..BcmConfig::default()
        };

        // Activate a with y=1.0 → threshold moves toward y²=1.0
        bcm_update(&mut g, &[(a, 1.0)], &mut state, &config);
        // θ += 0.1 * (1.0 - 0.5) = 0.55
        assert!((state.thresholds[0] - 0.55).abs() < 1e-10);
    }

    #[test]
    fn no_edge_no_modification() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());

        let mut state = BcmState::new(2, 0.5);
        let modified = bcm_update(&mut g, &[(a, 0.8), (b, 0.8)], &mut state, &BcmConfig::default());
        assert_eq!(modified, 0);
    }

    #[test]
    fn repeated_high_activity_raises_threshold() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);

        let mut state = BcmState::new(2, 0.1);
        let config = BcmConfig {
            threshold_rate: 0.5,
            learning_rate: 0.01,
            ..BcmConfig::default()
        };

        // Repeated high activation raises threshold → eventually weakens
        for _ in 0..20 {
            bcm_update(&mut g, &[(a, 1.0), (b, 1.0)], &mut state, &config);
        }
        // Threshold for b should be near 1.0 (y²=1.0)
        assert!(state.thresholds[1] > 0.9);
    }
}
