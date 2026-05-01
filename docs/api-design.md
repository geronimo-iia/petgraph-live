# petgraph-live — API Design

**Status:** pre-implementation  
**Version target:** 0.1.0

---

## Crate structure

```
petgraph_live
├── cache          GenerationCache<G> — hot-reload, generation-keyed
├── metrics        Distance-based graph characteristics (weighted + unweighted)
├── connect        Articulation points, bridges
├── shortest_path  Floyd-Warshall, Seidel, shortest_distances
├── mst            Prim, Borůvka (Kruskal re-exported from petgraph)
├── snapshot       Serde-based disk persistence (feature "snapshot")
└── live           GraphState<G> — composes cache + snapshot (feature "snapshot")

Out of scope: adj_matrix / spec (nalgebra), generate (rand)
```

---

## Module: `cache`

### `GenerationCache<G>`

Generic hot-reload cache keyed on a monotonic `u64` generation counter supplied
by the caller. The cache is blind to what the graph contains — no domain logic inside.

```rust
/// Hot-reload graph cache keyed on an external generation counter.
///
/// `G` is any graph type. `generation` is a monotonic `u64` controlled by
/// the caller — bump it whenever the underlying data source changes (index
/// commit, file watch event, etc.).
///
/// Only one graph is cached at a time (the last successfully built one).
/// Filtered or derived views must be computed from the cached graph by
/// the caller.
///
/// # Example
/// ```rust
/// use petgraph_live::cache::GenerationCache;
///
/// let cache: GenerationCache<MyGraph> = GenerationCache::new();
///
/// // Returns cached Arc<MyGraph> if generation matches, else calls build.
/// let graph: Arc<MyGraph> = cache.get_or_build(current_gen, || build_my_graph())?;
/// ```
pub struct GenerationCache<G> {
    inner: RwLock<Option<CacheEntry<G>>>,
}

struct CacheEntry<G> {
    graph:      Arc<G>,
    generation: u64,
}

impl<G> GenerationCache<G> {
    pub fn new() -> Self;

    /// Return cached graph if `generation` matches, else call `build` and cache.
    /// `build` is only called on stale or empty cache.
    pub fn get_or_build<F, E>(&self, generation: u64, build: F) -> Result<Arc<G>, E>
    where
        F: FnOnce() -> Result<G, E>;

    /// Force cache invalidation. Next call to `get_or_build` rebuilds.
    pub fn invalidate(&self);

    /// Cached generation, or `None` if cache is empty.
    pub fn current_generation(&self) -> Option<u64>;
}
```

No configuration struct — the cache has no behaviour to configure. Domain-level
concerns (e.g. minimum node thresholds for community detection) belong in the
caller's `build` closure, not in this crate.

---

## Module: `metrics`

Distance-based graph characteristics. All functions work on any graph type
satisfying the relevant `petgraph::visit` traits.

### Unweighted (hop count, BFS-based — O(n·(n+e)))

```rust
use petgraph_live::metrics;

metrics::eccentricity(&graph, node)  // f32 — max distance from node
metrics::radius(&graph)              // Option<f32>
metrics::diameter(&graph)            // Option<f32>
metrics::center(&graph)              // Vec<G::NodeId> — nodes with ecc == radius
metrics::periphery(&graph)           // Vec<G::NodeId> — nodes with ecc == diameter
metrics::girth(&graph)               // Option<u32> — shortest cycle length, None if acyclic
```

### Weighted (edge weights, Bellman-Ford — O(n·e))

```rust
metrics::weighted_eccentricity(&graph, node, |e| *e.weight())  // Option<K>
metrics::weighted_radius(&graph, |e| *e.weight())              // Option<K>
metrics::weighted_diameter(&graph, |e| *e.weight())      // Option<K>
metrics::weighted_center(&graph, |e| *e.weight())        // Vec<G::NodeId>
metrics::weighted_periphery(&graph, |e| *e.weight())     // Vec<G::NodeId>
```

Returns `None` on empty graph or negative cycle. Returns `INFINITY` for
disconnected graphs (diameter/eccentricity undefined — caller decides whether
to restrict to the largest connected component).

---

## Module: `connect`

Structural connectivity analysis. DFS-based, O(n+e). Undirected graphs only
(articulation points and bridges are undirected concepts).

```rust
use petgraph_live::connect;

