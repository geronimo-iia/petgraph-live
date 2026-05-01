//! Minimum spanning tree algorithms.
//!
//! `kruskal` is re-exported from petgraph's `min_spanning_tree`.
//! `prim` and `boruvka` are ported from [graphalgs](https://github.com/starovoid/graphalgs) (MIT).

pub use petgraph::algo::min_spanning_tree as kruskal;

mod boruvka;
pub use boruvka::boruvka;

mod prim;
pub use prim::prim;
