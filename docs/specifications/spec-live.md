---
title: "live module"
summary: "GraphState<G> — composites GenerationCache and snapshot into a single managed lifecycle with cold/warm start, stale-key rebuild, and snapshot rotation."
read_when:
  - Implementing or modifying GraphState or GraphStateBuilder
  - Understanding init/get/get_fresh/rebuild flows
  - Reasoning about concurrency guarantees in live module
  - Writing tests for the live module
status: pre-implementation
last_updated: "2026-05-02"
---

# Specification: `live` module

**Crate:** `petgraph-live`
**Feature flag:** `snapshot` (live is always included when snapshot is enabled)
**Status:** pre-implementation
**Depends on:** `cache` module, `snapshot` module

---

## Purpose

`GraphState<G>` composes `GenerationCache<G>` and the `snapshot` module into a
single managed lifecycle. The caller supplies two functions: how to compute the
current validity key, and how to build the graph from scratch. `GraphState`
handles everything else: cold start, warm start from snapshot, cache hit on hot
path, rebuild on key change, snapshot save after rebuild, rotation.

---

## Scope

In scope:
- Cold start: no snapshot → build → save
- Warm start: snapshot key matches → load from disk, skip build
- Hot path `get()`: no key check, return cached `Arc<G>`
- Stale-check `get_fresh()`: compare `key_fn()` to `current_key`, rebuild if different
- Forced `rebuild()`: unconditional rebuild + save
- Builder pattern for ergonomic setup
- `SnapshotConfig::key = None` enforced — key management is internal

Out of scope:
- Async rebuild (background thread / tokio task)
- Multiple concurrent rebuild coalescing (v0.2)
- TTL-based expiry

---

## Concurrency model

`build_fn` runs outside any lock. Only the cache-swap step (store new `Arc` +
bump generation in `GenerationCache`) acquires a write lock. Duration of write
lock: microseconds (pointer swap). Concurrent `get()` callers are never blocked
during a rebuild — they continue reading the stale `Arc` until the swap
completes. There is no `serve_stale_during_rebuild` toggle; this behaviour is
structural and always active.

Two concurrent callers both detecting a stale key will both call `build_fn`.
Both results are identical (idempotent build). The last writer wins the
cache-swap. Acceptable for v0.1 — coalescing deferred to v0.2.

---

## Public types

### `GraphStateConfig`

```rust
pub struct GraphStateConfig {
    pub snapshot: SnapshotConfig,
}

impl GraphStateConfig {
    pub fn new(snapshot: SnapshotConfig) -> Self;
}
```

`snapshot.key` must be `None` — `GraphState` manages keys internally.
`GraphStateBuilder::init()` returns `Err` if `snapshot.key` is `Some`.

### `GraphState<G>`

```rust
pub struct GraphState<G> {
    cache:    GenerationCache<G>,
    config:   GraphStateConfig,
    key_fn:   Arc<dyn Fn() -> Result<String, SnapshotError> + Send + Sync>,
    build_fn: Arc<dyn Fn() -> Result<G, SnapshotError> + Send + Sync>,
    inner:    RwLock<GraphStateInner>,
}

struct GraphStateInner {
    current_key: String,
    generation:  u64,
}
```

```rust
impl<G> GraphState<G>
where
    G: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    pub fn builder(config: GraphStateConfig) -> GraphStateBuilder<G>;

    /// Hot path. No key check. Returns Err only if cache is empty (precondition
    /// violation — should not occur after successful init()).
    pub fn get(&self) -> Result<Arc<G>, SnapshotError>;

    /// Check key_fn() against current_key. Rebuild if different.
    pub fn get_fresh(&self) -> Result<Arc<G>, SnapshotError>;

    /// Unconditional rebuild and snapshot save.
    pub fn rebuild(&self) -> Result<Arc<G>, SnapshotError>;

    /// Key of currently cached graph.
    pub fn current_key(&self) -> String;

    /// Process-lifetime generation counter.
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
    pub fn key_fn(
        self,
        f: impl Fn() -> Result<String, SnapshotError> + Send + Sync + 'static,
    ) -> Self;

    pub fn build_fn(
        self,
        f: impl Fn() -> Result<G, SnapshotError> + Send + Sync + 'static,
    ) -> Self;

    /// Provide current key directly — avoids calling key_fn at init.
    pub fn current_key(self, key: impl Into<String>) -> Self;

    /// Consume builder. Returns Err if key_fn or build_fn not set, or if
    /// config.snapshot.key != None.
    pub fn init(self) -> Result<GraphState<G>, SnapshotError>;
}
```

---

## Init flow

```
init():
  1. Validate: key_fn set, build_fn set, snapshot.key == None → else Err
  2. Determine current_key:
       use builder.current_key if provided
       else call key_fn()
  3. Try load(snapshot_cfg with key = Some(current_key))
       Ok(Some(g))   → warm start, build_called = false
       Ok(None) / Err(KeyNotFound) → call build_fn() → g, build_called = true
  4. If build_called: save(snapshot_cfg with key = Some(current_key), &g)
  5. Store g in GenerationCache with generation = 1
  6. Return GraphState { cache, config, key_fn, build_fn,
         inner: RwLock::new(GraphStateInner { current_key, generation: 1 }) }
```

## `get_fresh` flow

```
get_fresh():
  1. new_key = key_fn()
  2. Read-lock inner: if new_key == current_key → return get() [cache hit, no rebuild]
  3. Drop read lock
  4. build_fn() → g  [outside any lock]
  5. save(snapshot_cfg with key = Some(new_key), &g)
  6. Write-lock inner: bump generation, set current_key = new_key
  7. cache.invalidate(); cache.get_or_build(generation, || Ok(g.clone()))
  8. Return Ok(Arc<g>)
```

## `rebuild` flow

```
rebuild():
  1. current_key = key_fn()  [or reuse inner.current_key if key_fn not changed]
  2. build_fn() → g
  3. save(snapshot_cfg with key = Some(current_key), &g)
  4. Write-lock: bump generation
  5. cache.invalidate(); store new Arc
  6. Return Ok(Arc<g>)
```

---

## Files

| Path | Responsibility |
|---|---|
| `src/live/mod.rs` | Re-exports, module-level rustdoc |
| `src/live/config.rs` | `GraphStateConfig` |
| `src/live/state.rs` | `GraphState<G>`, `GraphStateBuilder<G>`, `GraphStateInner` |
| `tests/live.rs` | Integration tests |
| `examples/live_basic.rs` | End-to-end demo |

---

## Test matrix

| Test | Verifies |
|---|---|
| `test_config_new` | Config construction |
| `test_builder_missing_key_fn_errors` | init() without key_fn → Err |
| `test_builder_missing_build_fn_errors` | init() without build_fn → Err |
| `test_builder_key_some_errors` | snapshot.key = Some → init() Err |
| `test_init_cold_start_no_snapshot` | Empty dir → build called, snapshot written |
| `test_init_warm_start_from_snapshot` | Key matches → load, build NOT called |
| `test_init_snapshot_key_mismatch_rebuilds` | Key mismatch → build called |
| `test_get_returns_cached` | Two get() calls → same Arc::ptr_eq |
| `test_current_key_and_generation` | Values correct after init |
| `test_get_fresh_same_key_no_rebuild` | Same key → no rebuild |
| `test_get_fresh_new_key_triggers_rebuild` | New key → rebuild, snapshot written |
| `test_get_fresh_saves_snapshot` | Snapshot file exists after rebuild |
| `test_rebuild_forces_new_graph` | rebuild() → new Arc, different value |
| `test_concurrent_get` | 8 threads × 100 reads, no deadlock |
