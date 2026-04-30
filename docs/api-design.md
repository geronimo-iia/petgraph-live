# petgraph-live — API Design

**Status:** pre-implementation  
**Version target:** 0.1.0

---

## Crate structure

```
petgraph_live
├── cache      GenerationCache<G> — hot-reload, generation-keyed
├── metrics    Diameter, radius, eccentricity, center, periphery
├── connect    Articulation points, bridges
└── snapshot   Serde-based disk persistence (optional feature)
```

---

## Module: `cache`

### `GenerationCacheConfig`

Controls behaviour shared by all cache operations.

```rust
/// Configuration for GenerationCache.
///
/// # Example
/// ```rust
/// let cfg = GenerationCacheConfig {
///     min_community_nodes: 30,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct GenerationCacheConfig {
    /// Minimum local node count before community algorithms run.
    /// Community detection on tiny graphs is noisy and expensive.
    /// Default: 30.
    pub min_community_nodes: usize,
}

impl Default for GenerationCacheConfig {
    fn default() -> Self {
        Self { min_community_nodes: 30 }
    }
}
```

### `GenerationCache<G>`

```rust
/// Hot-reload graph cache keyed on an external generation counter.
///
/// `G` is the graph type (any `petgraph::graph::DiGraph<N, E>` or custom).
/// `generation` is a monotonic `u64` controlled by the caller — bump it
/// whenever the underlying data source changes (index commit, file watch,
/// cache invalidation).
///
/// Only unfiltered / "canonical" graphs are cached. Filtered or derived views
/// should be computed from the cached graph on demand.
///
/// # Example
/// ```rust
/// use petgraph_live::cache::{GenerationCache, GenerationCacheConfig};
///
/// let cache: GenerationCache<MyGraph> = GenerationCache::new(GenerationCacheConfig::default());
///
/// // Returns cached Arc<MyGraph> if generation matches, else rebuilds.
/// let graph = cache.get_or_build(current_gen, || build_my_graph())?;
/// ```
pub struct GenerationCache<G> {
    config: GenerationCacheConfig,
    inner:  RwLock<Option<CacheEntry<G>>>,
}

struct CacheEntry<G> {
    graph:      Arc<G>,
    generation: u64,
}

impl<G> GenerationCache<G> {
    pub fn new(config: GenerationCacheConfig) -> Self;

    /// Return cached graph if `generation` matches, else call `build` and cache result.
    /// `build` is only called when cache is stale or empty.
    pub fn get_or_build<F, E>(&self, generation: u64, build: F) -> Result<Arc<G>, E>
    where
        F: FnOnce() -> Result<G, E>;

    /// Force cache invalidation. Next call to `get_or_build` will rebuild.
    pub fn invalidate(&self);

    /// Return current cached generation, or `None` if cache is empty.
    pub fn current_generation(&self) -> Option<u64>;
}
```

### Community extension: `CommunityCache<G>`

Wraps `GenerationCache` with community detection layered on top. Designed for
callers that need both the graph and Louvain community data — runs Louvain
exactly once per generation.

```rust
use petgraph_live::cache::{CommunityCache, CommunityData};

let cache: CommunityCache<MyGraph> = CommunityCache::new(config);

// Builds graph + runs Louvain once on cache miss.
// Returns cached results on hit.
let data: CommunityData<MyGraph> = cache.get_or_build(gen, || build_my_graph())?;

data.graph          // Arc<MyGraph>
data.community_map  // Option<Arc<HashMap<String, usize>>>  node_id → community_id
data.stats          // Option<CommunityStats>

pub struct CommunityData<G> {
    pub graph:         Arc<G>,
    pub community_map: Option<Arc<HashMap<String, usize>>>,
    pub stats:         Option<CommunityStats>,
}

pub struct CommunityStats {
    pub count:    usize,    // number of communities
    pub largest:  usize,    // largest community size
    pub smallest: usize,    // smallest community size
    pub isolated: Vec<String>, // node ids in singleton communities
}
```

The node id type for `community_map` keys is `String` — callers supply the
mapping function from `NodeIndex` to `String` as part of the `build` closure.

---

## Module: `metrics`

All metrics operate on any `petgraph::visit::IntoNodeIdentifiers + IntoNeighbors`
(matches `DiGraph`, `UnGraph`, `StableGraph`, etc.). Unweighted — hop count only.

```rust
use petgraph_live::metrics;
use petgraph::graph::DiGraph;

