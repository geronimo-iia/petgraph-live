//! SOKM (Self-Organizing Knowledge Map) — decay → strengthen → prune.

use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;

/// Configuration for SOKM dynamics.
#[derive(Debug, Clone, Copy)]
pub struct SokmConfig {
    /// Multiplicative decay per tick (default: 0.95).
    pub decay_factor: f64,
    /// Base increment for co-activation strengthening (default: 0.02).
    pub delta: f64,
    /// Prune threshold — edges below this are removed (default: 0.001).
    pub min_weight: f64,
    /// Formula for combining activation scores.
    pub formula: StrengthFormula,
}

impl Default for SokmConfig {
    fn default() -> Self {
        Self {
            decay_factor: 0.95,
            delta: 0.02,
            min_weight: 0.001,
            formula: StrengthFormula::default(),
        }
    }
}

/// Strengthening formula — how activation scores combine.
#[derive(Debug, Clone, Copy, Default)]
pub enum StrengthFormula {
    /// `delta * sa * sb` — product, confidence amplifier.
    #[default]
    Product,
    /// `delta * min(sa, sb)` — weakest link.
    Min,
    /// `delta * (sa + sb) / 2` — average.
    Average,
}

/// Report from a SOKM tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HebbianReport {
    /// Edges that had decay applied.
    pub decayed: usize,
    /// Co-activated pairs that were strengthened.
    pub strengthened: usize,
    /// Edges removed by pruning.
    pub pruned: usize,
}

/// Multiply all edge weights by `factor`. Returns count of edges decayed.
///
/// # Examples
///
/// ```
/// use petgraph::stable_graph::StableDiGraph;
/// use petgraph_live::hebbian::decay;
///
/// let mut g = StableDiGraph::<(), f64>::new();
/// let a = g.add_node(());
/// let b = g.add_node(());
/// g.add_edge(a, b, 1.0);
///
/// assert_eq!(decay(&mut g, 0.5), 1);
/// assert_eq!(*g.edge_weight(0.into()).unwrap(), 0.5);
/// ```
pub fn decay<N>(graph: &mut StableDiGraph<N, f64>, factor: f64) -> usize {
    let indices: Vec<_> = graph.edge_indices().collect();
    for &idx in &indices {
        if let Some(w) = graph.edge_weight_mut(idx) {
            *w *= factor;
        }
    }
    indices.len()
}

/// Strengthen edges between co-activated node pairs.
///
/// For each ordered pair (a, b) in `activated`, increments the edge weight
/// using the configured formula. Creates the edge if it does not exist.
///
/// Returns number of pairs strengthened.
///
/// # Examples
///
/// ```
/// use petgraph::stable_graph::StableDiGraph;
/// use petgraph_live::hebbian::{strengthen, SokmConfig};
///
/// let mut g = StableDiGraph::<(), f64>::new();
/// let a = g.add_node(());
/// let b = g.add_node(());
///
/// let activated = vec![(a, 1.0), (b, 0.5)];
/// let config = SokmConfig::default();
/// let count = strengthen(&mut g, &activated, &config);
///
/// assert_eq!(count, 2); // a→b and b→a
/// assert_eq!(g.edge_count(), 2);
/// ```
pub fn strengthen<N>(
    graph: &mut StableDiGraph<N, f64>,
    activated: &[(NodeIndex, f64)],
    config: &SokmConfig,
) -> usize {
    let mut count = 0;
    for i in 0..activated.len() {
        for j in 0..activated.len() {
            if i == j {
                continue;
            }
            let (na, sa) = activated[i];
            let (nb, sb) = activated[j];
            let increment = match config.formula {
                StrengthFormula::Product => config.delta * sa * sb,
                StrengthFormula::Min => config.delta * sa.min(sb),
                StrengthFormula::Average => config.delta * (sa + sb) / 2.0,
            };
            if let Some(idx) = graph.find_edge(na, nb) {
                if let Some(w) = graph.edge_weight_mut(idx) {
                    *w += increment;
                }
            } else {
                graph.add_edge(na, nb, increment);
            }
            count += 1;
        }
    }
    count
}

/// Remove edges with weight below `threshold`. Returns count removed.
///
/// # Examples
///
/// ```
/// use petgraph::stable_graph::StableDiGraph;
/// use petgraph_live::hebbian::prune;
///
/// let mut g = StableDiGraph::<(), f64>::new();
/// let a = g.add_node(());
/// let b = g.add_node(());
/// g.add_edge(a, b, 0.0001);
/// g.add_edge(b, a, 0.5);
///
/// assert_eq!(prune(&mut g, 0.001), 1);
/// assert_eq!(g.edge_count(), 1);
/// ```
pub fn prune<N>(graph: &mut StableDiGraph<N, f64>, threshold: f64) -> usize {
    let to_remove: Vec<_> = graph
        .edge_indices()
        .filter(|&idx| {
            graph
                .edge_weight(idx)
                .map_or(false, |&w| w < threshold)
        })
        .collect();
    let count = to_remove.len();
    for idx in to_remove {
        graph.remove_edge(idx);
    }
    count
}