connect::articulation_points(&graph)  // Vec<G::NodeId>
connect::find_bridges(&graph)         // Vec<(G::NodeId, G::NodeId)>
```

- **Articulation points** — nodes whose removal increases connected component count (Tarjan DFS).
- **Bridges** — edges whose removal increases connected component count (Tarjan DFS).

---

## Module: `shortest_path`

Own implementations plus selective petgraph re-exports where petgraph already
covers the algorithm well.

```rust
use petgraph_live::shortest_path;

// Own implementations
shortest_path::shortest_distances(&graph, start)      // Vec<f32> — BFS distances from start
shortest_path::floyd_warshall(&graph, |e| cost)       // Result<Vec<Vec<K>>, NegativeCycle>
shortest_path::distance_map(&graph, |e| cost)         // HashMap<(NodeId,NodeId), K>
shortest_path::seidel(&graph)                         // unweighted APSP for undirected, O(n^ω log n)
shortest_path::apd(&graph)                            // all-pairs distances matrix (Seidel)

// Re-exported from petgraph
shortest_path::dijkstra(...)
shortest_path::bellman_ford(...)
shortest_path::astar(...)
shortest_path::spfa(...)
shortest_path::johnson(...)
shortest_path::k_shortest_path(...)
shortest_path::NegativeCycle
```

---

## Module: `mst`

Minimum spanning tree algorithms. Own implementations for Prim and Borůvka;
Kruskal re-exported from petgraph.

```rust
use petgraph_live::mst;

mst::prim(&graph, |e| *e.weight())    // Vec<(G::NodeId, G::NodeId)>
mst::boruvka(&graph, |e| *e.weight()) // Vec<(G::NodeId, G::NodeId)>
mst::kruskal(&graph)                  // impl Iterator<Item = Element<N,E>> (petgraph re-export)
```

---

## Module: `snapshot` (feature-gated)

Feature flag: `snapshot` (implies `petgraph/serde-1` + `serde/derive`).
Optional compression sub-feature: `snapshot-zstd` (adds `zstd` dep).

### File naming

Key is encoded in the filename — no key stored inside the file body.

```
{name}-{sanitized_key}.snap        (bincode, uncompressed)
{name}-{sanitized_key}.snap.zst    (bincode, zstd compressed)
{name}-{sanitized_key}.json        (json, uncompressed)
{name}-{sanitized_key}.json.zst    (json, zstd compressed)
```

`sanitize_key`: replace any char outside `[a-zA-Z0-9_.-]` with `_`. Git SHAs
and `u64` strings pass through unchanged.

Two saves with the same key = same filename = idempotent overwrite.
Rotation keeps the latest `keep` files by filesystem mtime.

### Format

```rust
pub enum SnapshotFormat {
    /// Bincode. Binary, fast, compact. Default.
    Bincode,
    /// JSON via serde_json. Human-readable, slower, larger.
    Json,
}
```

### Compression

```rust
pub enum Compression {
    None,
    Zstd { level: i32 },   // requires feature "snapshot-zstd"
}
```

### `SnapshotConfig`

```rust
pub struct SnapshotConfig {
    /// Directory where snapshots are stored.
    pub dir: PathBuf,
    /// Base name for snapshot files (no extension, no key).
    pub name: String,
    /// Validity key encoded in the filename.
    ///
    /// `load` looks for `{name}-{sanitized_key}.*` in `dir`.
    /// `None` = load the most recent file regardless of key (used by `GraphState`).
    ///
    /// The key is opaque — choose whatever uniquely identifies your source data:
    ///
    /// | Source | Key to use |
    /// |---|---|
    /// | Git-backed data | current commit SHA |
    /// | Index generation counter | `generation.to_string()` |
    /// | File/directory content | SHA256 hex |
    /// | Static graph | any fixed constant |
    pub key: Option<String>,
    /// Serialization format. Default: Bincode.
    pub format: SnapshotFormat,
    /// Compression. Default: None.
    pub compression: Compression,
    /// Number of snapshots to retain. Oldest deleted on save. Default: 3.
    pub keep: usize,
}
```

### `SnapshotMeta`

Serialized as the first section of the file. Readable without deserializing
the full graph (bincode: length-prefixed; JSON: `"meta"` field).

```rust
pub struct SnapshotMeta {
    pub key:                   String,   // redundant with filename, for convenience
    pub format:                SnapshotFormat,
    pub compression:           Compression,
    pub node_count:            usize,
    pub edge_count:            usize,
    pub created_at:            u64,      // Unix timestamp seconds
    pub petgraph_live_version: String,
}
```

### `SnapshotError`

```rust
pub enum SnapshotError {
    Io(std::io::Error),
    KeyNotFound { key: String },          // no file matching that key exists
    ParseError(String),
    CompressionError(String),
    NoSnapshotFound,                      // dir has no matching files at all
    InvalidKey(String),                   // key sanitizes to empty string
}
```

`KeyMismatch` is gone — key is now the filename, so a wrong key = `KeyNotFound`,
not a content comparison.

### API

```rust
use petgraph_live::snapshot::{SnapshotConfig, SnapshotError, SnapshotMeta};

