# petgraph-live — Plan: Live

**Goal:** Implement `live::GraphState<G>` — composing `GenerationCache<G>` with snapshot lifecycle.
**Feature flag:** `snapshot` (same flag — `live` module is always included when `snapshot` is enabled)
**Dependencies:** `snapshot` feature (see plan-snapshot.md); no new deps.

`GraphState<G>` provides:
- Cold-start from snapshot (if key matches) or fresh build
- Hot-path `get()` — no key check, zero overhead
- Stale-check `get_fresh()` — calls `key_fn`, rebuilds only if key changed
- Snapshot save after every rebuild, with rotation
- Explicit `rebuild()` for forced refresh
- Builder pattern for ergonomic setup

---

## File structure

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/live/mod.rs` | Re-exports, module doc |
| Create | `src/live/config.rs` | `GraphStateConfig`, `GraphStateConfigBuilder` |
| Create | `src/live/state.rs` | `GraphState<G>`, `GraphStateBuilder<G>` |
| Modify | `src/lib.rs` | Add `#[cfg(feature = "snapshot")] pub mod live;` |
| Create | `examples/live_basic.rs` | End-to-end demo |
| Create | `tests/live.rs` | Integration tests |

---

## Design notes

### Key flow

```
init():
  key_fn() → current_key
  load(cfg with key=Some(current_key))
    Ok(Some(g)) → cache.store(g, gen=1), state.current_key = current_key
    Ok(None)/KeyNotFound → build_fn() → save → cache.store

get():
  cache.get_or_build(current_gen, || Err) → Ok(arc) [never rebuilds — gen unchanged]

get_fresh():
  key_fn() → new_key
  if new_key == current_key: return cache hit
  else: build_fn() → save with new_key → cache invalidate + store → current_key = new_key

rebuild():
  key_fn() → current_key  (or reuse if already known)
  build_fn() → save → cache invalidate + store
```

### `SnapshotConfig::key` enforcement

`GraphStateBuilder::init()` verifies `cfg.snapshot.key == None` before initialising.
`GraphState` manages the key internally — it sets `cfg.snapshot.key = Some(current_key)` when calling `save`/`load`.

### Concurrency

`GraphState<G>` wraps internal mutable state in `RwLock`. Multiple callers may call `get()` concurrently. `get_fresh()` and `rebuild()` acquire a write lock for the rebuild phase; while rebuilding, if `serve_stale_during_rebuild = true`, stale readers on the read lock are unblocked immediately after the new graph is stored.

---

## Tasks

### Task 1 — Scaffold (sonnet)

- [ ] Add `#[cfg(feature = "snapshot")] pub mod live;` to `src/lib.rs`.
- [ ] Create stub files for `src/live/mod.rs`, `src/live/config.rs`, `src/live/state.rs`.
- [ ] Run `cargo build --features snapshot` — must compile.
- [ ] Commit: `feat(live): scaffold live module files`

---

### Task 2 — `GraphStateConfig` (sonnet)

- [ ] Write failing test in `tests/live.rs`:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_config_new() {
      use petgraph_live::{live::GraphStateConfig, snapshot::{Compression, SnapshotConfig, SnapshotFormat}};
      use std::path::PathBuf;
      let snap = SnapshotConfig {
          dir: PathBuf::from("/tmp"), name: "g".into(), key: None,
          format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
      };
      let cfg = GraphStateConfig::new(snap);
      assert_eq!(cfg.snapshot.name, "g");
  }
  ```
- [ ] Run `cargo test --features snapshot --test live test_config_new` — compile error expected.
- [ ] Implement `src/live/config.rs`:
  ```rust
  use crate::snapshot::SnapshotConfig;

  pub struct GraphStateConfig {
      pub snapshot: SnapshotConfig,
  }

  impl GraphStateConfig {
      pub fn new(snapshot: SnapshotConfig) -> Self {
          GraphStateConfig { snapshot }
      }
  }
  ```
  Concurrency note: `build_fn` runs outside any lock. Write lock covers only the cache-swap
  (store new Arc + bump generation). Concurrent `get()` readers hold the read lock during that
  swap at most for microseconds — no long blocking.
- [ ] Re-export from `src/live/mod.rs`: `pub use config::GraphStateConfig;`
- [ ] Run `cargo test --features snapshot --test live test_config_new` — pass.
- [ ] Commit: `feat(live): add GraphStateConfig`

---

### Task 3 — `GraphStateBuilder<G>` API (sonnet)

- [ ] Write failing tests:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_builder_missing_key_fn_errors() {
      // init() without key_fn → Err
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_builder_missing_build_fn_errors() {
      // init() with key_fn but no build_fn → Err
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_builder_key_some_errors() {
      // SnapshotConfig::key = Some("k") → init() → Err (key must be None for GraphState)
  }
  ```
