---
title: "snapshot module"
summary: "Serde-based disk persistence for petgraph graphs — key-as-filename, atomic write, mtime rotation, optional zstd compression."
read_when:
  - Implementing or modifying snapshot save/load/inspect/purge
  - Understanding binary and JSON file layout
  - Adding or changing compression support
status: implemented
last_updated: "2026-07-23"
---

# Specification: `snapshot` module

**Feature flags:** `snapshot` / `snapshot-zstd` / `snapshot-lz4`

## File naming

```
{name}-{sanitized_key}.snap
{name}-{sanitized_key}.snap.zst
{name}-{sanitized_key}.snap.lz4
{name}-{sanitized_key}.json
{name}-{sanitized_key}.json.zst
{name}-{sanitized_key}.json.lz4
```

`sanitize_key`: replace chars outside `[a-zA-Z0-9_.-]` with `_`. Error if result trims to empty.
Same key → same filename → idempotent overwrite.

## Binary layout (postcard, v2)

```
magic      : [u8; 4]          b"PGL\x02"
meta_len   : u64 le           (8 bytes)
meta_bytes : [u8; meta_len]   (postcard SnapshotMeta)
graph_bytes: [u8]             (postcard G, remainder)
```

`inspect` reads only `4 + 8 + meta_len` bytes. Graph bytes never touched.

### Legacy detection (v1 / bincode-era files)

Files written by v0.4.x lack the `b"PGL\x02"` magic prefix. On read:
- `load()` returns `Err(SnapshotError::LegacyFormat { path })`
- `load_or_build()` treats `LegacyFormat` as a cache miss — deletes nothing, calls `build`, saves a new v2 file
- `inspect()` / `list()` return `Err(SnapshotError::LegacyFormat { path })`

Users calling `load()` directly: delete old `.snap` / `.snap.zst` / `.snap.lz4` files before upgrading.

## JSON layout

```json
{"meta": <SnapshotMeta>, "graph": <G>}
```

## Key semantics

- `Some(k)` → load looks for `{name}-{sanitize(k)}.*`. Missing → `Err(KeyNotFound)`.
- `None` → load returns most recent by mtime. Empty dir → `Ok(None)`.
- `#[serde(skip)]` — always `None` after deserialization. Set at runtime only.
- `GraphState` uses `key = None`; key management is internal there.

## Rotation

mtime-based (not filename order). On every `save`: delete all but `keep` newest files.
`.tmp` files excluded from rotation and list.

## Serialization API

Uses `postcard 1.x` — `postcard::to_allocvec` (encode) / `postcard::from_bytes` (decode).
Requires `features = ["use-std"]` on the postcard dep. No config object needed.

## Known constraints

- `gen` is a reserved keyword in Rust 2024 — never use as variable name.
- `std::io::Error` has no `PartialEq` — `SnapshotError` uses a manual impl comparing by `ErrorKind`.
- Tmp path: `format!("{}.tmp", final_path)` not `path.with_extension(...)` — double extensions (`.snap.zst`) break the latter.
- Test graph types must be owned (e.g. `Graph<String, ()>`), not borrowed (`Graph<&str, ()>`), for `DeserializeOwned`.