// Save graph. Filename = {name}-{sanitized_key}.{ext}.
// Atomic write (temp + rename). Prunes old snapshots per `keep`.
save(&cfg, &graph) -> Result<(), SnapshotError>;

// Load snapshot matching cfg.key (Some) or most recent (None).
// Returns Ok(None) if not found.
load<G>(&cfg) -> Result<Option<G>, SnapshotError>
where G: DeserializeOwned;

// Load or build. Falls back to build on KeyNotFound / NoSnapshotFound.
// Saves result to disk after build.
load_or_build<G, F>(&cfg, build: F) -> Result<G, SnapshotError>
where
    G: Serialize + DeserializeOwned,
    F: FnOnce() -> Result<G, SnapshotError>;

// Read metadata from filename-matched snapshot without deserializing graph.
inspect(&cfg) -> Result<Option<SnapshotMeta>, SnapshotError>;

// List all snapshots in dir matching cfg.name, oldest first.
list(&cfg) -> Result<Vec<(PathBuf, SnapshotMeta)>, SnapshotError>;

// Delete all snapshots matching cfg.name in cfg.dir.
purge(&cfg) -> Result<usize, SnapshotError>;
```

### Example

```rust
let cfg = SnapshotConfig {
    dir:         PathBuf::from("/state/snapshots"),
    name:        "graph".to_string(),
    key:         Some(current_git_sha()),
    format:      SnapshotFormat::Bincode,
    compression: Compression::None,
    keep:        3,
};

// Load from {dir}/graph-{sha}.snap or build if absent.
let graph = load_or_build(&cfg, || build_graph_from_index())?;

// Inspect without loading graph
if let Some(meta) = inspect(&cfg)? {
    println!("{} nodes, key={}", meta.node_count, meta.key);
}

// List all retained snapshots — key visible from filename
for (path, meta) in list(&cfg)? {
    println!("{}: {} nodes", path.display(), meta.node_count);
}
```

---

## Module: `live` (feature "snapshot")

Composes `GenerationCache<G>` + `snapshot` into a single managed lifecycle.
Caller provides: how to compute the current key, how to build the graph.
`GraphState` handles: cold start, cache hits, rebuild-on-key-change, snapshot rotation.

### `GraphStateConfig`

```rust
pub struct GraphStateConfig {
    pub snapshot: SnapshotConfig,
}

impl GraphStateConfig {
    pub fn new(snapshot: SnapshotConfig) -> Self;
}
```

Concurrency contract: `build_fn` runs outside any lock. Only the final
cache-swap step acquires a write lock (microseconds). Concurrent `get()`
callers are never blocked during a rebuild — they read the stale `Arc`
until the swap completes.

### `GraphState<G>`

```rust
pub struct GraphState<G> {
    // private fields
}

impl<G> GraphState<G>
where
    G: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    pub fn builder(config: GraphStateConfig) -> GraphStateBuilder<G>;

    /// Return cached graph. No key check. Hot path — zero overhead.
    pub fn get(&self) -> Result<Arc<G>, SnapshotError>;

    /// Check key_fn() against current key. Rebuild + save snapshot if stale.
    /// Use after a known data change (file watch, index rebuild).
    pub fn get_fresh(&self) -> Result<Arc<G>, SnapshotError>;

    /// Force rebuild regardless of key. Saves new snapshot.
    pub fn rebuild(&self) -> Result<Arc<G>, SnapshotError>;

    /// Key the currently cached graph was built from.
    pub fn current_key(&self) -> String;

    /// Current generation counter (u64, process-lifetime only).
    pub fn generation(&self) -> u64;
}
```

### `GraphStateBuilder<G>`

```rust
pub struct GraphStateBuilder<G> { /* private */ }

