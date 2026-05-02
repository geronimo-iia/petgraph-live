use crate::snapshot::SnapshotConfig;

/// Configuration for a [`GraphState`](super::GraphState).
///
/// Wraps a [`SnapshotConfig`] that controls where snapshots are stored, which
/// format and compression they use, and how many past snapshots to retain.
///
/// **Important**: the `key` field of the inner [`SnapshotConfig`] must be `None`.
/// [`GraphState`](super::GraphState) manages the key itself; passing a pre-set key
/// will cause [`GraphStateBuilder::init`](super::GraphStateBuilder::init) to return
/// [`SnapshotError::InvalidKey`](crate::snapshot::SnapshotError::InvalidKey).
pub struct GraphStateConfig {
    /// Snapshot storage parameters (dir, name, format, compression, keep count).
    pub snapshot: SnapshotConfig,
}

impl GraphStateConfig {
    /// Create a new `GraphStateConfig` from a [`SnapshotConfig`].
    ///
    /// The `snapshot.key` field must be `None`; [`GraphState`](super::GraphState)
    /// injects the key at runtime based on what `key_fn` returns.
    pub fn new(snapshot: SnapshotConfig) -> Self {
        GraphStateConfig { snapshot }
    }
}
