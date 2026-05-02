//! `petgraph-live` — graph cache, snapshot, and algorithms for petgraph 0.8.
//!
//! # Features
//!
//! | Feature | Description |
//! |---|---|
//! | *(default)* | [`cache`], [`metrics`], [`connect`], [`shortest_path`], [`mst`] |
//! | `snapshot` | Disk persistence via bincode or JSON |
//! | `snapshot-zstd` | Zstd compression for snapshots (implies `snapshot`) |
//! | `snapshot-lz4` | LZ4 compression via `lz4_flex` (implies `snapshot`) |
//!
//! Enabling `snapshot` also gates [`live`], which composes the cache with snapshot lifecycle.
//!
//! # Quick start
//!
//! ```rust
//! use petgraph_live::cache::GenerationCache;
//! use std::sync::Arc;
//!
//! let cache: GenerationCache<Vec<u32>> = GenerationCache::new();
//! let g: Arc<Vec<u32>> = cache.get_or_build(1, || Ok::<_, ()>(vec![1, 2, 3])).unwrap();
//! assert_eq!(*g, vec![1, 2, 3]);
//! ```
//!
//! See [README](https://github.com/geronimo-iia/petgraph-live) for more examples.

pub mod cache;
pub mod connect;
pub mod metrics;
pub mod mst;
pub mod shortest_path;

#[cfg(feature = "snapshot")]
pub mod snapshot;

#[cfg(feature = "snapshot")]
pub mod live;
