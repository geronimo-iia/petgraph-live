use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::snapshot::error::SnapshotError;

/// Serialization format for snapshot files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SnapshotFormat {
    /// Binary format via bincode. Fast, compact. Default.
    Bincode,
    /// JSON via serde_json. Human-readable, slower, larger.
    Json,
}

/// Compression applied to snapshot files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Compression {
    /// No compression. Default.
    None,
    /// Zstd compression. Requires the `snapshot-zstd` feature.
    #[cfg(feature = "snapshot-zstd")]
    Zstd {
        /// Zstd compression level (1–22). Typical default: 3.
        level: i32,
    },
    /// LZ4 compression. Requires the `snapshot-lz4` feature.
    #[cfg_attr(docsrs, doc(cfg(feature = "snapshot-lz4")))]
    #[cfg(feature = "snapshot-lz4")]
    Lz4,
}

/// Configuration for snapshot save/load operations.
///
/// The key is encoded in the snapshot filename, not in the file body.
/// Two saves with the same key overwrite each other (idempotent).
///
/// | Source | Key to use |
/// |---|---|
/// | Git-backed data | current commit SHA |
/// | Index generation counter | `generation.to_string()` |
/// | File/directory content | SHA256 hex |
/// | Static graph | any fixed constant |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    /// Directory where snapshot files are stored.
    pub dir: PathBuf,
    /// Base name for snapshot files (no extension, no key).
    pub name: String,
    /// Validity key encoded in the snapshot filename.
    ///
    /// `load` looks for `{name}-{sanitized_key}.*` in `dir`.
    /// `None` loads the most recent file regardless of key.
    /// Always `None` after deserialization.
    #[serde(skip)]
    pub key: Option<String>,
    /// Serialization format.
    pub format: SnapshotFormat,
    /// Compression algorithm.
    pub compression: Compression,
    /// Number of snapshots to retain by mtime. Oldest deleted on save.
    pub keep: usize,
}

/// Replace any char outside `[a-zA-Z0-9_.-]` with `_`.
///
/// Git SHAs and `u64` strings pass through unchanged.
/// Returns `Err(InvalidKey)` if the result collapses to all underscores (i.e. empty after trimming `_`).
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
