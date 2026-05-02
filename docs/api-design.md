# petgraph-live — API Design

**Version target:** 0.1.0

## Crate structure

```
petgraph_live
├── cache          GenerationCache<G> — hot-reload, generation-keyed
├── metrics        Distance-based graph characteristics (weighted + unweighted)
├── connect        Articulation points, bridges
├── shortest_path  Floyd-Warshall, Seidel, BFS distances, petgraph re-exports
├── mst            Prim, Borůvka (Kruskal re-exported from petgraph)
├── snapshot       Serde-based disk persistence (feature "snapshot")
└── live           GraphState<G> — composes cache + snapshot (feature "snapshot")
```

## Module: `cache`

```rust
use petgraph_live::cache::GenerationCache;

let cache: GenerationCache<MyGraph> = GenerationCache::new();

// Returns cached Arc<MyGraph> if generation matches, else calls build.
let graph: Arc<MyGraph> = cache.get_or_build(current_gen, || build_my_graph())?;

// Force next call to rebuild.
cache.invalidate();

// Currently cached generation (None if empty).
let gen: Option<u64> = cache.current_generation();
```

`get_or_build` takes a `FnOnce() -> Result<G, E>` — the error type is caller-defined.

## Module: `metrics`

All functions work on any graph type satisfying the relevant `petgraph::visit` traits.

### Unweighted

```rust
use petgraph_live::metrics;

metrics::eccentricity(&graph, node)  // f32
metrics::radius(&graph)              // Option<f32>
metrics::diameter(&graph)            // Option<f32>
metrics::center(&graph)              // Vec<G::NodeId>
metrics::periphery(&graph)           // Vec<G::NodeId>
metrics::girth(&graph)               // Option<u32> — None if acyclic
```

Returns `None` on empty or disconnected graph.

### Weighted

All weighted functions take an `edge_cost: F` closure where `F: FnMut(G::EdgeRef) -> K`.

```rust
metrics::weighted_eccentricity(&graph, node, |e| *e.weight())  // Option<K>
metrics::weighted_radius(&graph, |e| *e.weight())              // Option<K>
metrics::weighted_diameter(&graph, |e| *e.weight())            // Option<K>
metrics::weighted_center(&graph, |e| *e.weight())              // Vec<G::NodeId>
metrics::weighted_periphery(&graph, |e| *e.weight())           // Vec<G::NodeId>
```

Returns `None` on empty graph or negative cycle.

## Module: `connect`

Undirected graphs only.

```rust
use petgraph_live::connect;

connect::articulation_points(&graph)  // Vec<G::NodeId>
connect::find_bridges(&graph)         // Vec<(G::NodeId, G::NodeId)>
```

## Module: `shortest_path`

```rust
use petgraph_live::shortest_path;

shortest_path::shortest_distances(&graph, start)  // Vec<f32> — BFS, unreachable = INFINITY
shortest_path::floyd_warshall(&graph, |e| cost)   // Result<Vec<Vec<K>>, NegativeCycle>
shortest_path::distance_map(&graph, |e| cost)     // Result<HashMap<(NodeId,NodeId), K>, NegativeCycle>
shortest_path::seidel(&graph)                     // Vec<Vec<u32>> — unweighted APSP, undirected

// Re-exported from petgraph::algo
shortest_path::dijkstra(...)
shortest_path::bellman_ford(...)
shortest_path::astar(...)
shortest_path::spfa(...)
shortest_path::johnson(...)
shortest_path::k_shortest_path(...)
shortest_path::NegativeCycle
```

## Module: `mst`

```rust
use petgraph_live::mst;

mst::prim(&graph, |e| *e.weight())    // Vec<(G::NodeId, G::NodeId)>
mst::boruvka(&graph, |e| *e.weight()) // Vec<(G::NodeId, G::NodeId)>
mst::kruskal(&graph)                  // impl Iterator<Item = Element<N,E>> — petgraph re-export
```

## Module: `snapshot` (feature-gated)

Feature flag: `snapshot`. Optional compression: `snapshot-zstd`.

### File naming

```
{name}-{sanitized_key}.snap
{name}-{sanitized_key}.snap.zst
{name}-{sanitized_key}.json
{name}-{sanitized_key}.json.zst
```

Key sanitization: replace chars outside `[a-zA-Z0-9_.-]` with `_`. Same key → same filename → idempotent overwrite.

### Config types