- [ ] Run `cargo test --features snapshot --test live` — compile error expected.
- [ ] Implement `GraphStateBuilder<G>` skeleton in `src/live/state.rs`:
  ```rust
  use std::sync::Arc;
  use crate::snapshot::SnapshotError;
  use crate::live::GraphStateConfig;

  pub struct GraphStateBuilder<G> {
      config:      GraphStateConfig,
      key_fn:      Option<Box<dyn Fn() -> Result<String, SnapshotError> + Send + Sync>>,
      build_fn:    Option<Box<dyn Fn() -> Result<G, SnapshotError> + Send + Sync>>,
      current_key: Option<String>,
  }

  impl<G> GraphStateBuilder<G>
  where
      G: serde::Serialize + serde::de::DeserializeOwned
          + petgraph::visit::NodeCount + petgraph::visit::EdgeCount
          + Send + Sync + 'static,
  {
      pub fn key_fn(
          mut self,
          f: impl Fn() -> Result<String, SnapshotError> + Send + Sync + 'static,
      ) -> Self { self.key_fn = Some(Box::new(f)); self }

      pub fn build_fn(
          mut self,
          f: impl Fn() -> Result<G, SnapshotError> + Send + Sync + 'static,
      ) -> Self { self.build_fn = Some(Box::new(f)); self }

      pub fn current_key(mut self, key: impl Into<String>) -> Self {
          self.current_key = Some(key.into()); self
      }

      pub fn init(self) -> Result<GraphState<G>, SnapshotError> { todo!() }
  }
  ```
- [ ] Run `cargo test --features snapshot --test live` — tests fail (not panic), expected.
- [ ] Commit: `feat(live): add GraphStateBuilder skeleton`

---

### Task 4 — `GraphState<G>` struct and `init()` (sonnet)

