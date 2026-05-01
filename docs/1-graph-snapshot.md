# imp: Graph snapshot — persist cache across process restarts

**Milestone:** v0.3.x (after v0.3.0 gap fixes)
**Branch:** `feat/graph-snapshot` (from `dev/v0.3.x`)
**Depends on:** `feat/graph-cache` merged + gap fix plan complete

---

## Problem

The in-memory `CachedGraph` introduced in v0.3.0 eliminates redundant
`build_graph` and Louvain calls **within a running process**. It does not
survive process restart. Every cold start (CLI invocation, server restart,
docker container start) must rebuild the full graph + compute communities from
scratch, even when the wiki has not changed since the last run.

At 1 000 pages this is negligible. At 10 000+ pages `build_graph` takes
hundreds of milliseconds and Louvain adds more. In automated pipelines (CI
linting, scheduled ACP research tasks) that restart the process frequently,
this cost accumulates.

---

## Why petgraph serde is the right answer

`petgraph` has a first-party `serde-1` feature that derives
`Serialize/Deserialize` on `DiGraph<N, E>` when `N` and `E` both implement
those traits. `PageNode` already derives both. `LabeledEdge` is missing the
derives — a one-line fix.

No external graph serialization library is needed. No custom adjacency-list
format. The graph round-trips through `serde_json` (already a dependency) with
zero new crates.

---

## Why this is NOT in v0.3.0

The gap fix plan (`2026-04-29-graph-cache-gaps.md`) is already in flight.
Adding snapshot to that branch would:
- Widen scope of an already-complex PR
- Introduce a new dependency mutation (`petgraph` feature flag) that could
  break unrelated compilation units
- Add file I/O to a caching layer that currently has none

The in-memory cache covers 95% of the serve-mode benefit. Snapshot is additive
and can land in a clean follow-on PR.

---

## Proposed solution

### Cache key: `last_commit`

`SpaceIndexManager::last_commit()` returns `Option<String>` (git commit SHA).
This is a stable, cross-process, human-readable key. It is already used to
detect index staleness in `rebuild` and `update` paths.

The snapshot file stores the commit SHA alongside the graph. On load, compare
stored SHA with `last_commit()`. Mismatch → discard snapshot, rebuild.

Using `last_commit` instead of the `AtomicU64` generation counter means the
snapshot is valid across restarts as long as the wiki has not been updated.

### Snapshot location

```
{state_dir}/indexes/{wiki_name}/graph-snapshot.json
```

Co-located with the tantivy index directory. Deleted automatically when
`rebuild` wipes the index directory (existing behaviour). No new cleanup logic
needed.

### Snapshot format

```json
{
  "version": 1,
  "commit":  "abc123...",
  "graph":   { /* petgraph serde-1 serialization of DiGraph */ },
  "community_map":   { "concepts/foo": 0, ... } | null,
  "community_stats": { "count": 3, "isolated": [...] } | null
}
```

`version` field allows forward-compatible format changes without corrupt-read
panics — unknown version → discard and rebuild.

### Write path

After every cache miss in `get_or_build_graph`, write snapshot atomically:
1. Serialize to `graph-snapshot.json.tmp`
2. `rename()` over `graph-snapshot.json`

Atomic rename prevents corrupt reads if the process dies mid-write.

### Read path (engine startup)

In `WikiEngine::build` (or `mount_space`), after index is opened, attempt to
load snapshot:

```
try_load_snapshot(state_dir, wiki_name, last_commit)
  → Ok(Some(CachedGraph)) if commit matches and parse succeeds
  → Ok(None)              if file absent, commit mismatch, or version unknown
  → rebuild on any Err    (log warning, do not fail startup)
```

Populate `graph_cache` with the loaded `CachedGraph` before returning
`SpaceContext`. First graph request is then a cache hit.

---

## What changes

### `Cargo.toml`

```toml
# Before
petgraph = "0.8"

# After
petgraph = { version = "0.8", features = ["serde-1"] }
```

### `src/graph.rs`

Add derives to `LabeledEdge`:
```rust
// Before
pub struct LabeledEdge {
    pub relation: String,
}

// After
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledEdge {
    pub relation: String,
}
```

