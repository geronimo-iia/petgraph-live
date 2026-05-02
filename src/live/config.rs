use crate::snapshot::SnapshotConfig;

pub struct GraphStateConfig {
    pub snapshot: SnapshotConfig,
}

impl GraphStateConfig {
    pub fn new(snapshot: SnapshotConfig) -> Self {
        GraphStateConfig { snapshot }
    }
}
