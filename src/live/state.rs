use std::sync::{Arc, RwLock};

use crate::cache::GenerationCache;
use crate::snapshot::{SnapshotError, load};
use crate::snapshot::io::save_any;
use super::GraphStateConfig;

/// Managed, versioned graph with snapshot-backed persistence.
///
/// `GraphState<G>` composes a [`GenerationCache<G>`](crate::cache::GenerationCache)
/// with snapshot I/O so that callers never touch the filesystem directly.
///
/// Construct one with [`GraphState::builder`].  See [`crate::live`] for a
/// complete end-to-end example.
pub struct GraphState<G> {
    cache:    GenerationCache<G>,
    config:   GraphStateConfig,
    key_fn:   Arc<dyn Fn() -> Result<String, SnapshotError> + Send + Sync>,
    build_fn: Arc<dyn Fn() -> Result<G, SnapshotError> + Send + Sync>,
    inner:    RwLock<GraphStateInner>,
}

struct GraphStateInner {
    current_key: String,
    generation:  u64,
}

/// Builder for [`GraphState<G>`].
///
/// Obtained via [`GraphState::builder`].  Call [`key_fn`](Self::key_fn),
/// [`build_fn`](Self::build_fn), and optionally [`current_key`](Self::current_key)
/// before calling [`init`](Self::init).
pub struct GraphStateBuilder<G> {
    config:      GraphStateConfig,
    key_fn:      Option<Box<dyn Fn() -> Result<String, SnapshotError> + Send + Sync>>,
    build_fn:    Option<Box<dyn Fn() -> Result<G, SnapshotError> + Send + Sync>>,
    current_key: Option<String>,
}

impl<G> GraphState<G> {
    /// Create a [`GraphStateBuilder`] seeded with `config`.
    pub fn builder(config: GraphStateConfig) -> GraphStateBuilder<G> {
        GraphStateBuilder { config, key_fn: None, build_fn: None, current_key: None }
    }
}

impl<G: Send + Sync + 'static> GraphState<G> {
    /// Return the current in-memory graph.
    ///
    /// Returns the cached [`Arc<G>`] for the current generation.  No I/O,
    /// no key check.  Use this on the hot path (e.g. per-request).
    ///
    /// # Errors
    ///
    /// Returns [`SnapshotError`] if the cache is unexpectedly empty (should
    /// not happen after a successful [`init`](GraphStateBuilder::init)).
    ///
    /// # Examples
    ///
    /// ```
    /// # use petgraph::Graph;
    /// # use petgraph_live::live::{GraphState, GraphStateConfig};
    /// # use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let snap = SnapshotConfig {
    /// #     dir: dir.path().to_path_buf(), name: "g".into(), key: None,
    /// #     format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    /// # };
    /// # let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
    /// #     .key_fn(|| Ok("v1".into()))
    /// #     .build_fn(|| { let mut g = Graph::new(); g.add_node(1u32); Ok(g) })
    /// #     .init().unwrap();
    /// let graph = state.get().unwrap();
    /// assert_eq!(graph.node_count(), 1);
    /// ```
    pub fn get(&self) -> Result<Arc<G>, SnapshotError> {
        let generation = self.inner.read().unwrap().generation;
        self.cache.get_or_build(generation, || Err(SnapshotError::NoSnapshotFound))
    }

    /// The key that was active when the graph was last built or loaded.
    pub fn current_key(&self) -> String {
        self.inner.read().unwrap().current_key.clone()
    }

    /// Monotonically increasing counter, incremented on every rebuild.
    pub fn generation(&self) -> u64 {
        self.inner.read().unwrap().generation
    }
}

impl<G: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static> GraphState<G> {
    /// Return the graph, rebuilding if `key_fn` returns a different key.
    ///
    /// Calls `key_fn` and compares the result to [`current_key`](Self::current_key).
    /// If they match, returns the cached graph.  If they differ, calls `build_fn`,
    /// persists a new snapshot, bumps the generation, and returns the new graph.
    ///
    /// `build_fn` executes **outside any lock**; only the final cache-swap takes
    /// a write lock.
    ///
    /// # Errors
    ///
    /// Propagates errors from `key_fn`, `build_fn`, or snapshot I/O.
    ///
    /// # Examples
    ///
    /// ```
    /// # use petgraph::Graph;
    /// # use petgraph_live::live::{GraphState, GraphStateConfig};
    /// # use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let snap = SnapshotConfig {
    /// #     dir: dir.path().to_path_buf(), name: "g".into(), key: None,
    /// #     format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    /// # };
    /// # let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
    /// #     .key_fn(|| Ok("v1".into()))
    /// #     .build_fn(|| { let mut g = Graph::new(); g.add_node(1u32); Ok(g) })
    /// #     .init().unwrap();
    /// let graph = state.get_fresh().unwrap();
    /// assert_eq!(graph.node_count(), 1);
    /// ```
    pub fn get_fresh(&self) -> Result<Arc<G>, SnapshotError> {
        let new_key = (self.key_fn)()?;
        {
            let inner = self.inner.read().unwrap();
            if new_key == inner.current_key {
                let cur_gen = inner.generation;
                drop(inner);
                return self.cache.get_or_build(cur_gen, || Err(SnapshotError::NoSnapshotFound));
            }
        }
        // Key changed — build outside any lock
        let graph = (self.build_fn)()?;
        // Save snapshot
        let mut save_cfg = self.config.snapshot.clone();
        save_cfg.key = Some(new_key.clone());
        save_any(&save_cfg, &graph)?;
        // Write-lock: bump generation, update key
        let new_gen = {
            let mut inner = self.inner.write().unwrap();
            inner.generation += 1;
            inner.current_key = new_key;
            inner.generation
        };
        // Store in cache
        self.cache.invalidate();
        self.cache.get_or_build(new_gen, || Ok::<G, SnapshotError>(graph))
    }