```rust
pub enum SnapshotFormat { Bincode, Json }

pub enum Compression {
    None,
    Zstd { level: i32 },  // requires feature "snapshot-zstd"
}

pub struct SnapshotConfig {
    pub dir:         PathBuf,
    pub name:        String,
    pub key:         Option<String>,  // None = load most recent
    pub format:      SnapshotFormat,
    pub compression: Compression,
    pub keep:        usize,           // rotation: retain N newest files
}
```

`key = Some(k)` → load looks for `{name}-{sanitized(k)}.*`; missing file → `Err(KeyNotFound)`.
`key = None` → load returns most recent by mtime; empty dir → `Ok(None)`.

### API

```rust
use petgraph_live::snapshot::{self, SnapshotConfig, SnapshotError, SnapshotMeta};

snapshot::save(&cfg, &graph)                  // -> Result<(), SnapshotError>
snapshot::load::<G>(&cfg)                     // -> Result<Option<G>, SnapshotError>
snapshot::load_or_build::<G, F>(&cfg, build)  // -> Result<G, SnapshotError>
snapshot::inspect(&cfg)                       // -> Result<Option<SnapshotMeta>, SnapshotError>
snapshot::list(&cfg)                          // -> Result<Vec<(PathBuf, SnapshotMeta)>, SnapshotError>
snapshot::purge(&cfg)                         // -> Result<usize, SnapshotError>
```

`inspect` reads only the metadata header — graph bytes never loaded.

### SnapshotMeta

```rust
pub struct SnapshotMeta {
    pub key:                   String,
    pub format:                SnapshotFormat,
    pub compression:           Compression,
    pub node_count:            usize,
    pub edge_count:            usize,
    pub created_at:            u64,       // Unix timestamp seconds
    pub petgraph_live_version: String,
}
```

### SnapshotError

```rust
pub enum SnapshotError {
    Io(std::io::Error),
    KeyNotFound { key: String },
    ParseError(String),
    CompressionError(String),
    NoSnapshotFound,
    InvalidKey(String),
}
```

### Example

```rust
let cfg = SnapshotConfig {
    dir:         PathBuf::from("/state/snapshots"),
    name:        "graph".into(),
    key:         Some(current_git_sha()),
    format:      SnapshotFormat::Bincode,
    compression: Compression::None,
    keep:        3,
};

let graph = snapshot::load_or_build(&cfg, || build_graph_from_index())?;

if let Some(meta) = snapshot::inspect(&cfg)? {
    println!("{} nodes, key={}", meta.node_count, meta.key);
}
```

## Module: `live` (feature "snapshot")

Composes `GenerationCache<G>` + `snapshot` into a single managed lifecycle.
Caller provides: how to compute the current key, how to build the graph.
`GraphState` handles: cold start, warm start from snapshot, rebuild on key change, rotation.

### API

```rust
use petgraph_live::live::{GraphState, GraphStateConfig};

let config = GraphStateConfig::new(SnapshotConfig {
    dir: "/state".into(), name: "graph".into(), keep: 3,
    format: Default::default(), key: None,  // must be None — managed internally
    ..Default::default()
});

let state: GraphState<MyGraph> = GraphState::builder(config)
    .key_fn(|| Ok(current_git_sha()))
    .build_fn(|| build_graph_from_index())
    .current_key(current_git_sha())  // optional — avoids calling key_fn twice at init
    .init()?;

let graph = state.get()?;        // hot path — no key check
let graph = state.get_fresh()?;  // checks key_fn(), rebuilds if stale
let graph = state.rebuild()?;    // unconditional rebuild + snapshot save

state.current_key()  // -> String
state.generation()   // -> u64 (process-lifetime counter)
```

### Concurrency

`build_fn` runs outside any lock. Only the cache-swap step acquires a write lock
(microseconds). Concurrent `get()` callers are never blocked during a rebuild.

Two concurrent `get_fresh()` callers detecting a stale key both call `build_fn`
(last writer wins). Acceptable for v0.1 — coalescing deferred to v0.2.

## Error handling

| Module             | Error type                                      |
| ------------------ | ----------------------------------------------- |
| `cache`            | caller-supplied `E` from the `build` closure    |
| `metrics`          | no `Result` — `None` for degenerate/empty cases |
| `shortest_path`    | `NegativeCycle` (petgraph re-export)            |
| `snapshot`, `live` | `SnapshotError`                                 |

## Dependency surface

| Feature         | Dependencies added                                   |
| --------------- | ---------------------------------------------------- |
| default         | `petgraph 0.8` only                                  |
| `snapshot`      | `serde`, `serde_json`, `bincode`, `petgraph/serde-1` |
| `snapshot-zstd` | `snapshot` + `zstd`                                  |

No `nalgebra`, `rand`, `rayon`, or async runtime.
