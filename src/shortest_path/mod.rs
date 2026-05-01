//! Shortest path algorithms.
//!
//! Own implementations plus petgraph re-exports.
//! Ported from [graphalgs](https://github.com/starovoid/graphalgs) (MIT).

mod floyd_warshall;
mod seidel;
mod shortest_distances;

pub use floyd_warshall::{distance_map, floyd_warshall};
#[allow(unused_imports)]
pub(crate) use seidel::apd;
pub use seidel::seidel;
pub use shortest_distances::shortest_distances;

pub use petgraph::algo::NegativeCycle;
pub use petgraph::algo::{astar, bellman_ford, dijkstra, johnson, k_shortest_path, spfa};