    /// Force a full rebuild regardless of whether the key changed.
    ///
    /// Calls `key_fn` and `build_fn`, persists a new snapshot, and bumps the
    /// generation counter.  Use after an explicit invalidation event (e.g. a
    /// force-push that rewrites history for the same key).
    ///
    /// `build_fn` executes **outside any lock**.
    ///
    /// # Errors
    ///
    /// Propagates errors from `key_fn`, `build_fn`, or snapshot I/O.
    ///
    /// # Examples
    ///
    /// ```
    /// # use petgraph::Graph;
    /// # use petgraph_live::live::{GraphState, GraphStateConfig};
    /// # use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let snap = SnapshotConfig {
    /// #     dir: dir.path().to_path_buf(), name: "g".into(), key: None,
    /// #     format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    /// # };
    /// # let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
    /// #     .key_fn(|| Ok("v1".into()))
    /// #     .build_fn(|| { let mut g = Graph::new(); g.add_node(1u32); Ok(g) })
    /// #     .init().unwrap();
    /// let gen_before = state.generation();
    /// let graph = state.rebuild().unwrap();
    /// assert!(state.generation() > gen_before);
    /// assert_eq!(graph.node_count(), 1);
    /// ```
    pub fn rebuild(&self) -> Result<Arc<G>, SnapshotError> {
        let current_key = (self.key_fn)()?;
        let graph = (self.build_fn)()?;
        let mut save_cfg = self.config.snapshot.clone();
        save_cfg.key = Some(current_key.clone());
        save_any(&save_cfg, &graph)?;
        let new_gen = {
            let mut inner = self.inner.write().unwrap();
            inner.generation += 1;
            inner.current_key = current_key;
            inner.generation
        };
        self.cache.invalidate();
        self.cache.get_or_build(new_gen, || Ok::<G, SnapshotError>(graph))
    }
}

impl<G> GraphStateBuilder<G> {
    /// Set the closure that returns the current key.
    ///
    /// The key is an arbitrary string (git SHA, file hash, timestamp, …) that
    /// uniquely identifies the data version.  Must be set before calling
    /// [`init`](Self::init).
    pub fn key_fn(
        mut self,
        f: impl Fn() -> Result<String, SnapshotError> + Send + Sync + 'static,
    ) -> Self {
        self.key_fn = Some(Box::new(f));
        self
    }

    /// Set the closure that builds the graph from scratch.
    ///
    /// Called on cold start (no matching snapshot) and whenever
    /// [`get_fresh`](GraphState::get_fresh) or [`rebuild`](GraphState::rebuild)
    /// determines a new graph is needed.  Must be set before calling
    /// [`init`](Self::init).
    pub fn build_fn(
        mut self,
        f: impl Fn() -> Result<G, SnapshotError> + Send + Sync + 'static,
    ) -> Self {
        self.build_fn = Some(Box::new(f));
        self
    }

    /// Provide the current key directly, skipping the first `key_fn` call.
    ///
    /// Optional.  Useful when the caller already holds the key (e.g. from a
    /// command-line argument or environment variable) to avoid calling `key_fn`
    /// twice during initialisation.
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
    /// Build the [`GraphState`], performing a cold-start load or build.
    ///
    /// 1. Validates that `key_fn` and `build_fn` are set and that
    ///    `snapshot.key` is `None`.
    /// 2. Determines the current key (from [`current_key`](Self::current_key)
    ///    if provided, otherwise calls `key_fn`).
    /// 3. Attempts to load a matching snapshot; falls back to `build_fn`.
    /// 4. If the graph was freshly built, persists it as a snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`SnapshotError`] if required closures are missing, if
    /// `snapshot.key` is pre-set, if `key_fn` / `build_fn` fail, or if
    /// snapshot I/O fails.
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
