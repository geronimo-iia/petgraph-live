//! Hebbian learning on graphs: self-organizing dynamics that modify edge weights.
//!
//! The primary algorithm is **SOKM** (Self-Organizing Knowledge Map):
//! decay → strengthen → prune per tick.
//!
//! # Examples
//!
//! ```
//! use petgraph::stable_graph::StableDiGraph;
//! use petgraph_live::hebbian::{sokm_tick, SokmConfig};
//!
//! let mut graph = StableDiGraph::<&str, f64>::new();
//! let a = graph.add_node("A");
//! let b = graph.add_node("B");
//! graph.add_edge(a, b, 0.5);
//!
//! let activated = vec![(a, 1.0), (b, 0.8)];
//! let report = sokm_tick(&mut graph, &activated, &SokmConfig::default());
//! ```

mod sokm;

pub use sokm::{
    decay, prune, sokm_tick, strengthen, HebbianReport, SokmConfig, StrengthFormula,
};
