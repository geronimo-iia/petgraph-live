---
title: "cache module"
summary: "Thread-safe generation-keyed graph cache — lazy rebuild, concurrent reads, Arc-based sharing."
read_when:
  - Modifying GenerationCache behaviour or concurrency model
  - Understanding hit/miss semantics before integrating
status: implemented
last_updated: "2026-05-02"
---

# `cache` module


## API

```rust
pub struct GenerationCache<G> { /* RwLock<Option<CacheEntry<G>>> */ }

impl<G> GenerationCache<G> {
    pub fn new() -> Self;
    pub fn get_or_build<F, E>(&self, generation: u64, build: F) -> Result<Arc<G>, E>
    where F: FnOnce() -> Result<G, E>;
    pub fn invalidate(&self);
    pub fn current_generation(&self) -> Option<u64>;
}
impl<G> Default for GenerationCache<G> { ... }
```

## Contracts

| Condition                       | Result                                      |
| ------------------------------- | ------------------------------------------- |
| Cache empty or stale generation | Call `build`, store, return `Arc`           |
| Cached generation matches       | Return cached `Arc`, `build` not called     |
| `build` returns `Err`           | Propagate, leave existing entry unchanged   |
| After `invalidate()`            | Next call rebuilds regardless of generation |

`build` runs outside any lock. Concurrent misses may each call `build` —
last writer wins. `build` must be idempotent. To prevent redundant work,
serialize at a higher level (e.g. `GraphState::get_fresh`).

## Out of scope

Multiple cached values, TTL eviction, automatic invalidation, build coalescing.

## Files

`src/cache.rs` · `tests/cache.rs` · `examples/cache_basic.rs`