impl<G> GraphStateBuilder<G>
where
    G: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    /// How to compute the current validity key. Called at init and in get_fresh().
    pub fn key_fn(
        self,
        f: impl Fn() -> Result<String, SnapshotError> + Send + Sync + 'static,
    ) -> Self;

    /// How to build the graph from scratch.
    /// Called on cold start (no snapshot), key change, or explicit rebuild().
    pub fn build_fn(
        self,
        f: impl Fn() -> Result<G, SnapshotError> + Send + Sync + 'static,
    ) -> Self;

    /// Provide current key directly — avoids calling key_fn at init.
    /// Use when caller already has the key (e.g. just read git HEAD).
    pub fn current_key(self, key: impl Into<String>) -> Self;

    /// Consume builder. Loads from snapshot if key matches, else builds.
    /// Returns Err if key_fn or build_fn not set.
    pub fn init(self) -> Result<GraphState<G>, SnapshotError>;
}
```

### Key management in `GraphState`

`GraphState` manages the key internally via `key_fn`. `SnapshotConfig::key`
must be set to `None` when used inside `GraphState` — the builder enforces this.
`get_fresh()` calls `key_fn()`, compares to `current_key`, and rebuilds if different.
On rebuild: new snapshot saved as `{name}-{new_key}.{ext}`, old ones rotated per `keep`.

### Example

```rust
let snapshot_cfg = SnapshotConfig {
    dir:         PathBuf::from("/state/snapshots"),
    name:        "graph".into(),
    key:         None,             // managed by GraphState
    format:      SnapshotFormat::Bincode,
    compression: Compression::None,
    keep:        3,
};

let config = GraphStateConfig::new(snapshot_cfg);

let state: GraphState<MyGraph> = GraphState::builder(config)
    .key_fn(|| Ok(current_git_sha()))
    .build_fn(|| build_graph_from_index())
    .current_key(current_git_sha())  // optional — avoids calling key_fn twice at init
    .init()?;

// Hot path — every request
let graph: Arc<MyGraph> = state.get()?;

// After file-watch ingest — checks key, rebuilds only if changed
let graph: Arc<MyGraph> = state.get_fresh()?;

// Force rebuild regardless
let graph: Arc<MyGraph> = state.rebuild()?;

println!("key={} gen={}", state.current_key(), state.generation());
```

---

## Error handling

| Module | Error type |
|---|---|
| `cache` | caller-supplied `E` from the `build` closure |
| `metrics` | no `Result` — returns `None` for degenerate/empty cases |
| `shortest_path` | `NegativeCycle` (re-export from petgraph) |
| `snapshot` | `SnapshotError` — distinguishes `Io`, `KeyNotFound`, `InvalidKey`, `ParseError`, `CompressionError`, `NoSnapshotFound` |

---

## Dependency surface

| Feature | Dependencies added |
|---|---|
| default (`cache`, `metrics`, `connect`, `shortest_path`, `mst`) | `petgraph 0.8` only |
| `snapshot` | `serde`, `serde_json`, `bincode`, `petgraph/serde-1` |
| `snapshot-zstd` | `snapshot` + `zstd` |

No `nalgebra`, `rand`, `rayon`, or async runtime.

---

## Scope boundary vs graphalgs

| graphalgs module | petgraph-live |
|---|---|
| `metrics` | ✅ full port (weighted + unweighted) |
| `connect` (bridges, articulation) | ✅ full port |
| `shortest_path` (Floyd-Warshall, Seidel, re-exports) | ✅ full port |
| `mst` (Prim, Borůvka, Kruskal) | ✅ full port |
| `elementary_circuits` (Tarjan) | deferred — v0.2.0 |
| `tournament` | deferred — v0.2.0 |
| `coloring` (DSATUR) | deferred — v0.2.0 |
| `generate` (random graphs, complement) | ❌ skipped — pulls `rand`, niche use case |
| `adj_matrix`, `spec` (nalgebra) | ❌ skipped — `nalgebra` is a heavy dep, petgraph already has `to_adjacency_matrix()` |

---

## Usage from llm-wiki

| llm-wiki concept | petgraph-live API |
|---|---|
| `get_or_build_graph` + `CachedGraph` | `cache::GenerationCache<WikiGraph>` |
| graph snapshot improvement | `snapshot::save` / `snapshot::load` |
| future: `diameter`, `center` on wiki graph | `metrics` module |
| future: find structurally critical pages | `connect::articulation_points` |
