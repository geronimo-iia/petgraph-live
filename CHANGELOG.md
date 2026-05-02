# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[0.3.0]: https://github.com/geronimo-iia/petgraph-live/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/geronimo-iia/petgraph-live/releases/tag/v0.2.0
