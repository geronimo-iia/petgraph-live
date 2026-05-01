use thiserror::Error;

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("key not found: {key:?}")]
    KeyNotFound { key: String },
    #[error("invalid key: {0:?}")]
    InvalidKey(String),
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("compression error: {0}")]
    CompressionError(String),
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
