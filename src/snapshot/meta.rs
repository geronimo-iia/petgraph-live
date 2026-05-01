use serde::{Deserialize, Serialize};

use crate::snapshot::config::{Compression, SnapshotFormat};

/// Metadata stored at the head of every snapshot file.
///
/// Readable via [`inspect`](crate::snapshot::inspect) without deserializing the full graph.
/// In bincode files the metadata is length-prefixed so only the header bytes are read.
/// In JSON files it occupies the `"meta"` key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMeta {
    /// Validity key — mirrors the filename segment, stored for convenience.
    pub key: String,
    /// Serialization format used when this snapshot was written.
    pub format: SnapshotFormat,
    /// Compression applied when this snapshot was written.
    pub compression: Compression,
    /// Number of nodes in the serialized graph.
    pub node_count: usize,
    /// Number of edges in the serialized graph.
    pub edge_count: usize,
    /// Unix timestamp (seconds) when this snapshot was created.
    pub created_at: u64,
    /// `CARGO_PKG_VERSION` of petgraph-live that wrote this snapshot.
    pub petgraph_live_version: String,
}

impl SnapshotMeta {
    pub fn new(
        key: &str,
        format: SnapshotFormat,
        compression: Compression,
        node_count: usize,
        edge_count: usize,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        Self {
            key: key.to_string(),
            format,
            compression,
            node_count,
            edge_count,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            petgraph_live_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
