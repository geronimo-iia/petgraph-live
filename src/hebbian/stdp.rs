//! STDP (Spike-Timing Dependent Plasticity) — temporal Hebbian rule.
//!
//! Strengthening depends on *order* of activation: causal pairs (pre fires
//! before post) are strengthened, anti-causal pairs are weakened.

use petgraph::EdgeType;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph;

/// STDP configuration.
#[derive(Debug, Clone, Copy)]
pub struct StdpConfig {
    /// Strengthening amplitude for causal order (pre fires before post).
    pub a_plus: f64,
    /// Weakening amplitude for anti-causal order (post fires before pre).
    pub a_minus: f64,
    /// Time constant for causal window (ticks).
    pub tau_plus: f64,
    /// Time constant for anti-causal window (ticks).
    pub tau_minus: f64,
}

impl Default for StdpConfig {
    fn default() -> Self {
        Self {
            a_plus: 0.01,
            a_minus: 0.005,
            tau_plus: 5.0,
            tau_minus: 5.0,
        }
    }
}

/// Activation with timing: (node, score, tick_fired).
pub type TimedActivation = (NodeIndex, f64, u64);

/// Apply STDP rule to edges between nodes that fired in the given window.
///
/// For each directed edge (pre → post) where both endpoints fired:
/// - If pre fired before post (causal): Δw = +a_plus × exp(-Δt / tau_plus)
/// - If post fired before pre (anti-causal): Δw = -a_minus × exp(-Δt / tau_minus)
/// - If same tick: no change
///
/// Creates edges for causal pairs if none exists. Returns number of edges modified.
///
/// # Examples
///
/// ```
/// use petgraph::stable_graph::StableDiGraph;
/// use petgraph_live::hebbian::{stdp_update, StdpConfig};
///
/// let mut g = StableDiGraph::<(), f64>::new();
/// let a = g.add_node(());
/// let b = g.add_node(());
/// g.add_edge(a, b, 0.5);
///
/// // a fires at tick 1, b fires at tick 3 → a→b is causal (strengthened)
/// let activations = vec![(a, 1.0, 1), (b, 1.0, 3)];
/// let modified = stdp_update(&mut g, &activations, &StdpConfig::default());
/// assert!(modified > 0);
/// assert!(*g.edge_weight(0.into()).unwrap() > 0.5);
/// ```
pub fn stdp_update<N, Ty: EdgeType>(
    graph: &mut StableGraph<N, f64, Ty>,
    activations: &[TimedActivation],
    config: &StdpConfig,
) -> usize {
    let mut modified = 0;
    for i in 0..activations.len() {
        let j_start = if Ty::is_directed() { 0 } else { i + 1 };
        for j in j_start..activations.len() {
            if i == j {
                continue;
            }
            let (ni, _si, ti) = activations[i];
            let (nj, _sj, tj) = activations[j];
            if ti == tj {
                continue;
            }

            // For directed: consider edge i→j
            // pre=i, post=j: if ti < tj → causal, else anti-causal
            let dt = (tj as f64) - (ti as f64);
            let delta_w = if dt > 0.0 {
                config.a_plus * (-dt / config.tau_plus).exp()
            } else {
                -config.a_minus * (dt / config.tau_minus).exp()
            };

            if let Some(idx) = graph.find_edge(ni, nj) {
                if let Some(w) = graph.edge_weight_mut(idx) {
                    *w += delta_w;
                    modified += 1;
                }
            } else if delta_w > 0.0 {
                graph.add_edge(ni, nj, delta_w);
                modified += 1;
            }

            // For directed graphs, also process j→i edge
            if Ty::is_directed() {
                let delta_w_rev = -delta_w; // reversed direction
                if let Some(idx) = graph.find_edge(nj, ni) {
                    if let Some(w) = graph.edge_weight_mut(idx) {
                        *w += delta_w_rev;
                        modified += 1;
                    }
                } else if delta_w_rev > 0.0 {
                    graph.add_edge(nj, ni, delta_w_rev);
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
    use petgraph::stable_graph::{StableDiGraph, StableUnGraph};

    #[test]
    fn causal_pair_strengthened() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);

        // a fires at tick 1, b fires at tick 2 → a→b is causal
        stdp_update(&mut g, &[(a, 1.0, 1), (b, 1.0, 2)], &StdpConfig::default());
        assert!(*g.edge_weight(0.into()).unwrap() > 0.5);
    }

    #[test]
    fn anti_causal_pair_weakened() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);

        // b fires at tick 1, a fires at tick 3 → a→b is anti-causal (post fired first)
        stdp_update(&mut g, &[(a, 1.0, 3), (b, 1.0, 1)], &StdpConfig::default());
        assert!(*g.edge_weight(0.into()).unwrap() < 0.5);
    }

    #[test]
    fn strength_depends_on_time_difference() {
        let config = StdpConfig::default();

        // Close in time → stronger effect
        let mut g1 = StableDiGraph::<(), f64>::new();
        let a1 = g1.add_node(());
        let b1 = g1.add_node(());
        g1.add_edge(a1, b1, 0.5);
        stdp_update(&mut g1, &[(a1, 1.0, 1), (b1, 1.0, 2)], &config);
        let w_close = *g1.edge_weight(0.into()).unwrap();

        // Far in time → weaker effect
        let mut g2 = StableDiGraph::<(), f64>::new();
        let a2 = g2.add_node(());
        let b2 = g2.add_node(());
        g2.add_edge(a2, b2, 0.5);
        stdp_update(&mut g2, &[(a2, 1.0, 1), (b2, 1.0, 10)], &config);
        let w_far = *g2.edge_weight(0.into()).unwrap();

        assert!(w_close > w_far);
    }

    #[test]
    fn beyond_tau_window_minimal_effect() {
        let config = StdpConfig {
            tau_plus: 2.0,
            ..StdpConfig::default()
        };

        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);

        // dt=20, tau=2 → exp(-10) ≈ 0.0000454 → negligible
        stdp_update(&mut g, &[(a, 1.0, 1), (b, 1.0, 21)], &config);
        let w = *g.edge_weight(0.into()).unwrap();
        assert!((w - 0.5).abs() < 0.0001);
    }

    #[test]
    fn same_tick_no_change() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);

        let modified = stdp_update(&mut g, &[(a, 1.0, 5), (b, 1.0, 5)], &StdpConfig::default());
        assert_eq!(modified, 0);
        assert_eq!(*g.edge_weight(0.into()).unwrap(), 0.5);
    }

    #[test]
    fn undirected_stdp() {
        let mut g = StableUnGraph::<(), f64>::with_capacity(0, 0);
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);

        // a fires before b → causal → strengthen
        let modified = stdp_update(&mut g, &[(a, 1.0, 1), (b, 1.0, 3)], &StdpConfig::default());
        assert_eq!(modified, 1);
        assert!(*g.edge_weight(0.into()).unwrap() > 0.5);
    }
}
