use thiserror::Error;

/// Errors returned by snapshot operations.
#[derive(Debug, Error)]
pub enum SnapshotError {
    /// Filesystem I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// No file matching the requested key exists in the snapshot directory.
    #[error("key not found: {key:?}")]
    KeyNotFound { key: String },
    /// The key sanitizes to an empty string and cannot be used as a filename component.
    #[error("invalid key: {0:?}")]
    InvalidKey(String),
    /// Serialization or deserialization failed.
    #[error("parse error: {0}")]
    ParseError(String),
    /// Compression or decompression failed (zstd feature required for `.zst` files).
    #[error("compression error: {0}")]
    CompressionError(String),
    /// The snapshot directory contains no files matching the configured name.
    #[error("no snapshot found")]
    NoSnapshotFound,
}

// std::io::Error doesn't impl PartialEq; compare Io variants by ErrorKind
impl PartialEq for SnapshotError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(a), Self::Io(b)) => a.kind() == b.kind(),
            (Self::KeyNotFound { key: a }, Self::KeyNotFound { key: b }) => a == b,
            (Self::InvalidKey(a), Self::InvalidKey(b)) => a == b,
            (Self::ParseError(a), Self::ParseError(b)) => a == b,
            (Self::CompressionError(a), Self::CompressionError(b)) => a == b,
            (Self::NoSnapshotFound, Self::NoSnapshotFound) => true,
            _ => false,
        }
    }
}
