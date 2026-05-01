---
title: "snapshot module"
summary: "Serde-based disk persistence for petgraph graphs — key-as-filename, atomic write, mtime rotation, optional zstd compression."
read_when:
  - Implementing or modifying snapshot save/load/inspect/purge
  - Understanding binary and JSON file layout
  - Adding or changing compression support
  - Writing tests for the snapshot module
status: pre-implementation
last_updated: "2026-05-02"
---

# Specification: `snapshot` module

**Crate:** `petgraph-live`
**Feature flag:** `snapshot` (sub-feature: `snapshot-zstd`)
**Status:** pre-implementation

---

## Purpose

Serde-based disk persistence for any petgraph graph type. Survives process
restarts. Atomic writes, key-as-filename validity, optional zstd compression,
mtime-based rotation.

---

## Scope

In scope:
- Save/load any graph type implementing `Serialize + DeserializeOwned`
- Bincode (compact binary) and JSON (human-readable) formats
- Optional zstd compression (sub-feature)
- Key encoded in filename — no key stored in file body for lookup
- Atomic write (temp file + rename)
- Rotation: keep latest N snapshots by filesystem mtime
- `inspect`: read metadata without deserializing graph
- `list`: enumerate snapshots with metadata
- `purge`: delete all snapshots for a given name

Out of scope:
- Encryption
- Remote storage
- Streaming / incremental serialization
- Schema migration (version mismatch → caller rebuilds)

---

## Feature flags

```toml
[features]
snapshot = [
    "dep:serde", "dep:serde_json", "dep:bincode",
    "dep:thiserror", "petgraph/serde-1",
]
snapshot-zstd = ["snapshot", "dep:zstd"]
```

Bincode version: **2.x** (`bincode::encode_to_vec` / `bincode::decode_from_slice`
with `bincode::config::standard()`). Not 1.x.

---

## File naming

Key is encoded in the filename. No key stored in file body.

```
{name}-{sanitized_key}.snap       bincode, uncompressed
{name}-{sanitized_key}.snap.zst   bincode, zstd compressed
{name}-{sanitized_key}.json       JSON, uncompressed
{name}-{sanitized_key}.json.zst   JSON, zstd compressed
```

`sanitize_key(k)`: replace any char outside `[a-zA-Z0-9_.-]` with `_`.
Returns `Err(InvalidKey)` if result is empty (e.g. input is all spaces).

Two saves with the same key → same filename → idempotent overwrite.

---

## Binary file layout (bincode)

```
meta_len  : u64 little-endian
meta_bytes: [u8; meta_len]   (bincode-encoded SnapshotMeta)
graph_bytes: [u8]            (bincode-encoded G, remainder of file)
```

`inspect` reads only `meta_len` + `meta_bytes`. Graph bytes are not read or
decoded. This property must be covered by a dedicated test (save large graph,
inspect, verify no full deserialization needed).

## JSON file layout

```json
{"meta": <SnapshotMeta>, "graph": <G>}
```

`inspect` extracts `"meta"` field via `serde_json::Value`, does not deserialize `"graph"`.

---

## Public types

### `SnapshotFormat`

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SnapshotFormat { Bincode, Json }
```

### `Compression`

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Compression {
    None,
    #[cfg(feature = "snapshot-zstd")]
    Zstd { level: i32 },
}
```

### `SnapshotConfig`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    pub dir:         PathBuf,
    pub name:        String,
    #[serde(skip)]   // runtime value — always None after deserialization
    pub key:         Option<String>,
    pub format:      SnapshotFormat,
    pub compression: Compression,
    pub keep:        usize,    // default: 3
}
```

`key` semantics:
- `Some(k)` → `load` looks for `{name}-{sanitize(k)}.{ext}`. Returns
  `Err(KeyNotFound)` if absent.
- `None` → `load` returns the most recent file by mtime. Returns `Ok(None)` if
  dir has no matching files.
- `#[serde(skip)]` — always deserializes to `None`. Set programmatically at
  runtime (e.g. current git SHA). Never put in a static config file.

Used by `GraphState` with `key = None` — key management is internal there.

### `SnapshotMeta`

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub key:                   String,
    pub format:                SnapshotFormat,
    pub compression:           Compression,
    pub node_count:            usize,
    pub edge_count:            usize,
    pub created_at:            u64,    // Unix seconds
    pub petgraph_live_version: String,
}
```

`key` is redundant with the filename — included for convenience when reading
metadata in isolation.

### `SnapshotError`

```rust
#[derive(Debug, thiserror::Error)]
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

---

## Public functions

