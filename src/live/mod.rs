//! Live graph state — composes [`GenerationCache`](crate::cache::GenerationCache) with snapshot lifecycle.
//!
//! [`GraphState`] is the single entry point for applications that need a managed,
//! versioned graph that survives process restarts.  The caller supplies two
//! closures — one to compute the *current key* (e.g. a git SHA, a file hash, or a
//! monotonic counter) and one to *build* the graph from scratch — and
//! [`GraphState`] takes care of the rest:
//!
//! * **Cold start** — loads the latest matching snapshot from disk, or calls
//!   `build_fn` and persists the result.
//! * **Cache hits** — [`get`](GraphState::get) returns the in-memory
//!   [`Arc<G>`](std::sync::Arc) without any I/O.
//! * **Key-change detection** — [`get_fresh`](GraphState::get_fresh) compares the
//!   current key against the last-seen key and rebuilds only when they differ.
//! * **Forced rebuild** — [`rebuild`](GraphState::rebuild) always invokes
//!   `build_fn`, snapshots the result, and bumps the generation counter.
//!
//! # Concurrency contract
//!
//! * `build_fn` and `key_fn` are called **outside any lock**.
//! * A short **write lock** is acquired only to swap the cached `Arc<G>` and
//!   update the `current_key` / `generation` fields.
//! * Concurrent readers blocked on the write lock see at most one cache-swap
//!   latency spike; no reader holds a lock across I/O or computation.
//!
//! # Example
//!
//! ```no_run
//! use std::path::PathBuf;
//! use petgraph::Graph;
//! use petgraph_live::live::{GraphState, GraphStateConfig};
//! use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};
//!
//! fn current_git_sha() -> String { todo!() }
//! fn build_graph_from_index() -> Result<Graph<u32, ()>, Box<dyn std::error::Error>> { todo!() }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let snapshot_cfg = SnapshotConfig {
//!     dir:         PathBuf::from("/state/snapshots"),
//!     name:        "graph".into(),
//!     key:         None,             // managed by GraphState
//!     format:      SnapshotFormat::Bincode,
//!     compression: Compression::None,
//!     keep:        3,
//! };
//!
//! let config = GraphStateConfig::new(snapshot_cfg);
//!
//! let state: GraphState<Graph<u32, ()>> = GraphState::builder(config)
//!     .key_fn(|| Ok(current_git_sha()))
//!     .build_fn(|| build_graph_from_index().map_err(|e| {
//!         petgraph_live::snapshot::SnapshotError::Io(
//!             std::io::Error::other(e.to_string())
//!         )
//!     }))
//!     .current_key(current_git_sha())  // avoids calling key_fn twice at init
//!     .init()?;
//!
//! // Hot path — every request
//! let graph = state.get()?;
//!
//! // After file-watch ingest — checks key, rebuilds only if changed
//! let graph = state.get_fresh()?;
//!
//! // Force rebuild regardless
//! let graph = state.rebuild()?;
//!
//! println!("key={} gen={}", state.current_key(), state.generation());
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod state;

pub use config::GraphStateConfig;
pub use state::{GraphState, GraphStateBuilder};
