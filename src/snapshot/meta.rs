use serde::{Deserialize, Serialize};

use crate::snapshot::config::{Compression, SnapshotFormat};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub key: String,
    pub format: SnapshotFormat,
    pub compression: Compression,
    pub node_count: usize,
    pub edge_count: usize,
    pub created_at: u64,
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
