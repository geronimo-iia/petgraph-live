# petgraph-live — Roadmap

**Current status:** pre-implementation. API designed, plans written, no production code yet.

---

## v0.1.0 — Core

All four modules. No breaking changes expected after this milestone.

### Milestone order (dependency-driven)

```
1. cache        ← no deps, foundation for live
2. algorithms   ← no deps, independent of cache/snapshot (parallel with snapshot)
2. snapshot     ← no deps on cache/algorithms (parallel with algorithms)
3. live         ← depends on cache + snapshot
```

### Deliverables

| Module | Spec | Plan | Status |
|---|---|---|---|
| `cache::GenerationCache<G>` | [spec-cache](specifications/spec-cache.md) | [plan-cache](plan-cache.md) | not started |
| `metrics`, `connect`, `shortest_path`, `mst` | [spec-algorithms](specifications/spec-algorithms.md) | [plan-algorithms](plan-algorithms.md) | not started |
| `snapshot` | [spec-snapshot](specifications/spec-snapshot.md) | [plan-snapshot](plan-snapshot.md) | not started |
| `live::GraphState<G>` | [spec-live](specifications/spec-live.md) | [plan-live](plan-live.md) | not started |

### Definition of done for v0.1.0

- [ ] `cargo test` — zero failures (no features)
- [ ] `cargo test --features snapshot,snapshot-zstd` — zero failures
- [ ] `cargo test --doc` — zero doctest failures
- [ ] `cargo clippy -- -D warnings` — zero warnings
- [ ] `cargo fmt --check` — clean
- [ ] All examples run without panic
- [ ] `docs.rs`-compatible rustdoc on all public items
- [ ] README updated to reflect actual API

---

## v0.2.0 — Extended algorithms

Algorithms deferred from v0.1 (require more complex implementation or design).

| Feature | Notes |
|---|---|
| `elementary_circuits` — Tarjan's circuit enumeration | Needs careful cycle detection, expensive on dense graphs |
| `tournament` module | Tournament-specific algorithms from graphalgs |
| `coloring::dsatur` | DSATUR greedy graph colouring |

---

## v0.3.0 — Live improvements

Operational improvements to `GraphState`.

| Feature | Notes |
|---|---|
| Rebuild coalescing | Two concurrent `get_fresh()` on stale key → one `build_fn` call |
| Background rebuild | `build_fn` in a `std::thread::spawn`; stale graph served until done |
| `GraphState` metrics | Expose hit/miss counters, last rebuild duration |

---

## v0.4.0 — Snapshot improvements

| Feature | Notes |
|---|---|
| `snapshot-lz4` sub-feature | Faster compression than zstd for large graphs |
| Schema versioning helper | `SnapshotMeta::petgraph_live_version` mismatch → caller hook |
| `list` + `inspect` without loading graph body (JSON streaming) | Skip `"graph"` field entirely using streaming deserializer |

---

## Permanently out of scope

| Item | Reason |
|---|---|
| `adj_matrix` / `spec` (nalgebra) | Heavy dep; petgraph already has `to_adjacency_matrix()` |
| `generate` (random graphs) | Pulls `rand`; niche use, not a core concern |
| Async runtime support | No `tokio`/`async-std` — library stays runtime-agnostic |
| petgraph < 0.8 | Not maintained here; `graphalgs` covers older versions |
| Community detection (Louvain, etc.) | Domain-specific; belongs in consumer crate (e.g. `llm-wiki`) |

---

## Consumer

Primary consumer: [`llm-wiki`](https://github.com/geronimo-iia/llm-wiki), a
git-backed wiki engine with MCP server. The specific mapping:

| llm-wiki concept | petgraph-live API |
|---|---|
| `get_or_build_graph` + `CachedGraph` | `cache::GenerationCache<WikiGraph>` |
| graph snapshot across restarts | `snapshot::save` / `snapshot::load` |
| managed cache + snapshot lifecycle | `live::GraphState<WikiGraph>` |
| structural analysis of wiki pages | `connect::articulation_points`, `connect::find_bridges` |
| wiki graph health | `metrics::diameter`, `metrics::center` |