- [ ] Write failing tests:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_init_cold_start_no_snapshot() {
      // empty dir, build_fn returns 5-node graph
      // init() → Ok, get() returns graph with 5 nodes
      // snapshot file created in dir
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_init_warm_start_from_snapshot() {
      // save 3-node graph with key "v1"
      // init() with key_fn=|| Ok("v1") → loads from snapshot, build_fn NOT called
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_init_snapshot_key_mismatch_rebuilds() {
      // save 3-node graph with key "v1"
      // init() with key_fn=|| Ok("v2") → KeyNotFound → build_fn called
  }
  ```
- [ ] Implement `GraphState<G>` struct:
  ```rust
  use std::sync::{Arc, RwLock};
  use crate::cache::GenerationCache;

  pub struct GraphState<G> {
      cache:       GenerationCache<G>,
      config:      GraphStateConfig,
      key_fn:      Arc<dyn Fn() -> Result<String, SnapshotError> + Send + Sync>,
      build_fn:    Arc<dyn Fn() -> Result<G, SnapshotError> + Send + Sync>,
      inner:       RwLock<GraphStateInner>,
  }

  struct GraphStateInner {
      current_key: String,
      generation:  u64,
  }
  ```
- [ ] Implement `init()` in `GraphStateBuilder`:
  1. Validate `key_fn` and `build_fn` are set; error if not.
  2. Validate `config.snapshot.key == None`; error if not.
  3. Determine `current_key`: use `self.current_key` if provided, else call `key_fn()`.
  4. Try `load` with `cfg.snapshot.key = Some(current_key.clone())`.
     - `Ok(Some(g))` → use `g`, `build_called = false`.
     - `Err(KeyNotFound)` / `Ok(None)` → call `build_fn()` → `g`, `build_called = true`.
  5. If `build_called`: save with `cfg.snapshot.key = Some(current_key.clone())`.
  6. Store `g` in `GenerationCache` with `gen = 1`.
  7. Return `GraphState { cache, config, key_fn, build_fn, inner: RwLock::new(GraphStateInner { current_key, generation: 1 }) }`.
- [ ] Run `cargo test --features snapshot --test live` — three new tests must pass.
- [ ] Commit: `feat(live): implement GraphState and init with cold/warm start`

---

### Task 5 — `get()` and `current_key()` / `generation()` (sonnet)

- [ ] Write failing tests:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_get_returns_cached() {
      // init, get twice → same Arc::ptr_eq
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_current_key_and_generation() {
      // init with key_fn=|| Ok("sha1") → current_key() == "sha1", generation() == 1
  }
  ```
- [ ] Implement `get()`:
  ```rust
  pub fn get(&self) -> Result<Arc<G>, SnapshotError> {
      let gen = self.inner.read().unwrap().generation;
      self.cache.get_or_build(gen, || Err(SnapshotError::NoSnapshotFound))
  }
  ```
  Note: `get_or_build` never calls the build fn on a hit; `Err` fallback is unreachable on a warm cache.
- [ ] Implement `current_key()` and `generation()` as read-lock readers on `inner`.
- [ ] Run `cargo test --features snapshot --test live` — pass.
- [ ] Commit: `feat(live): implement get, current_key, generation`

---

### Task 6 — `get_fresh()` (sonnet)

- [ ] Write failing tests:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_get_fresh_same_key_no_rebuild() {
      let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
      // key_fn always returns "v1"
      // build_fn increments call_count
      // init() → call_count == 1
      // get_fresh() → call_count still 1 (same key)
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_get_fresh_new_key_triggers_rebuild() {
      // key_fn returns "v1" on first call, "v2" on second
      // init() → call_count == 1
      // get_fresh() → call_count == 2, current_key() == "v2"
      // snapshot for "v2" exists in dir
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_get_fresh_saves_snapshot() {
      // get_fresh with new key → saves {name}-{new_key}.snap to dir
  }
  ```
- [ ] Implement `get_fresh()`:
  1. Call `key_fn()` → `new_key`.
  2. Acquire read lock, check `current_key`. If equal → return `get()`.
  3. Drop read lock. Call `build_fn()` → `g`.
  4. Save: set `cfg.snapshot.key = Some(new_key.clone())`, call `save(&cfg, &g)`.
  5. Acquire write lock: bump `generation`, set `current_key = new_key`.
  6. Invalidate cache and store new graph in `GenerationCache`.
  7. Return `Ok(Arc::clone(&g))`.
- [ ] Run `cargo test --features snapshot --test live` — pass.
- [ ] Commit: `feat(live): implement get_fresh with stale-key rebuild`

---

### Task 7 — `rebuild()` (sonnet)

- [ ] Write failing test:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_rebuild_forces_new_graph() {
      let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
      // build_fn returns graph with counter.fetch_add(1) nodes
      // init() → counter == 1
      // g1 = get()
      // rebuild()
      // g2 = get()
      // counter == 2, !Arc::ptr_eq(g1, g2)
  }
  ```
- [ ] Implement `rebuild()`:
  1. Call `key_fn()` → `current_key` (use existing if `key_fn` is same).
  2. Call `build_fn()` → `g`.
  3. Save with current key.
  4. Write-lock: bump generation.
  5. Invalidate cache, store new graph.
  6. Return `Ok(Arc)`.
- [ ] Run `cargo test --features snapshot --test live` — pass.
- [ ] Commit: `feat(live): implement rebuild`

---

### Task 8 — Concurrent access test (sonnet)

- [ ] Write:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_concurrent_get() {
      use std::sync::Arc as StdArc;
      use std::thread;
      let dir = tempfile::tempdir().unwrap();
      // ... init state with 1024-node graph
      let state = StdArc::new(/* ... */);
      let handles: Vec<_> = (0..8).map(|_| {
          let s = StdArc::clone(&state);
          thread::spawn(move || {
              for _ in 0..100 {
                  let g = s.get().unwrap();
                  assert_eq!(g.node_count(), 1024);
              }
          })
      }).collect();
      for h in handles { h.join().unwrap(); }
  }
  ```
- [ ] Run — pass (no deadlock, no data race).
- [ ] Commit: `test(live): add concurrent get stress test`

---

### Task 9 — Example `examples/live_basic.rs` (sonnet)

- [ ] Create `examples/live_basic.rs`:
  ```rust
  //! Basic usage of GraphState.
  //! Run with: cargo run --example live_basic --features snapshot
  use petgraph::Graph;
  use petgraph_live::{
      live::GraphStateConfig,
      snapshot::{Compression, SnapshotConfig, SnapshotFormat},
      live::GraphState,
  };
  use std::path::PathBuf;
  use std::sync::atomic::{AtomicU32, Ordering};
  use std::sync::Arc;

  fn main() {
      let dir = tempfile::tempdir().unwrap();
      let version = Arc::new(AtomicU32::new(1));

      let snap = SnapshotConfig {
          dir: dir.path().to_path_buf(), name: "graph".into(), key: None,
          format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
      };
      let config = GraphStateConfig::new(snap);

      let v = Arc::clone(&version);
      let v2 = Arc::clone(&version);
      let state: GraphState<Graph<u32, ()>> = GraphState::builder(config)
          .key_fn(move || Ok(v.load(Ordering::SeqCst).to_string()))
          .build_fn(move || {
              let ver = v2.load(Ordering::SeqCst);
              let mut g = Graph::new();
              for i in 0..ver { g.add_node(i); }
              Ok(g)
          })
          .init()
          .unwrap();

      let g1 = state.get().unwrap();
      println!("init: {} nodes, key={}", g1.node_count(), state.current_key());

      // Same key → cached
      let g2 = state.get_fresh().unwrap();
      println!("get_fresh (same key): same Arc? {}", Arc::ptr_eq(&g1, &g2));

      // Bump version → get_fresh rebuilds
      version.store(5, Ordering::SeqCst);
      let g3 = state.get_fresh().unwrap();
      println!("get_fresh (new key): {} nodes, key={}", g3.node_count(), state.current_key());

      // Force rebuild
      version.store(10, Ordering::SeqCst);
      let g4 = state.rebuild().unwrap();
      println!("rebuild: {} nodes, key={}", g4.node_count(), state.current_key());
  }
  ```
- [ ] Run `cargo run --example live_basic --features snapshot` — no panics.
- [ ] Commit: `docs(live): add live_basic example`

---

### Task 10 — Rustdoc (sonnet)

- [ ] Add full module-level rustdoc to `src/live/mod.rs` with the end-to-end example from `docs/api-design.md`.
- [ ] Add `# Examples` doctests on `get()`, `get_fresh()`, `rebuild()`.
- [ ] Add rustdoc to `GraphStateConfig`, `GraphState`, `GraphStateBuilder`.
- [ ] Run `cargo test --doc --features snapshot` — all doctests pass.
- [ ] Commit: `docs(live): add rustdoc and doctests`

---

### Task 11 — Final check (sonnet)

- [ ] Run `cargo test --features snapshot` — zero failures.
- [ ] Run `cargo test` (no features) — zero failures.
- [ ] Run `cargo clippy --features snapshot -- -D warnings` — zero warnings.
- [ ] Run `cargo fmt --check` — clean.
- [ ] Commit: `chore(live): clippy and fmt clean`
