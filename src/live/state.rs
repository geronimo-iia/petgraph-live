use std::sync::{Arc, RwLock};

use crate::cache::GenerationCache;
use crate::snapshot::{SnapshotError, load};
use crate::snapshot::io::save_any;
use super::GraphStateConfig;

pub struct GraphState<G> {
    cache:    GenerationCache<G>,
    #[allow(dead_code)] // used by get_fresh/rebuild in later tasks
    config:   GraphStateConfig,
    #[allow(dead_code)] // used by get_fresh/rebuild in later tasks
    key_fn:   Arc<dyn Fn() -> Result<String, SnapshotError> + Send + Sync>,
    #[allow(dead_code)] // used by get_fresh/rebuild in later tasks
    build_fn: Arc<dyn Fn() -> Result<G, SnapshotError> + Send + Sync>,
    inner:    RwLock<GraphStateInner>,
}

struct GraphStateInner {
    current_key: String,
    generation:  u64,
}

pub struct GraphStateBuilder<G> {
    config:      GraphStateConfig,
    key_fn:      Option<Box<dyn Fn() -> Result<String, SnapshotError> + Send + Sync>>,
    build_fn:    Option<Box<dyn Fn() -> Result<G, SnapshotError> + Send + Sync>>,
    current_key: Option<String>,
}

impl<G> GraphState<G> {
    pub fn builder(config: GraphStateConfig) -> GraphStateBuilder<G> {
        GraphStateBuilder { config, key_fn: None, build_fn: None, current_key: None }
    }
}

impl<G: Send + Sync + 'static> GraphState<G> {
    pub fn get(&self) -> Result<Arc<G>, SnapshotError> {
        let generation = self.inner.read().unwrap().generation;
        self.cache.get_or_build(generation, || Err(SnapshotError::NoSnapshotFound))
    }

    pub fn current_key(&self) -> String {
        self.inner.read().unwrap().current_key.clone()
    }

    pub fn generation(&self) -> u64 {
        self.inner.read().unwrap().generation
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
}

impl<G> GraphStateBuilder<G>
where
    G: serde::Serialize + serde::de::DeserializeOwned
        + Send + Sync + 'static,
{
    pub fn init(self) -> Result<GraphState<G>, SnapshotError> {
        let key_fn: Arc<dyn Fn() -> Result<String, SnapshotError> + Send + Sync> = Arc::from(
            self.key_fn.ok_or_else(|| SnapshotError::InvalidKey("key_fn not set".into()))?,
        );
        let build_fn: Arc<dyn Fn() -> Result<G, SnapshotError> + Send + Sync> = Arc::from(
            self.build_fn.ok_or_else(|| SnapshotError::InvalidKey("build_fn not set".into()))?,
        );

        if self.config.snapshot.key.is_some() {
            return Err(SnapshotError::InvalidKey(
                "SnapshotConfig::key must be None for GraphState".into(),
            ));
        }

        let current_key = match self.current_key {
            Some(k) => k,
            None => key_fn()?,
        };

        let mut load_cfg = self.config.snapshot.clone();
        load_cfg.key = Some(current_key.clone());

        let (graph, built) = match load::<G>(&load_cfg) {
            Ok(Some(g)) => (g, false),
            Ok(None) | Err(SnapshotError::KeyNotFound { .. }) => (build_fn()?, true),
            Err(e) => return Err(e),
        };

        if built {
            let mut save_cfg = self.config.snapshot.clone();
            save_cfg.key = Some(current_key.clone());
            save_any(&save_cfg, &graph)?;
        }

        let cache = GenerationCache::new();
        cache.get_or_build(1u64, || Ok::<G, SnapshotError>(graph))?;

        Ok(GraphState {
            cache,
            config: self.config,
            key_fn,
            build_fn,
            inner: RwLock::new(GraphStateInner { current_key, generation: 1 }),
        })
    }
}
