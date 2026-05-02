# Specifications

Per-module design specifications for `petgraph-live`.

Each document covers: purpose, public API, implementation constraints, and test matrix.

| Spec                                  | Module                                       |
| ------------------------------------- | -------------------------------------------- |
| [spec-cache](spec-cache.md)           | `cache::GenerationCache<G>`                  |
| [spec-algorithms](spec-algorithms.md) | `metrics`, `connect`, `shortest_path`, `mst` |
| [spec-snapshot](spec-snapshot.md)     | `snapshot` (feature-gated)                   |
| [spec-live](spec-live.md)             | `live::GraphState<G>` (feature-gated)        |

## Feature flags

| Flag            | Modules unlocked                                         |
| --------------- | -------------------------------------------------------- |
| _(default)_     | `cache`, `metrics`, `connect`, `shortest_path`, `mst`    |
| `snapshot`      | `snapshot`, `live`                                       |
| `snapshot-zstd` | adds zstd compression to `snapshot` (implies `snapshot`) |
