# petgraph-live — Plan: Snapshot

**Goal:** Implement serde-based disk persistence for any petgraph graph type, feature-gated behind `snapshot`.
**Feature flag:** `snapshot` (with sub-feature `snapshot-zstd`)
**Dependencies:**

```toml
# Cargo.toml additions

[features]
default = []
snapshot = [
    "dep:serde",
    "dep:serde_json",
    "dep:bincode",
    "dep:thiserror",
    "petgraph/serde-1",
]
snapshot-zstd = ["snapshot", "dep:zstd"]

[dependencies]
petgraph = { version = "0.8", features = [] }  # serde-1 added by feature

serde      = { version = "1",    features = ["derive"], optional = true }
serde_json = { version = "1",    optional = true }
bincode    = { version = "2",    optional = true }
thiserror  = { version = "2",    optional = true }
zstd       = { version = "0.13", optional = true }

[dev-dependencies]
tempfile = "3"
```

---

## File structure

| Action | Path | Responsibility |
|---|---|---|
| Modify | `Cargo.toml` | Add `snapshot`, `snapshot-zstd` features and optional deps |
| Create | `src/snapshot/mod.rs` | `#[cfg(feature = "snapshot")]` re-exports of all public items |
| Create | `src/snapshot/config.rs` | `SnapshotConfig`, `SnapshotFormat`, `Compression`, `sanitize_key` |
| Create | `src/snapshot/meta.rs` | `SnapshotMeta` with serde derives |
| Create | `src/snapshot/error.rs` | `SnapshotError` enum with `thiserror` |
| Create | `src/snapshot/io.rs` | `save`, `load`, `load_or_build`, `inspect`, `list`, `purge` |
| Create | `src/snapshot/rotation.rs` | `keep_n` helper: keep latest N by mtime, delete rest |
| Modify | `src/lib.rs` | Add `#[cfg(feature = "snapshot")] pub mod snapshot;` |
| Create | `examples/snapshot_basic.rs` | End-to-end demo |
| Create | `tests/snapshot.rs` | Integration tests (all gated on `#[cfg(feature = "snapshot")]`) |

---

## File naming convention

Key is encoded in the filename — no key stored inside the file body for lookup purposes.

```
{name}-{sanitized_key}.snap        (bincode, uncompressed)
{name}-{sanitized_key}.snap.zst    (bincode, zstd compressed)
{name}-{sanitized_key}.json        (json, uncompressed)
{name}-{sanitized_key}.json.zst    (json, zstd compressed)
```

`sanitize_key(k)`: replace any char outside `[a-zA-Z0-9_.-]` with `_`. Returns error if result is empty.

Two saves with the same key = same filename = idempotent overwrite.

Rotation keeps the latest `keep` files by **filesystem mtime** (not by filename order) — because key strings have no inherent ordering.

Bincode file layout: `meta_len (u64 LE) ++ meta_bytes (bincode) ++ graph_bytes (bincode)`. Length prefix allows `inspect` to read only the meta without deserializing the full graph.

JSON file layout: `{"meta": <SnapshotMeta>, "graph": <G>}` — `inspect` extracts only the `"meta"` field.

Atomic write: write to `{final_path}.tmp`, then `std::fs::rename` to `{final_path}`.

---

## Tasks

### Task 1 — Cargo.toml features and deps (sonnet)

- [ ] Edit `Cargo.toml` to add the exact stanza shown above under **Dependencies**.
- [ ] Add `#[cfg(feature = "snapshot")] pub mod snapshot;` to `src/lib.rs`.
- [ ] Create empty stub files for all six `src/snapshot/*.rs` files.
- [ ] Run `cargo build --features snapshot` — must compile (empty modules, no errors).
- [ ] Run `cargo build` (no features) — must still compile.
- [ ] Commit: `feat(snapshot): add Cargo.toml feature flags and module stubs`

---

### Task 2 — `SnapshotFormat`, `Compression`, `SnapshotConfig`, `sanitize_key` (sonnet)

