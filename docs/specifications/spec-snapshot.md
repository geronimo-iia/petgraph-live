---
title: "snapshot module"
summary: "Serde-based disk persistence for petgraph graphs — key-as-filename, atomic write, mtime rotation, optional zstd compression."
read_when:
  - Implementing or modifying snapshot save/load/inspect/purge
  - Understanding binary and JSON file layout
  - Adding or changing compression support
status: implemented
last_updated: "2026-05-02"
---

# Specification: `snapshot` module

**Feature flags:** `snapshot` / `snapshot-zstd`

## File naming

```
{name}-{sanitized_key}.snap
{name}-{sanitized_key}.snap.zst
{name}-{sanitized_key}.json
{name}-{sanitized_key}.json.zst
```

`sanitize_key`: replace chars outside `[a-zA-Z0-9_.-]` with `_`. Error if result trims to empty.
Same key → same filename → idempotent overwrite.

## Binary layout (bincode)

```
meta_len   : u64 le
meta_bytes : [u8; meta_len]   (bincode SnapshotMeta)
graph_bytes: [u8]             (bincode G, remainder)
```

`inspect` reads only `meta_len + meta_bytes`. Graph bytes never touched.

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

## Bincode API

Uses `bincode 2.x` — `bincode::serde::encode_to_vec` / `bincode::serde::decode_from_slice`
with `bincode::config::standard()`. Requires `features = ["serde"]` on the bincode dep.
Not the 1.x API. Not `bincode::Encode`/`Decode` traits.

## Known constraints

- `gen` is a reserved keyword in Rust 2024 — never use as variable name.
- `std::io::Error` has no `PartialEq` — `SnapshotError` uses a manual impl comparing by `ErrorKind`.
- Tmp path: `format!("{}.tmp", final_path)` not `path.with_extension(...)` — double extensions (`.snap.zst`) break the latter.
- Test graph types must be owned (e.g. `Graph<String, ()>`), not borrowed (`Graph<&str, ()>`), for `DeserializeOwned`.
