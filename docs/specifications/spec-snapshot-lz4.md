---
title: "snapshot-lz4 sub-feature"
summary: "LZ4 compression for snapshot files via lz4_flex — pure Rust, no C binding, 2–3× faster decompression than zstd for warm-start latency reduction."
read_when:
  - Adding or modifying LZ4 compression support in snapshot module
  - Choosing between snapshot-lz4 and snapshot-zstd
  - Understanding file extension conventions for compressed snapshots
status: implemented
last_updated: "2026-05-02"
---

# Specification: `snapshot-lz4` sub-feature

**Crate:** `petgraph-live`
**Feature flag:** `snapshot-lz4` (implies `snapshot`)
**Status:** pre-implementation
**Depends on:** `snapshot` module

---

## Purpose

Add LZ4 compression as an alternative to zstd for snapshot files. LZ4
decompresses 2–3× faster than zstd at the cost of a larger file. The primary
benefit is reduced warm-start latency — the time to load a snapshot on process
startup before a `GraphState` or manual `load()` call returns.

---

## When to use each codec

| Scenario | Recommended compression |
|---|---|
| Warm start latency matters (server restart, frequent cold start) | `Compression::Lz4` |
| Disk space or transfer size matters (large graph, many rotations) | `Compression::Zstd { level }` |
| CI / automated pipeline (rebuild is fast anyway) | `Compression::None` |
| Default / no opinion | `Compression::None` |

LZ4 and zstd serve different optimisation targets and are not substitutes.
Both sub-features can be enabled simultaneously; the caller picks the variant
in `SnapshotConfig::compression`.

---

## Dependency

| Crate | Version | Notes |
|---|---|---|
| `lz4_flex` | `0.13` | Pure Rust, no `liblz4` C binding, no `unsafe` by default |

Compile-time cost: small (pure Rust, no sys crate).

---

## Feature flag

```toml
[features]
snapshot-lz4 = ["snapshot", "dep:lz4_flex"]

[dependencies]
lz4_flex = { version = "0.13", optional = true }
```

---

## `Compression` enum addition

```rust
pub enum Compression {
    None,
    #[cfg(feature = "snapshot-zstd")]
    Zstd { level: i32 },
    #[cfg(feature = "snapshot-lz4")]
    Lz4,
}
```

`Lz4` has no parameters — `lz4_flex` does not expose a compression level in
its default API.

---

## File naming

```
{name}-{sanitized_key}.snap.lz4    (bincode + lz4)
{name}-{sanitized_key}.json.lz4    (json + lz4)
```

Extension detection on load is by filename suffix (same convention as `.snap.zst`).

---

## API

No new public functions. `Compression::Lz4` is used in `SnapshotConfig::compression`:

```rust
let cfg = SnapshotConfig {
    dir:         PathBuf::from("/state/snapshots"),
    name:        "graph".into(),
    key:         Some(current_key()),
    format:      SnapshotFormat::Bincode,
    compression: Compression::Lz4,
    keep:        3,
};

snapshot::save(&cfg, &graph)?;
let g: Option<MyGraph> = snapshot::load(&cfg)?;
```

`inspect`, `list`, `purge`, `load_or_build` — all work unchanged.

---

## Performance notes

Typical numbers for sequential in-memory compression (not streaming):

| Codec | Compress | Decompress | Ratio vs uncompressed |
|---|---|---|---|
| None | — | — | 1× |
| LZ4 | ~500 MB/s | ~1800 MB/s | 0.5–0.7× |
| zstd level 3 | ~250 MB/s | ~1000 MB/s | 0.3–0.5× |

For a 10 MB bincode snapshot:
- LZ4 decompress: ~5 ms
- zstd decompress: ~10 ms
- Difference is measurable at startup for large graphs, negligible for small ones.

---

## Test matrix

| Test | Verifies |
|---|---|
| `test_lz4_roundtrip` | save + load with `Compression::Lz4` → graph identical |
| `test_lz4_extension` | saved file has `.snap.lz4` extension |
| `test_lz4_inspect` | `inspect()` reads meta without loading graph body |
| combined feature test | `--features snapshot-lz4,snapshot-zstd` compiles and tests clean |

---

## Known constraints

- `lz4_flex::compress_prepend_size` is infallible — wraps directly in `Ok(...)`.
- `lz4_flex::decompress_size_prepended` can fail on corrupt input — maps to `SnapshotError::CompressionError`.
- No streaming API — snapshot bytes are loaded fully into memory before decompression (same as zstd path).
