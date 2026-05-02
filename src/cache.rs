use std::sync::{Arc, RwLock};

struct CacheEntry<G> {
    graph: Arc<G>,
    generation: u64,
}

/// Hot-reload graph cache keyed on an external generation counter.
///
/// `G` is any graph type. `generation` is a monotonic `u64` controlled by the
/// caller — bump it whenever the underlying data source changes (index commit,
/// file-watch event, etc.).
///
/// Only one graph is cached at a time. Filtered or derived views must be
/// computed from the cached graph by the caller.
///
/// # Examples
///
/// ```
/// use petgraph_live::cache::GenerationCache;
///
/// let cache: GenerationCache<Vec<u32>> = GenerationCache::new();
/// assert_eq!(cache.current_generation(), None);
/// ```
pub struct GenerationCache<G> {
    inner: RwLock<Option<CacheEntry<G>>>,
}

impl<G> GenerationCache<G> {
    /// Create an empty cache with no cached graph.
    pub fn new() -> Self {
        GenerationCache {
            inner: RwLock::new(None),
        }
    }

    /// Return cached graph if `generation` matches, else call `build` and cache result.
    ///
    /// `build` is called only on a miss or stale entry. On error from `build`,
    /// the existing cache entry is left unchanged.
    ///
    /// Concurrent callers that both observe a miss may each call `build`
    /// independently. The last writer wins the cache slot. `build` must be
    /// idempotent — callers that need to prevent redundant work should
    /// serialize `get_or_build` at a higher level.
    ///
    /// # Examples
    ///
    /// ```
    /// use petgraph_live::cache::GenerationCache;
    /// use std::sync::Arc;
    ///
    /// let cache: GenerationCache<Vec<u32>> = GenerationCache::new();
    /// let g1: Arc<Vec<u32>> = cache.get_or_build(1, || Ok::<_, ()>(vec![1, 2, 3])).unwrap();
    /// assert_eq!(*g1, vec![1, 2, 3]);
    ///
    /// // Same generation — cached Arc returned, build closure not called.
    /// let g2 = cache.get_or_build(1, || Ok::<_, ()>(vec![9, 9])).unwrap();
    /// assert!(Arc::ptr_eq(&g1, &g2));
    /// ```
    pub fn get_or_build<F, E>(&self, generation: u64, build: F) -> Result<Arc<G>, E>
    where
        F: FnOnce() -> Result<G, E>,
    {
        {
            let guard = self.inner.read().unwrap();
            if let Some(entry) = guard.as_ref()
                && entry.generation == generation
            {
                return Ok(Arc::clone(&entry.graph));
            }
        }
        let graph = Arc::new(build()?);
        *self.inner.write().unwrap() = Some(CacheEntry {
            graph: Arc::clone(&graph),
            generation,
        });
        Ok(graph)
    }

    /// Force cache invalidation. Next `get_or_build` call always rebuilds.
    pub fn invalidate(&self) {
        *self.inner.write().unwrap() = None;
    }

    /// Generation of the currently cached graph, or `None` if cache is empty.
    pub fn current_generation(&self) -> Option<u64> {
        self.inner.read().unwrap().as_ref().map(|e| e.generation)
    }
}

impl<G> Default for GenerationCache<G> {
    fn default() -> Self {
        Self::new()
    }
}
