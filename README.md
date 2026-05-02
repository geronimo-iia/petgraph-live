# petgraph-live

> **Status: v0.3.0.** All modules implemented. API not yet stable.

Graph cache, snapshot, and algorithms for [`petgraph`](https://docs.rs/petgraph) 0.8.

## Purpose

`petgraph` is excellent for building and traversing graphs. `petgraph-live` adds
the operational layer missing for long-running processes:

- **Hot-reload cache** — generic `GenerationCache<G>` that reuses a built graph
  until an external counter (e.g. an index generation, a file watch event) signals
  a change. No redundant rebuilds in serve mode.

- **Disk snapshot** — persist and restore the cached graph across process restarts.
  Atomic writes, key-based validity check, silent fallback to rebuild on
  mismatch or corruption. Optional zstd compression.

- **Managed lifecycle** — `GraphState<G>` composes cache and snapshot into a
  single object: cold start, warm start from snapshot, stale-key rebuild, rotation.

- **Graph algorithms** — unweighted metrics and connectivity analysis on
  `petgraph 0.8` graphs, with no heavy dependencies (`nalgebra`, `rand`, etc.):
  - `metrics`: diameter, radius, eccentricity, center, periphery, girth
  - `connect`: articulation points, bridges
  - `shortest_path`: Floyd-Warshall, Seidel APSP, BFS distances, petgraph re-exports
  - `mst`: Prim, Borůvka, Kruskal (petgraph re-export)

All algorithms work on any `DiGraph<N, E>` — no domain concepts inside this crate.

## API

```rust
use petgraph_live::cache::GenerationCache;
use petgraph_live::metrics;
use petgraph_live::connect;

// Cache a graph, rebuild only when generation changes
let cache = GenerationCache::new();
let graph = cache.get_or_build(current_gen, || build_my_graph())?;

// Graph health metrics
let d = metrics::diameter(&graph);   // longest shortest path
let c = metrics::center(&graph);     // most central nodes

// Connectivity analysis
let ap = connect::articulation_points(&graph);
let br = connect::find_bridges(&graph);
```

With snapshot (requires `features = ["snapshot"]`):

```rust
use petgraph_live::live::{GraphState, GraphStateConfig};
use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};

// LZ4 compression — requires `features = ["snapshot-lz4"]`
#[cfg(feature = "snapshot-lz4")]
let compression = Compression::Lz4;

let config = GraphStateConfig::new(SnapshotConfig {
    dir: "/tmp/my-cache".into(),
    name: "wiki".into(),
    keep: 3,
    format: SnapshotFormat::Bincode,
    compression: Compression::None,
    key: None,  // managed internally
});

let state = GraphState::builder(config)
    .key_fn(|| Ok(current_git_sha()))
    .build_fn(|| Ok(build_graph()))
    .init()?;

let graph = state.get()?;           // hot path, no key check
let graph = state.get_fresh()?;     // checks key, rebuilds if stale
```

## Feature flags

| Flag | Adds |
|---|---|
| _(default)_ | `cache`, `metrics`, `connect`, `shortest_path`, `mst` |
| `snapshot` | `snapshot`, `live` |
| `snapshot-zstd` | zstd compression for snapshots (implies `snapshot`) |
| `snapshot-lz4` | LZ4 compression for snapshots via `lz4_flex` (implies `snapshot`) |

```toml
[dependencies]
petgraph-live = "0.3"

# With snapshot:
petgraph-live = { version = "0.3", features = ["snapshot"] }

# With zstd compression:
petgraph-live = { version = "0.3", features = ["snapshot-zstd"] }

# With LZ4 compression (faster decompression, larger files):
petgraph-live = { version = "0.3", features = ["snapshot-lz4"] }
```

## Motivation

Built as a companion to [`llm-wiki`](https://github.com/geronimo-iia/llm-wiki),
a git-backed wiki engine with MCP server. The graph cache and algorithm needs
there are generic enough to live in a standalone crate on `petgraph 0.8`.

The only available alternative (`graphalgs`) is pinned to `petgraph ^0.6.5` and
appears unmaintained (last release 2023). `petgraph-live` targets `petgraph 0.8`
only and has no plans to support older versions.

## Documentation

- [Roadmap](docs/roadmap.md)
- [Specifications](docs/specifications/README.md)
- [Release guide](docs/release.md)
- [API design](docs/api-design.md)

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