```rust
// Save graph. Atomic write. Rotates old snapshots per cfg.keep.
// cfg.key must be Some — Err(InvalidKey) if None.
pub fn save<G>(cfg: &SnapshotConfig, graph: &G) -> Result<(), SnapshotError>
where G: Serialize + NodeCount + EdgeCount;

// Load snapshot matching cfg.key (Some) or most recent (None).
// Returns Ok(None) if no matching file exists and key is None.
// Returns Err(KeyNotFound) if key is Some and no matching file.
pub fn load<G>(cfg: &SnapshotConfig) -> Result<Option<G>, SnapshotError>
where G: DeserializeOwned;

// Load or build. Calls build on KeyNotFound / NoSnapshotFound / Ok(None).
// Saves result after build. Best-effort save (logs warn on error, does not fail).
pub fn load_or_build<G, F>(cfg: &SnapshotConfig, build: F) -> Result<G, SnapshotError>
where
    G: Serialize + DeserializeOwned + NodeCount + EdgeCount,
    F: FnOnce() -> Result<G, SnapshotError>;

// Read SnapshotMeta without deserializing graph body.
pub fn inspect(cfg: &SnapshotConfig) -> Result<Option<SnapshotMeta>, SnapshotError>;

// List all snapshots matching cfg.name in cfg.dir, ascending mtime (oldest first).
pub fn list(cfg: &SnapshotConfig) -> Result<Vec<(PathBuf, SnapshotMeta)>, SnapshotError>;

// Delete all snapshots matching cfg.name in cfg.dir. Returns count deleted.
pub fn purge(cfg: &SnapshotConfig) -> Result<usize, SnapshotError>;
```

---

## Rotation

Rotation is mtime-based (not filename/key order). Key strings have no inherent
chronological ordering. On every `save`: after writing the new file, call
`keep_n(&cfg.dir, &cfg.name, cfg.keep)` which deletes all but the `keep` newest
files by filesystem mtime.

---

## Atomic write

1. Serialize + optionally compress → bytes
2. Write to `{final_path}.tmp`
3. `std::fs::rename(tmp_path, final_path)` — atomic on POSIX

On crash between step 2 and 3: orphan `.tmp` file exists, final file unchanged.
`.tmp` files are not matched by rotation or list functions.

---

## Files

| Path | Responsibility |
|---|---|
| `src/snapshot/mod.rs` | Re-exports of all public items |
| `src/snapshot/config.rs` | `SnapshotConfig`, `SnapshotFormat`, `Compression`, `sanitize_key` |
| `src/snapshot/meta.rs` | `SnapshotMeta` |
| `src/snapshot/error.rs` | `SnapshotError` |
| `src/snapshot/io.rs` | `save`, `load`, `load_or_build`, `inspect`, `list`, `purge` |
| `src/snapshot/rotation.rs` | `keep_n`, `list_snapshot_files` |
| `tests/snapshot.rs` | Integration tests (all `#[cfg(feature = "snapshot")]`) |
| `examples/snapshot_basic.rs` | Usage demo |

---

## Test matrix

| Test | Verifies |
|---|---|
| `test_config_defaults` | Config fields |
| `test_sanitize_key` | Pass-through, replacement, empty→error |
| `test_meta_new` | Fields populated, version non-empty |
| `test_error_display` | `KeyNotFound`, `InvalidKey`, `NoSnapshotFound` messages |
| `test_rotation_keep_3` | 5 files → 3 newest retained by mtime |
| `test_save_creates_file` | Correct filename from key |
| `test_save_same_key_idempotent` | Two saves same key → 1 file |
| `test_save_load_roundtrip_bincode` | 3-node graph round-trips |
| `test_save_load_roundtrip_json` | Same with JSON format |
| `test_load_key_not_found` | Wrong key → `Err(KeyNotFound)` |
| `test_load_no_snapshot_returns_none` | Empty dir, `key=None` → `Ok(None)` |
| `test_load_none_key_returns_most_recent` | Two snapshots, `key=None` → newest |
| `test_load_or_build_falls_back_on_empty` | Build called, file saved |
| `test_load_or_build_falls_back_on_key_not_found` | Wrong key → build |
| `test_inspect_reads_meta_without_graph` | `node_count` correct, no full decode |
| `test_inspect_none_key_most_recent` | Most recent meta returned |
| `test_list_sorted_oldest_first` | Ascending mtime order |
| `test_purge_deletes_all` | Count returned, dir empty |
| `test_rotation_save_5_keep_3` | Integration: save loop, rotation |
| `test_zstd_roundtrip` | `#[cfg(feature="snapshot-zstd")]` — compress + load |