/// Full SOKM tick: decay → strengthen → prune.
///
/// # Examples
///
/// ```
/// use petgraph::stable_graph::StableDiGraph;
/// use petgraph_live::hebbian::{sokm_tick, SokmConfig, HebbianReport};
///
/// let mut g = StableDiGraph::<(), f64>::new();
/// let a = g.add_node(());
/// let b = g.add_node(());
/// g.add_edge(a, b, 0.5);
///
/// let report = sokm_tick(&mut g, &[(a, 1.0), (b, 0.8)], &SokmConfig::default());
/// assert_eq!(report.decayed, 1);
/// assert!(report.strengthened > 0);
/// ```
pub fn sokm_tick<N>(
    graph: &mut StableDiGraph<N, f64>,
    activated: &[(NodeIndex, f64)],
    config: &SokmConfig,
) -> HebbianReport {
    let decayed = decay(graph, config.decay_factor);
    let strengthened = strengthen(graph, activated, config);
    let pruned = prune(graph, config.min_weight);
    HebbianReport {
        decayed,
        strengthened,
        pruned,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph() -> StableDiGraph<(), f64> {
        let mut g = StableDiGraph::new();
        let a = g.add_node(());
        let b = g.add_node(());
        let c = g.add_node(());
        g.add_edge(a, b, 0.5);
        g.add_edge(b, c, 0.3);
        g.add_edge(a, c, 0.01);
        g
    }

    #[test]
    fn decay_multiplies_all_weights() {
        let mut g = make_graph();
        assert_eq!(decay(&mut g, 0.5), 3);
        assert_eq!(*g.edge_weight(0.into()).unwrap(), 0.25);
        assert_eq!(*g.edge_weight(1.into()).unwrap(), 0.15);
    }

    #[test]
    fn decay_repeated_converges() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 1.0);

        for _ in 0..10 {
            decay(&mut g, 0.95);
        }
        let expected = 0.95f64.powi(10);
        assert!((g.edge_weight(0.into()).unwrap() - expected).abs() < 1e-10);
    }

    #[test]
    fn prune_removes_below_threshold() {
        let mut g = make_graph();
        assert_eq!(prune(&mut g, 0.1), 1); // removes 0.01 edge
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn prune_retains_above_threshold() {
        let mut g = make_graph();
        assert_eq!(prune(&mut g, 0.001), 0);
        assert_eq!(g.edge_count(), 3);
    }

    #[test]
    fn strengthen_creates_missing_edges() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        assert_eq!(g.edge_count(), 0);

        let config = SokmConfig::default();
        strengthen(&mut g, &[(a, 1.0), (b, 1.0)], &config);
        assert_eq!(g.edge_count(), 2); // a→b and b→a
    }

    #[test]
    fn strengthen_higher_scores_get_more() {
        let mut g1 = StableDiGraph::<(), f64>::new();
        let a1 = g1.add_node(());
        let b1 = g1.add_node(());

        let mut g2 = StableDiGraph::<(), f64>::new();
        let a2 = g2.add_node(());
        let b2 = g2.add_node(());

        let config = SokmConfig::default();
        strengthen(&mut g1, &[(a1, 0.5), (b1, 0.5)], &config);
        strengthen(&mut g2, &[(a2, 1.0), (b2, 1.0)], &config);

        let w1 = *g1.edge_weight(g1.find_edge(a1, b1).unwrap()).unwrap();
        let w2 = *g2.edge_weight(g2.find_edge(a2, b2).unwrap()).unwrap();
        assert!(w2 > w1);
    }

    #[test]
    fn strengthen_non_activated_pairs_unchanged() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        let c = g.add_node(());
        g.add_edge(b, c, 0.5);

        let config = SokmConfig::default();
        // only a is activated — no pairs to strengthen
        strengthen(&mut g, &[(a, 1.0)], &config);
        assert_eq!(*g.edge_weight(0.into()).unwrap(), 0.5);
    }

    #[test]
    fn strengthen_increments_existing_edge() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.5);

        let config = SokmConfig::default();
        strengthen(&mut g, &[(a, 1.0), (b, 1.0)], &config);
        // 0.5 + 0.02*1.0*1.0 = 0.52
        assert!((g.edge_weight(0.into()).unwrap() - 0.52).abs() < 1e-10);
    }

    #[test]
    fn sokm_tick_full_cycle() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0.0005); // will be pruned after decay

        let config = SokmConfig::default();
        let report = sokm_tick(&mut g, &[(a, 1.0), (b, 1.0)], &config);
        assert_eq!(report.decayed, 1);
        assert_eq!(report.strengthened, 2);
        // 0.0005 * 0.95 = 0.000475, then +0.02 = 0.020475, not pruned
        // But b→a is newly created at 0.02, not pruned either
        assert_eq!(report.pruned, 0);
    }

    #[test]
    fn sokm_tick_prunes_weak_edge() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        let c = g.add_node(());
        g.add_edge(a, c, 0.0005); // decays to 0.000475, no co-activation → pruned

        let config = SokmConfig::default();
        let report = sokm_tick(&mut g, &[(a, 1.0), (b, 1.0)], &config);
        assert_eq!(report.pruned, 1);
    }

    #[test]
    fn strength_formula_min() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());

        let config = SokmConfig {
            formula: StrengthFormula::Min,
            ..SokmConfig::default()
        };
        strengthen(&mut g, &[(a, 0.8), (b, 0.3)], &config);
        // delta * min(0.8, 0.3) = 0.02 * 0.3 = 0.006
        assert!((g.edge_weight(0.into()).unwrap() - 0.006).abs() < 1e-10);
    }

    #[test]
    fn strength_formula_average() {
        let mut g = StableDiGraph::<(), f64>::new();
        let a = g.add_node(());
        let b = g.add_node(());

        let config = SokmConfig {
            formula: StrengthFormula::Average,
            ..SokmConfig::default()
        };
        strengthen(&mut g, &[(a, 0.8), (b, 0.4)], &config);
        // delta * (0.8 + 0.4) / 2 = 0.02 * 0.6 = 0.012
        assert!((g.edge_weight(0.into()).unwrap() - 0.012).abs() < 1e-10);
    }
}
