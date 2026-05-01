//! Serde-based disk persistence for petgraph graphs.
//!
//! Enabled with the `snapshot` feature flag. Optional zstd compression requires
//! `snapshot-zstd`.
//!
//! Snapshots are stored as `{name}-{sanitized_key}.{ext}` files. The key is
//! encoded in the filename so two saves with the same key overwrite each other.
//! Rotation keeps the latest `keep` files by filesystem mtime.
//!
//! # Quick start
//!
//! ```rust
//! # use std::path::PathBuf;
//! use petgraph::Graph;
//! use petgraph_live::snapshot::{
//!     Compression, SnapshotConfig, SnapshotFormat, load_or_build, inspect, list,
//! };
//!
//! # let dir = tempfile::tempdir().unwrap();
//! let cfg = SnapshotConfig {
//!     dir: dir.path().to_path_buf(),
//!     name: "graph".into(),
//!     key: Some("v1".into()),
//!     format: SnapshotFormat::Bincode,
//!     compression: Compression::None,
//!     keep: 3,
//! };
//!
//! // Load from disk or build from scratch on first run.
//! let graph: Graph<String, String> = load_or_build(&cfg, || {
//!     let mut g: Graph<String, String> = Graph::new();
//!     let a = g.add_node("A".into());
//!     let b = g.add_node("B".into());
//!     g.add_edge(a, b, "edge".into());
//!     Ok(g)
//! })?;
//!
//! // Inspect metadata without loading the graph.
//! if let Some(meta) = inspect(&cfg)? {
//!     println!("{} nodes, key={}", meta.node_count, meta.key);
//! }
//!
//! // List all retained snapshots, oldest first.
//! for (path, meta) in list(&cfg)? {
//!     println!("{}: {} nodes", path.display(), meta.node_count);
//! }
//! # Ok::<_, petgraph_live::snapshot::SnapshotError>(())
//! ```

pub mod config;
pub mod error;
pub mod io;
pub mod meta;
pub mod rotation;

pub use config::{Compression, SnapshotConfig, SnapshotFormat, sanitize_key};
pub use error::SnapshotError;
pub use io::{inspect, list, load, load_or_build, purge, save};
pub use meta::SnapshotMeta;