Add snapshot structs:
```rust
#[derive(Serialize, Deserialize)]
struct GraphSnapshot {
    version:         u32,
    commit:          String,
    graph:           WikiGraph,
    community_map:   Option<HashMap<String, usize>>,
    community_stats: Option<CommunityStats>,
}
```

Add two public functions:
```rust
pub fn save_graph_snapshot(
    snapshot_path: &Path,
    commit: &str,
    cached: &CachedGraph,
) -> Result<()>

pub fn load_graph_snapshot(
    snapshot_path: &Path,
    current_commit: &str,
) -> Result<Option<CachedGraph>>
```

### `src/graph.rs` — `get_or_build_graph`

After populating the in-memory cache on miss, call `save_graph_snapshot` in a
best-effort fire-and-forget pattern (log warning on error, never fail the
caller):

```rust
if let Some(ref commit) = space.last_commit() {
    let path = snapshot_path(state_dir, wiki_name);
    if let Err(e) = save_graph_snapshot(&path, commit, cached) {
        tracing::warn!(error = %e, "failed to write graph snapshot");
    }
}
```

This requires passing `state_dir` and `wiki_name` into `get_or_build_graph`,
or moving snapshot write responsibility to the caller (`ops/graph.rs`, etc.).
Simpler: add a `save_snapshot: Option<&Path>` parameter — `None` in tests,
`Some(path)` in production callers.

### `src/engine.rs` / `space_builder.rs`

In `mount_space`, after `SpaceIndexManager` is opened, attempt snapshot load:

```rust
let graph_cache = if let Some(commit) = index_manager.last_commit() {
    let snap_path = snapshot_path(&state_dir, &entry.name);
    match graph::load_graph_snapshot(&snap_path, &commit) {
        Ok(Some(cached)) => {
            tracing::debug!(wiki = %entry.name, "graph snapshot loaded");
            RwLock::new(Some(cached))
        }
        Ok(None) => RwLock::new(None),
        Err(e) => {
            tracing::warn!(error = %e, "graph snapshot corrupt, will rebuild");
            RwLock::new(None)
        }
    }
} else {
    RwLock::new(None)
};
```

---

## What this does NOT change

- `AtomicU64` generation counter — still used for in-process invalidation
- `reload_reader()` bump — unchanged
- All existing cache tests — unchanged
- Hot-reload / watch path — unchanged (write ops bump generation; next request
  rebuilds and writes new snapshot)

---

## Trade-offs

| Option | Pros | Cons |
|--------|------|------|
| **JSON snapshot via petgraph serde-1** (this doc) | No new deps; human-readable; debuggable; ~trivial format | Larger file than bincode; slower parse for very large graphs |
| bincode snapshot | Smallest file; fastest parse | New dep; binary format (not debuggable); petgraph serde-1 still required anyway |
| Re-build on cold start (current) | Zero complexity | Redundant work on every restart |
| Separate tantivy index for graph | Survives restart | Already rejected in `imp-graph-cache.md` |

JSON is the right choice. For a graph of 50 000 nodes, the JSON file is ~20 MB
and parses in ~100 ms — still faster than a full `build_graph` rebuild.

---

## Risks

- **Corrupt snapshot on power loss mid-write:** Atomic rename mitigates. On
  corrupt read, fall through to rebuild — no user-visible failure.
- **petgraph serde-1 feature flag:** Additive only; does not remove any API.
  Compile time increases slightly. Run a clean `cargo test` to verify no
  breakage before merging.
- **Snapshot grows stale on schema changes:** If `PageNode` fields change
  between versions, deserialization fails with a serde error. The `version`
  field + catch-all `Err` → rebuild path handles this.

---

## Files changed

```
Cargo.toml                — petgraph serde-1 feature
src/graph.rs              — LabeledEdge derives; GraphSnapshot struct; save/load fns; snapshot write in get_or_build_graph
src/engine.rs             — snapshot load attempt in mount_space / space_builder
tests/graph_snapshot.rs   — round-trip test; stale-commit test; corrupt-file test
```

No changes to: `src/ops/`, `src/index_manager.rs`, `src/acp/`.
