use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::snapshot::error::SnapshotError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SnapshotFormat {
    Bincode,
    Json,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Compression {
    None,
    #[cfg(feature = "snapshot-zstd")]
    Zstd {
        level: i32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    pub dir: PathBuf,
    pub name: String,
    /// Validity key encoded in the snapshot filename. Always `None` after deserialization.
    #[serde(skip)]
    pub key: Option<String>,
    pub format: SnapshotFormat,
    pub compression: Compression,
    /// Number of snapshots to retain (by mtime). Oldest deleted on save.
    pub keep: usize,
}

/// Replace any char outside `[a-zA-Z0-9_.-]` with `_`.
/// Returns `Err(InvalidKey)` if the result is empty.
pub fn sanitize_key(key: &str) -> Result<String, SnapshotError> {
    let s: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.trim_matches('_').is_empty() {
        Err(SnapshotError::InvalidKey(key.to_string()))
    } else {
        Ok(s)
    }
}
