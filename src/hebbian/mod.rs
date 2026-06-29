//! Hebbian learning on graphs: self-organizing dynamics that modify edge weights.
//!
//! Algorithms:
//! - **SOKM** — decay → strengthen → prune per tick (cooperative)
//! - **STDP** — spike-timing dependent plasticity (temporal)
//! - **Anti-Hebbian** — lateral inhibition (competitive)
//! - **Oja** — normalized Hebbian, self-converging weights
//! - **BCM** — homeostatic plasticity with sliding threshold
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

mod anti_hebbian;
mod bcm;
mod oja;
mod sokm;
mod stdp;

pub use anti_hebbian::{AntiHebbianConfig, anti_hebbian_update};
pub use bcm::{BcmConfig, BcmState, bcm_update};
pub use oja::{OjaConfig, oja_update};
pub use sokm::{HebbianReport, SokmConfig, StrengthFormula, decay, prune, sokm_tick, strengthen};
pub use stdp::{StdpConfig, TimedActivation, stdp_update};