let g: DiGraph<(), ()> = ...;

let d  = metrics::diameter(&g);          // Option<usize>  — longest shortest path
let r  = metrics::radius(&g);            // Option<usize>  — shortest eccentricity
let ec = metrics::eccentricity(&g, n);   // Option<usize>  — max dist from node n
let c  = metrics::center(&g);            // Vec<NodeIndex> — nodes with ecc == radius
let p  = metrics::periphery(&g);         // Vec<NodeIndex> — nodes with ecc == diameter
```

All functions return `None` when graph is empty or disconnected (diameter/radius
undefined for disconnected graphs — caller decides whether to run on the largest
connected component).

Implementation: BFS from each node, O(n · (n + e)).

---

## Module: `connect`

```rust
use petgraph_live::connect;

let ap = connect::articulation_points(&g);  // Vec<NodeIndex>
let br = connect::bridges(&g);              // Vec<(NodeIndex, NodeIndex)>
```

- **Articulation points** — nodes whose removal increases the number of
  connected components. DFS-based (Tarjan), O(n + e).
- **Bridges** — edges whose removal increases the number of connected
  components. DFS-based, O(n + e).

Works on both directed and undirected graphs via `petgraph::visit` traits.

---

## Module: `snapshot` (feature-gated)

Feature flag: `snapshot` (implies `petgraph/serde-1` + `serde/derive`).

```rust
use petgraph_live::snapshot::{SnapshotConfig, save, load};

let cfg = SnapshotConfig {
    path:          PathBuf::from("/state/graph-snapshot.json"),
    commit_key:    "abc123".to_string(),  // git SHA or other stable key
};

// Persist to disk (atomic write via temp + rename).
save(&cfg, &graph)?;

// Restore. Returns None if file absent, key mismatch, or parse failure.
// Never returns an error for "expected" mismatches — only for I/O failures.
let maybe_graph: Option<MyGraph> = load(&cfg)?;
```

`G` must implement `Serialize + DeserializeOwned` — satisfied automatically when
`petgraph/serde-1` is enabled and `N: Serialize + DeserializeOwned`,
`E: Serialize + DeserializeOwned`.

Atomic write: `save` writes to `path.tmp` then `rename` to `path`. Safe against
crash mid-write.

---

## Configuration summary

| Config type | Scope | Key fields |
|---|---|---|
| `GenerationCacheConfig` | cache module | `min_community_nodes: usize` |
| `SnapshotConfig` | snapshot module | `path: PathBuf`, `commit_key: String` |

No global config. Each struct is constructed by the caller and passed into the
relevant function or type. All fields have documented defaults via `Default`.

---

## Error handling

All fallible functions return `Result<T, E>` where `E` is the caller-supplied
error type (cache) or `std::io::Error` (snapshot). No crate-level error enum in
v0.1.0 — avoids coupling. Revisit if error context becomes important.

---

## Dependency surface

| Feature | Dependencies added |
|---|---|
| default (cache, metrics, connect) | `petgraph 0.8` |
| `snapshot` | `serde`, `serde_json`, `petgraph/serde-1` |

No `rand`, `nalgebra`, `rayon`, or async runtime.

---

## Usage from llm-wiki

The immediate consumer is `llm-wiki`. Mapping:

| llm-wiki concept | petgraph-live API |
|---|---|
| `CachedGraph` + `get_or_build_graph` | `GenerationCache<WikiGraph>` |
| `community_map` + `community_stats` | `CommunityCache<WikiGraph>` |
| `graph_snapshot` improvement | `snapshot::save` / `snapshot::load` |
| `compute_communities` (Louvain) | internal to `CommunityCache` — caller doesn't call Louvain directly |
| `metrics::diameter` etc. | `metrics` module |
| `articulation_points` | `connect` module |
