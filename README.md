# petgraph-live

> **Status: coming soon.** Implementation in progress. API not yet stable.

Graph cache, snapshot, and algorithms for [`petgraph`](https://docs.rs/petgraph) 0.8.

---

## Purpose

`petgraph` is excellent for building and traversing graphs. `petgraph-live` adds
the operational layer missing for long-running processes:

- **Hot-reload cache** — generic `GenerationCache<G>` that reuses a built graph
  until an external counter (e.g. an index generation, a file watch event) signals
  a change. No redundant rebuilds in serve mode.

- **Disk snapshot** — persist and restore the cached graph across process restarts.
  Atomic writes, commit-keyed validity check, silent fallback to rebuild on
  mismatch or corruption.

- **Graph algorithms** — unweighted metrics and connectivity analysis on
  `petgraph 0.8` graphs, with no heavy dependencies (`nalgebra`, `rand`, etc.):
  - `metrics`: diameter, radius, eccentricity, center, periphery
  - `connect`: articulation points, bridges

All algorithms work on any `DiGraph<N, E>` — no wiki or domain concepts inside
this crate.

---

## Planned API

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
let ap = connect::articulation_points(&graph);  // removal disconnects graph
let br = connect::find_bridges(&graph);          // edge removal disconnects graph
```

---

## Motivation

Built as a companion to [`llm-wiki`](https://github.com/geronimo-iia/llm-wiki),
a git-backed wiki engine with MCP server. The graph cache and algorithm needs
there are generic enough to live in a standalone crate on `petgraph 0.8`.

The only available alternative (`graphalgs`) is pinned to `petgraph ^0.6.5` and
appears unmaintained (last release 2023). `petgraph-live` targets `petgraph 0.8`
only and has no plans to support older versions.

---

## Roadmap

- [ ] `cache::GenerationCache<G>` with read-write lock, hit/miss semantics
- [ ] `metrics` — diameter, radius, eccentricity, center, periphery (BFS, O(n·(n+e)))
- [ ] `connect` — articulation points, bridges (DFS, O(n+e))
- [ ] `snapshot` — serde-based disk persistence (`petgraph serde-1` feature)
- [ ] Docs and examples

---

## License

MIT OR Apache-2.0
