use crate::snapshot::SnapshotError;
use super::GraphStateConfig;

pub struct GraphState<G> {
    _phantom: std::marker::PhantomData<G>,
}

pub struct GraphStateBuilder<G> {
    config:      GraphStateConfig,
    key_fn:      Option<Box<dyn Fn() -> Result<String, SnapshotError> + Send + Sync>>,
    build_fn:    Option<Box<dyn Fn() -> Result<G, SnapshotError> + Send + Sync>>,
    current_key: Option<String>,
}

impl<G> GraphState<G> {
    pub fn builder(config: GraphStateConfig) -> GraphStateBuilder<G> {
        GraphStateBuilder {
            config,
            key_fn: None,
            build_fn: None,
            current_key: None,
        }
    }
}

impl<G> GraphStateBuilder<G> {
    pub fn key_fn(
        mut self,
        f: impl Fn() -> Result<String, SnapshotError> + Send + Sync + 'static,
    ) -> Self {
        self.key_fn = Some(Box::new(f));
        self
    }

    pub fn build_fn(
        mut self,
        f: impl Fn() -> Result<G, SnapshotError> + Send + Sync + 'static,
    ) -> Self {
        self.build_fn = Some(Box::new(f));
        self
    }

    pub fn current_key(mut self, key: impl Into<String>) -> Self {
        self.current_key = Some(key.into());
        self
    }

    pub fn init(self) -> Result<GraphState<G>, SnapshotError> {
        if self.key_fn.is_none() {
            return Err(SnapshotError::InvalidKey("key_fn not set".into()));
        }
        if self.build_fn.is_none() {
            return Err(SnapshotError::InvalidKey("build_fn not set".into()));
        }
        if self.config.snapshot.key.is_some() {
            return Err(SnapshotError::InvalidKey(
                "SnapshotConfig::key must be None for GraphState".into(),
            ));
        }
        todo!("GraphState full init in Task 4")
    }
}