- [ ] Write failing test in `tests/snapshot.rs`:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_config_defaults() {
      use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};
      use std::path::PathBuf;
      let cfg = SnapshotConfig {
          dir: PathBuf::from("/tmp/test-snapshots"),
          name: "mygraph".to_string(),
          key: Some("abc123".to_string()),
          format: SnapshotFormat::Bincode,
          compression: Compression::None,
          keep: 3,
      };
      assert_eq!(cfg.keep, 3);
      assert_eq!(cfg.key.as_deref(), Some("abc123"));
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_sanitize_key() {
      use petgraph_live::snapshot::sanitize_key;
      assert_eq!(sanitize_key("abc123"), Ok("abc123".to_string()));
      assert_eq!(sanitize_key("a/b c"), Ok("a_b_c".to_string()));
      assert!(sanitize_key("   ").is_err());  // all spaces → InvalidKey
  }
  ```
- [ ] Run `cargo test --features snapshot --test snapshot test_config_defaults` — compile error expected.
- [ ] Implement `src/snapshot/config.rs`:
  ```rust
  use std::path::PathBuf;
  use serde::{Deserialize, Serialize};
  use crate::snapshot::error::SnapshotError;

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub enum SnapshotFormat { Bincode, Json }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub enum Compression {
      None,
      #[cfg(feature = "snapshot-zstd")]
      Zstd { level: i32 },
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct SnapshotConfig {
      pub dir:         PathBuf,
      pub name:        String,
      /// Validity key encoded in the snapshot filename.
      ///
      /// `Some(k)` → load the file `{name}-{sanitize(k)}.{ext}`.
      /// Returns `KeyNotFound` if no such file exists.
      ///
      /// `None` → load the most recent file regardless of key.
      /// Used by `GraphState` which manages keys internally.
      ///
      /// Always `None` in static config files — this is a runtime value
      /// (e.g. current git SHA). Set programmatically before calling `save`/`load`.
      ///
      /// The key is opaque:
      ///
      /// | Source | Key to use |
      /// |---|---|
      /// | Git-backed data | current commit SHA |
      /// | Index generation counter | `generation.to_string()` |
      /// | File/directory content | SHA256 hex |
      /// | Static graph | any fixed constant |
      #[serde(skip)]
      pub key:         Option<String>,
      pub format:      SnapshotFormat,
      pub compression: Compression,
      /// Number of snapshots to retain (by mtime). Oldest deleted on save. Default: 3.
      pub keep:        usize,
  }

  /// Replace any char outside `[a-zA-Z0-9_.-]` with `_`.
  /// Returns `Err(InvalidKey)` if the result is empty.
  pub fn sanitize_key(key: &str) -> Result<String, SnapshotError> {
      let s: String = key.chars()
          .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' { c } else { '_' })
          .collect();
      if s.trim_matches('_').is_empty() {
          Err(SnapshotError::InvalidKey(key.to_string()))
      } else {
          Ok(s)
      }
  }
  ```
- [ ] Add serde roundtrip test:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_config_serde_roundtrip() {
      use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};
      use std::path::PathBuf;
      let cfg = SnapshotConfig {
          dir: PathBuf::from("/tmp"),
          name: "g".into(),
          key: Some("should-be-skipped".into()),
          format: SnapshotFormat::Bincode,
          compression: Compression::None,
          keep: 5,
      };
      let json = serde_json::to_string(&cfg).unwrap();
      let back: SnapshotConfig = serde_json::from_str(&json).unwrap();
      assert_eq!(back.name, "g");
      assert_eq!(back.keep, 5);
      assert_eq!(back.key, None);  // skipped — always None after deserialization
  }
  ```
- [ ] Re-export from `src/snapshot/mod.rs`: `pub use config::{sanitize_key, Compression, SnapshotConfig, SnapshotFormat};`
- [ ] Run `cargo test --features snapshot --test snapshot test_config_defaults` — pass.
- [ ] Commit: `feat(snapshot): add SnapshotConfig, SnapshotFormat, Compression, sanitize_key`

---

### Task 3 — `SnapshotMeta` (sonnet)

- [ ] Implement `src/snapshot/meta.rs`:
  ```rust
  use serde::{Deserialize, Serialize};
  use crate::snapshot::config::{Compression, SnapshotFormat};

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub struct SnapshotMeta {
      pub key:                    String,   // redundant with filename, for convenience
      pub format:                 SnapshotFormat,
      pub compression:            Compression,
      pub node_count:             usize,
      pub edge_count:             usize,
      pub created_at:             u64,      // Unix timestamp seconds
      pub petgraph_live_version:  String,
  }

  impl SnapshotMeta {
      pub fn new(
          key: &str,
          format: SnapshotFormat,
          compression: Compression,
          node_count: usize,
          edge_count: usize,
      ) -> Self {
          use std::time::{SystemTime, UNIX_EPOCH};
          Self {
              key: key.to_string(),
              format,
              compression,
              node_count,
              edge_count,
              created_at: SystemTime::now()
                  .duration_since(UNIX_EPOCH).unwrap().as_secs(),
              petgraph_live_version: env!("CARGO_PKG_VERSION").to_string(),
          }
      }
  }
  ```
- [ ] Re-export from `src/snapshot/mod.rs`: `pub use meta::SnapshotMeta;`
- [ ] Write test:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_meta_new() {
      use petgraph_live::snapshot::{Compression, SnapshotFormat, SnapshotMeta};
      let meta = SnapshotMeta::new("sha123", SnapshotFormat::Bincode, Compression::None, 10, 5);
      assert_eq!(meta.node_count, 10);
      assert_eq!(meta.edge_count, 5);
      assert_eq!(meta.key, "sha123");
      assert!(!meta.petgraph_live_version.is_empty());
  }
  ```
- [ ] Run `cargo test --features snapshot --test snapshot test_meta_new` — pass.
- [ ] Commit: `feat(snapshot): add SnapshotMeta with serde derives`

---

### Task 4 — `SnapshotError` (sonnet)

- [ ] Implement `src/snapshot/error.rs`:
  ```rust
  use thiserror::Error;

  #[derive(Debug, Error)]
  pub enum SnapshotError {
      #[error("I/O error: {0}")]
      Io(#[from] std::io::Error),
      #[error("key not found: {key:?}")]
      KeyNotFound { key: String },
      #[error("invalid key: {0:?}")]
      InvalidKey(String),
      #[error("parse error: {0}")]
      ParseError(String),
      #[error("compression error: {0}")]
      CompressionError(String),
      #[error("no snapshot found")]
      NoSnapshotFound,
  }
  ```
- [ ] Re-export from `src/snapshot/mod.rs`: `pub use error::SnapshotError;`
- [ ] Write test:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_error_display() {
      use petgraph_live::snapshot::SnapshotError;
      let e = SnapshotError::KeyNotFound { key: "sha_abc".into() };
      assert!(e.to_string().contains("sha_abc"));
      let e2 = SnapshotError::InvalidKey("   ".into());
      assert!(e2.to_string().contains("invalid key"));
      assert!(SnapshotError::NoSnapshotFound.to_string().contains("no snapshot"));
  }
  ```
- [ ] Run `cargo test --features snapshot --test snapshot test_error_display` — pass.
- [ ] Commit: `feat(snapshot): add SnapshotError with thiserror`

---

### Task 5 — `rotation.rs`: keep N snapshots (sonnet)

Rotation is by **filesystem mtime** (not filename order) since key strings have no chronological ordering.

- [ ] Write failing test:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_rotation_keep_3() {
      use petgraph_live::snapshot::rotation::{keep_n, list_snapshot_files};
      use std::{fs, time::{SystemTime, Duration}};
      let dir = tempfile::tempdir().unwrap();
      // Write 5 files with distinct mtimes
      for i in 1u64..=5 {
          let fname = format!("mygraph-key{}.snap", i);
          let path = dir.path().join(&fname);
          fs::write(&path, b"data").unwrap();
          // Set mtime ascending: file1 oldest, file5 newest
          let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(i * 1000);
          filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(mtime)).unwrap();
      }
      let files = list_snapshot_files(dir.path(), "mygraph").unwrap();
      assert_eq!(files.len(), 5);
      keep_n(dir.path(), "mygraph", 3).unwrap();
      let remaining = list_snapshot_files(dir.path(), "mygraph").unwrap();
      assert_eq!(remaining.len(), 3);
      // key3, key4, key5 kept (newest by mtime)
      for i in 3u64..=5 {
          assert!(dir.path().join(format!("mygraph-key{}.snap", i)).exists());
      }
  }
  ```
  Note: add `filetime = "0.2"` to `[dev-dependencies]` for mtime control in tests.
- [ ] Run `cargo test --features snapshot --test snapshot test_rotation_keep_3` — compile error expected.
- [ ] Implement `src/snapshot/rotation.rs`:
  - `pub fn list_snapshot_files(dir: &Path, name: &str) -> Result<Vec<PathBuf>, SnapshotError>`:
    - Read `dir`, filter files matching `{name}-*.snap`, `{name}-*.snap.zst`, `{name}-*.json`, `{name}-*.json.zst`.
    - Sort ascending by mtime (oldest first).
  - `pub fn keep_n(dir: &Path, name: &str, n: usize) -> Result<(), SnapshotError>`:
    - Call `list_snapshot_files` (ascending mtime). Delete all but the last `n` entries.
- [ ] Run `cargo test --features snapshot --test snapshot test_rotation_keep_3` — pass.
- [ ] Commit: `feat(snapshot): add rotation keep_n and list_snapshot_files (mtime-based)`

---

### Task 6 — `io.rs`: `save` (sonnet)

- [ ] Write failing test:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_save_creates_file() {
      use petgraph::Graph;
      use petgraph_live::snapshot::{save, Compression, SnapshotConfig, SnapshotFormat};
      let dir = tempfile::tempdir().unwrap();
      let cfg = SnapshotConfig {
          dir: dir.path().to_path_buf(),
          name: "g".into(),
          key: Some("sha1abc".into()),
          format: SnapshotFormat::Bincode,
          compression: Compression::None,
          keep: 3,
      };
      let mut graph: Graph<(), ()> = Graph::new();
      graph.add_node(());
      save(&cfg, &graph).unwrap();
      let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap()
          .filter_map(|e| e.ok()).collect();
      assert_eq!(entries.len(), 1);
      let name = entries[0].file_name().to_string_lossy().into_owned();
      assert_eq!(name, "g-sha1abc.snap");
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_save_same_key_idempotent() {
      use petgraph::Graph;
      use petgraph_live::snapshot::{save, Compression, SnapshotConfig, SnapshotFormat};
      let dir = tempfile::tempdir().unwrap();
      let cfg = SnapshotConfig {
          dir: dir.path().to_path_buf(), name: "g".into(),
          key: Some("v1".into()), format: SnapshotFormat::Bincode,
          compression: Compression::None, keep: 3,
      };
      let graph: Graph<(), ()> = Graph::new();
      save(&cfg, &graph).unwrap();
      save(&cfg, &graph).unwrap();
      // Same key → same filename → still 1 file
      let count = std::fs::read_dir(dir.path()).unwrap().count();
      assert_eq!(count, 1);
  }
  ```
- [ ] Run `cargo test --features snapshot --test snapshot test_save_creates_file` — compile error expected.
- [ ] Implement `pub fn save<G>(cfg: &SnapshotConfig, graph: &G) -> Result<(), SnapshotError>`:
  1. Extract key: `cfg.key.as_deref().ok_or(SnapshotError::InvalidKey("None key in save".into()))?`.
  2. `sanitized = sanitize_key(key)?`
  3. Extension: `.snap` (Bincode+None), `.snap.zst` (Bincode+Zstd), `.json` (Json+None), `.json.zst` (Json+Zstd).
  4. `filename = format!("{}-{}{}", cfg.name, sanitized, ext)`.
  5. `final_path = cfg.dir.join(filename)`, `tmp_path = {final_path}.tmp`.
  6. Build `SnapshotMeta::new(key, cfg.format.clone(), cfg.compression.clone(), graph.node_count(), graph.edge_count())`.
  7. Serialize:
     - Bincode: encode meta bytes, encode graph bytes; write `meta_len (u64 LE) ++ meta_bytes ++ graph_bytes`.
     - JSON: `serde_json::to_vec(&json!({"meta": meta, "graph": graph}))`.
  8. If `Compression::Zstd { level }` (feature-gated): compress with `zstd::encode_all`.
  9. Write to `tmp_path`. `fs::rename(tmp_path, final_path)`.
  10. Call `rotation::keep_n(&cfg.dir, &cfg.name, cfg.keep)`.
- [ ] Trait bounds on `G`: `G: serde::Serialize + petgraph::visit::NodeCount + petgraph::visit::EdgeCount`.
- [ ] Run `cargo test --features snapshot --test snapshot test_save_creates_file` — pass.
- [ ] Commit: `feat(snapshot): implement save with key-as-filename, atomic write, rotation`

---

### Task 7 — `io.rs`: `load` and `load_or_build` (sonnet)

- [ ] Write failing tests:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_save_load_roundtrip_bincode() {
      // save 3-node graph, load with same key → Ok(Some(g)), g.node_count() == 3
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_save_load_roundtrip_json() {
      // same as above with SnapshotFormat::Json
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_load_key_not_found() {
      // save with key Some("v1"), load with key Some("v2") → Err(KeyNotFound { key: "v2" })
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_load_no_snapshot_returns_none() {
      // load from empty dir with key Some("v1") → Err(KeyNotFound)
      // load from empty dir with key None → Ok(None)
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_load_none_key_returns_most_recent() {
      // save "v1" then "v2", load with key None → returns "v2" graph
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_load_or_build_falls_back_on_empty() {
      // empty dir, key Some("v1") → KeyNotFound → build closure called → saved → can load
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_load_or_build_falls_back_on_key_not_found() {
      // save "v1", load_or_build with "v2" → build called → saves "v2" → original "v1" still present
  }
  ```
- [ ] Implement `load<G>(cfg: &SnapshotConfig) -> Result<Option<G>, SnapshotError>`:
  1. If `cfg.key = Some(k)`:
     - `sanitized = sanitize_key(k)?`
     - Search `cfg.dir` for `{name}-{sanitized}.{ext}` (try all four extensions).
     - If not found → `Err(SnapshotError::KeyNotFound { key: k.to_string() })`.
  2. If `cfg.key = None`:
     - `list_snapshot_files` → take last (most recent by mtime). If empty → `Ok(None)`.
  3. Read bytes. Decompress if `.snap.zst` / `.json.zst`.
  4. Deserialize:
     - Bincode: read 8-byte LE `meta_len`, skip meta bytes, decode remainder → `G`.
     - JSON: parse `Value`, extract `"graph"` field → `serde_json::from_value`.
  5. Return `Ok(Some(graph))`.
- [ ] Implement `load_or_build<G, F>(cfg: &SnapshotConfig, build: F) -> Result<G, SnapshotError>`:
  ```rust
  match load(cfg) {
      Ok(Some(g)) => Ok(g),
      Ok(None) | Err(SnapshotError::KeyNotFound { .. }) | Err(SnapshotError::NoSnapshotFound) => {
          let g = build()?;
          let _ = save(cfg, &g); // best-effort, don't fail on save error
          Ok(g)
      }
      Err(e) => Err(e),
  }
  ```
- [ ] Re-export in `src/snapshot/mod.rs`: `pub use io::{inspect, list, load, load_or_build, purge, save};`
- [ ] Run `cargo test --features snapshot --test snapshot` — all passing tests must pass.
- [ ] Commit: `feat(snapshot): implement load and load_or_build`

---

### Task 8 — `io.rs`: `inspect`, `list`, `purge` (sonnet)

- [ ] Write failing tests:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_inspect_reads_meta_without_graph() {
      // save 2-node graph with key "sha1", inspect with same key
      // → meta.node_count == 2, meta.key == "sha1" — no G type param needed
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_inspect_none_key_most_recent() {
      // save "v1" then "v2", inspect with key None → meta.key == "v2"
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_list_sorted_oldest_first() {
      // save 3 times with different keys, list → 3 entries in ascending mtime order
  }

  #[cfg(feature = "snapshot")]
  #[test]
  fn test_purge_deletes_all() {
      // save 4 times with 4 different keys, purge → returns 4, dir empty
  }
  ```
- [ ] Implement `inspect(cfg: &SnapshotConfig) -> Result<Option<SnapshotMeta>, SnapshotError>`:
  - Locate file using same logic as `load` (by key or most recent).
  - If none → `Ok(None)`.
  - Bincode: read 8-byte `meta_len`, read exactly that many bytes, decode `SnapshotMeta`.
  - JSON: parse `Value`, extract `"meta"` field → `serde_json::from_value`.
- [ ] Implement `list(cfg: &SnapshotConfig) -> Result<Vec<(PathBuf, SnapshotMeta)>, SnapshotError>`:
  - `list_snapshot_files` → for each file, call `inspect_file(path, format)` internal helper.
  - Return `Vec<(PathBuf, SnapshotMeta)>` sorted oldest-first (ascending mtime).
- [ ] Implement `purge(cfg: &SnapshotConfig) -> Result<usize, SnapshotError>`:
  - List all matching files, delete each, return count.
- [ ] Run `cargo test --features snapshot --test snapshot` — all tests pass.
- [ ] Commit: `feat(snapshot): implement inspect, list, purge`

---

### Task 9 — Rotation integration test (sonnet)

- [ ] Write and run:
  ```rust
  #[cfg(feature = "snapshot")]
  #[test]
  fn test_rotation_save_5_keep_3() {
      // save 5 times with distinct keys (key1..key5), keep=3
      // verify list() returns 3 entries
      // verify the 3 most recently written are retained
  }
  ```
- [ ] Confirm test passes.
- [ ] Commit: `test(snapshot): add rotation integration test (save 5 keep 3)`

---

### Task 10 — zstd compression integration test (sonnet)

- [ ] Write:
  ```rust
  #[cfg(all(feature = "snapshot", feature = "snapshot-zstd"))]
  #[test]
  fn test_zstd_roundtrip() {
      // save 100-node graph with Compression::Zstd { level: 3 }
      // load back, verify node_count == 100
      // verify file ends with .snap.zst
  }
  ```
- [ ] Run `cargo test --features snapshot,snapshot-zstd --test snapshot test_zstd_roundtrip` — pass.
- [ ] Commit: `test(snapshot): add zstd compression roundtrip integration test`

---

### Task 11 — Example `examples/snapshot_basic.rs` (sonnet)

- [ ] Create `examples/snapshot_basic.rs` demonstrating: `load_or_build`, `inspect`, `list`, `purge`.
  Runs with `cargo run --example snapshot_basic --features snapshot`.
- [ ] The example must:
  - Use a `tempdir` for the snapshot directory
  - Show build closure called on first run
  - Show `inspect` printing node count and key
  - Show `list` iterating snapshots
  - Show `purge` and confirm count
- [ ] Run example — no panics.
- [ ] Commit: `docs(snapshot): add snapshot_basic example`

---

### Task 12 — Rustdoc (sonnet)

- [ ] Add full module-level rustdoc to `src/snapshot/mod.rs` with the end-to-end example from `docs/api-design.md`.
- [ ] Add `# Examples` doctests on `save`, `load`, `load_or_build`, `inspect`, `list`, `purge`.
- [ ] Add rustdoc to `SnapshotConfig`, `SnapshotFormat`, `Compression`, `SnapshotMeta`, `SnapshotError`.
- [ ] Run `cargo test --doc --features snapshot` — all doctests pass.
- [ ] Commit: `docs(snapshot): add rustdoc and doctests for all public items`

---

### Task 13 — Final check (sonnet)

- [ ] Run `cargo test --features snapshot,snapshot-zstd` — zero failures.
- [ ] Run `cargo test` (no features) — zero failures.
- [ ] Run `cargo clippy --features snapshot,snapshot-zstd -- -D warnings` — zero warnings.
- [ ] Run `cargo fmt --check` — clean.
- [ ] Commit: `chore(snapshot): clippy and fmt clean`
