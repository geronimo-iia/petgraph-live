# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0]

### Breaking

- Replaced `bincode` with `postcard` for binary snapshot serialization.
  `bincode-org/bincode` was archived 2025-08-15; `bincode 2.0.1` is its final release.
- Snapshot wire format v2: `.snap` files now start with magic bytes `b"PGL\x02"`.
  Existing `.snap` / `.snap.zst` / `.snap.lz4` files written by v0.4.x are not readable.
- `load()` returns `Err(SnapshotError::LegacyFormat { path })` for old files.
- `load_or_build()` transparently rebuilds on legacy files — no action needed for most users.
- Users calling `load()` directly: delete old snapshot files before upgrading.

## [0.4.0] — 2026-06-29

### Added
- `hebbian` module — SOKM (Self-Organizing Knowledge Map) algorithm: `decay`, `strengthen`, `prune`, `sokm_tick`; works on both directed and undirected `StableGraph<N, f64, Ty>`; config types `SokmConfig`, `StrengthFormula`, `HebbianReport`; example `hebbian_sokm`; criterion benchmark
- `hebbian::stdp_update` — Spike-Timing Dependent Plasticity: causal pairs strengthened, anti-causal weakened, with exponential time decay
- `hebbian::anti_hebbian_update` — lateral inhibition: weakens edges between co-activated nodes (competitive learning)
- `hebbian::oja_update` — Oja's normalized Hebbian rule: self-converging weights toward principal component
- `hebbian::bcm_update` — BCM homeostatic plasticity: sliding threshold prevents runaway strengthening

## [Unreleased]

## [0.3.1] — 2026-05-03

### Fixed

- `snapshot::io::save()` now calls `std::fs::create_dir_all` before writing, so it no longer returns `Err(NotFound)` when `SnapshotConfig.dir` does not exist. Callers no longer need to pre-create the directory. (fixes #7)
- `snapshot::rotation::list_snapshot_files()` returns `Ok(vec![])` instead of `Err(NotFound)` when the snapshot directory is absent. (fixes #7)

## [0.3.0] — 2026-05-02

### Added
- `snapshot-lz4` feature — LZ4 compression via `lz4_flex` (pure Rust); `Compression::Lz4` variant; files: `.snap.lz4` / `.json.lz4`
- `inspect()` and `list()` no longer read graph bytes for uncompressed bincode files; JSON path skips `G::deserialize` via `MetaOnly` serde helper

## [0.2.0] — 2026-05-02

### Added
- `cache::GenerationCache<G>` — thread-safe generation-keyed graph cache with
  `get_or_build`, `invalidate`, `current_generation`; integration tests and
  `examples/cache_basic` included
- `metrics` — unweighted (BFS) and weighted (Floyd-Warshall) graph metrics:
  `eccentricity`, `radius`, `diameter`, `center`, `periphery`, `girth`;
  ported from graphalgs (MIT) with deviations documented in spec
- `connect` — articulation points and bridges (Tarjan DFS); undirected graphs
- `shortest_path` — `shortest_distances` (BFS), `floyd_warshall`,
  `distance_map`, `seidel` (unweighted APSP); re-exports `dijkstra`,
  `bellman_ford`, `astar`, `spfa`, `johnson`, `k_shortest_path`,
  `NegativeCycle` from `petgraph::algo`
- `mst` — `prim` and `boruvka` returning `Vec<(G::NodeId, G::NodeId)>`;
  re-exports `min_spanning_tree` as `kruskal` from petgraph
- `snapshot` (feature `snapshot`) — key-as-filename disk persistence: atomic
  write, mtime rotation, bincode and JSON formats, optional zstd compression
  (feature `snapshot-zstd`); `save`, `load`, `load_or_build`, `inspect`,
  `list`, `purge`
- `live::GraphState<G>` (feature `snapshot`) — composites `GenerationCache`
  and snapshot into a managed lifecycle: cold start, warm start from snapshot,
  stale-key rebuild via `get_fresh`, forced `rebuild`, snapshot rotation;
  builder API; integration tests and `examples/live_basic`
- `SECURITY.md`, `docs/release.md`, `docs/specifications/` index,
  `docs/roadmap.md`, `docs/api-design.md`

[0.4.0]: https://github.com/geronimo-iia/petgraph-live/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/geronimo-iia/petgraph-live/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/geronimo-iia/petgraph-live/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/geronimo-iia/petgraph-live/releases/tag/v0.2.0
