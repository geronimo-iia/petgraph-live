---
title: "snapshot lazy metadata — inspect and list without loading graph body"
summary: "Partial file read for bincode inspect/list; serde ignore-unknown-fields for JSON — avoids deserializing G when only SnapshotMeta is needed."
read_when:
  - Implementing or modifying inspect() or list() in snapshot module
  - Understanding why inspect is cheaper than load
  - Reasoning about compressed vs uncompressed partial read behavior
status: implemented
last_updated: "2026-05-02"
---

# Specification: snapshot lazy metadata

**Crate:** `petgraph-live`
**Feature flag:** `snapshot` (no new feature — improvement to existing functions)
**Status:** implemented
**Depends on:** `snapshot` module (implemented)

---

## Problem

`inspect()` and `list()` currently read the full snapshot file into memory, decompress it
entirely, then extract only the metadata header. For large graphs (100k+ nodes, multi-MB
snapshots) this loads and decompresses megabytes that are immediately discarded.

---

## Solution

Two independent improvements, one per format:

### Bincode — partial file read

The bincode layout is:

```
meta_len   : u64 le           (8 bytes)
meta_bytes : [u8; meta_len]   (bincode SnapshotMeta)
graph_bytes: [u8]             (remainder — never needed for inspect)
```

`inspect` only needs `8 + meta_len` bytes. Replace `std::fs::read(&path)` with:

```rust
use std::io::{Read, Seek};

let mut f = std::fs::File::open(&path)?;
let mut len_buf = [0u8; 8];
f.read_exact(&mut len_buf)?;
let meta_len = u64::from_le_bytes(len_buf) as usize;
let mut meta_buf = vec![0u8; meta_len];
f.read_exact(&mut meta_buf)?;
// f is dropped — graph bytes never read from disk
```

**Win:** I/O reduced from `file_size` bytes to `8 + meta_len` bytes (typically a few hundred bytes vs. megabytes).

### JSON — skip graph deserialization

The JSON layout is:

```json
{"meta": <SnapshotMeta>, "graph": <G>}
```

Replace the current `serde_json::Value` parse (which allocates the full document) with a
targeted struct that serde silently ignores unknown fields by default:

```rust
#[derive(serde::Deserialize)]
struct MetaOnly {
    meta: SnapshotMeta,
}
```

`serde_json::from_slice::<MetaOnly>(&bytes)` tokenizes the full JSON but never calls
`G::deserialize` and never allocates a `G` value. For graphs with complex node/edge
types this eliminates the most expensive allocation.

**Win:** Avoids `G::deserialize`. Graph JSON bytes still tokenized — full I/O still
occurs — but no Rust graph object is constructed.

### Compressed files — known limitation

For `.snap.zst`, `.snap.lz4`, `.json.zst`, `.json.lz4`: the entire compressed payload
must be decompressed before partial reads or serde tricks can apply. Partial I/O is not
possible through a block decompressor without seeking. This is documented, not fixed.

The gains still apply after decompression: for bincode+compression, partial parse
applies to the decompressed bytes. For JSON+compression, `MetaOnly` still avoids `G`.

---

## Scope

In scope:
- `inspect()` in `src/snapshot/io.rs` — both format paths
- `list()` in `src/snapshot/io.rs` — calls `read_meta_from_bytes`, same improvement
- `read_meta_from_bytes()` — internal helper, both paths updated here

Out of scope:
- Streaming decompression (would require replacing zstd/lz4 with streaming wrappers)
- `load()` — always needs graph bytes
- `load_or_build()` — delegates to `load()`

---

## API

No public API changes. `inspect` and `list` signatures are unchanged:

```rust
pub fn inspect(cfg: &SnapshotConfig) -> Result<Option<SnapshotMeta>, SnapshotError>;
pub fn list(cfg: &SnapshotConfig) -> Result<Vec<(PathBuf, SnapshotMeta)>, SnapshotError>;
```

---

## Implementation detail

The change is in `read_meta_from_bytes` (shared by `inspect` and `list`) and in the
`inspect` call site which currently does `std::fs::read` before calling it.

Split into two functions:

```rust
// For inspect — reads only what's needed from disk (uncompressed bincode)
fn read_meta_from_file(path: &Path) -> Result<SnapshotMeta, SnapshotError>

// For load — still reads full file (graph bytes needed)
fn read_meta_from_bytes(path: &Path, bytes: &[u8]) -> Result<SnapshotMeta, SnapshotError>
```

`inspect` uses `read_meta_from_file` for uncompressed bincode, falls back to full read
for compressed or JSON files. `list` follows the same pattern.

The JSON path in `read_meta_from_bytes` changes from `serde_json::Value` to `MetaOnly`.

---

## Test matrix

| Test | Verifies |
|---|---|
| `test_inspect_does_not_read_graph_bytes_bincode` | bincode inspect: file handle read position after inspect = `8 + meta_len` (not EOF) |
| `test_inspect_json_meta_only` | JSON inspect: works on file where graph field is gigantic (large serialized value) |
| `test_list_meta_only_bincode` | list() returns correct meta for multiple bincode files |
| `test_list_meta_only_json` | list() returns correct meta for multiple JSON files |
| existing tests | `test_inspect_reads_meta_without_graph`, `test_inspect_none_key_most_recent` still pass |

---

## Known constraints

- Edition 2024: never use `gen` as variable name
- `std::io::Error` has no `PartialEq` — `SnapshotError::Io` uses manual impl
- Tmp path: `format!("{}.tmp", final_path)` not `path.with_extension(...)` (double extension issue)
- Test graph types must be owned (`Graph<String, ()>`), not borrowed
