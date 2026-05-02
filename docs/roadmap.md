# petgraph-live — Roadmap

**Current status:** v0.2.0 implemented. All four modules done; release pending.


## v0.2.0 — Core

All four modules. No breaking changes expected after this milestone.

### Milestone order (dependency-driven)

```
1. cache        ← no deps, foundation for live
2. algorithms   ← no deps, independent of cache/snapshot (parallel with snapshot)
2. snapshot     ← no deps on cache/algorithms (parallel with algorithms)
3. live         ← depends on cache + snapshot
```

### Deliverables

| Module                                       | Spec                                                 | Status      |
| -------------------------------------------- | ---------------------------------------------------- | ----------- |
| `cache::GenerationCache<G>`                  | [spec-cache](specifications/spec-cache.md)           | implemented |
| `metrics`, `connect`, `shortest_path`, `mst` | [spec-algorithms](specifications/spec-algorithms.md) | implemented |
| `snapshot`                                   | [spec-snapshot](specifications/spec-snapshot.md)     | implemented |
| `live::GraphState<G>`                        | [spec-live](specifications/spec-live.md)             | implemented |

### Definition of done for v0.2.0

- [x] `cargo test` — zero failures (no features)
- [x] `cargo test --features snapshot,snapshot-zstd` — zero failures
- [x] `cargo test --doc` — zero doctest failures
- [x] `cargo clippy -- -D warnings` — zero warnings
- [x] `cargo fmt --check` — clean
- [x] All examples run without panic
- [x] `docs.rs`-compatible rustdoc on all public items
- [x] README updated to reflect actual API


## v0.3.0 — Snapshot improvements

| Feature                                                        | Notes                                                        |
| -------------------------------------------------------------- | ------------------------------------------------------------ |
| `snapshot-lz4` sub-feature — [spec](specifications/spec-snapshot-lz4.md) / [plan](brainstorming/plan-snapshot-lz4.md) | Faster decompression than zstd; pure Rust (`lz4_flex`) | **done** |
| Schema versioning helper                                       | `SnapshotMeta::petgraph_live_version` mismatch → caller hook |
| `list` + `inspect` without graph body — [spec](specifications/spec-snapshot-lazy-meta.md) / [plan](brainstorming/plan-snapshot-lazy-meta.md) | Bincode: partial file read; JSON: `MetaOnly` serde skip | **done** |


## v0.4.0 — Extended algorithms

Algorithms deferred from v0.2 (require more complex implementation or design).

| Feature                                              | Notes                                                    |
| ---------------------------------------------------- | -------------------------------------------------------- |
| `elementary_circuits` — Tarjan's circuit enumeration | Needs careful cycle detection, expensive on dense graphs |
| `tournament` module                                  | Tournament-specific algorithms from graphalgs            |
| `coloring::dsatur`                                   | DSATUR greedy graph colouring                            |


## Permanently out of scope

| Item                                | Reason                                                        |
| ----------------------------------- | ------------------------------------------------------------- |
| `adj_matrix` / `spec` (nalgebra)    | Heavy dep; petgraph already has `to_adjacency_matrix()`       |
| `generate` (random graphs)          | Pulls `rand`; niche use, not a core concern                   |
| Async runtime support               | No `tokio`/`async-std` — library stays runtime-agnostic       |
| petgraph < 0.8                      | Not maintained here; `graphalgs` covers older versions        |
| Community detection (Louvain, etc.) | Domain-specific; belongs in consumer crate (e.g. `llm-wiki`)  |
| Live operational improvements       | Rebuild coalescing, background rebuild — caller responsibility |


## Consumer

Primary consumer: [`llm-wiki`](https://github.com/geronimo-iia/llm-wiki), a
git-backed wiki engine with MCP server. The specific mapping:

| llm-wiki concept                     | petgraph-live API                                       |
| ------------------------------------ | ------------------------------------------------------- |
| `get_or_build_graph` + `CachedGraph` | `cache::GenerationCache<WikiGraph>`                     |
| graph snapshot across restarts       | `snapshot::save` / `snapshot::load`                     |
| managed cache + snapshot lifecycle   | `live::GraphState<WikiGraph>`                           |
| structural analysis of wiki pages    | `connect::articulation_points`, `connect::find_bridges` |
| wiki graph health                    | `metrics::diameter`, `metrics::center`                  |
