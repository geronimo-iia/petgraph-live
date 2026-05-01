use petgraph_live::cache::GenerationCache;
use std::sync::Arc;

#[test]
fn test_cache_miss_calls_build() {
    let cache: GenerationCache<Vec<i32>> = GenerationCache::new();
    let call_count = std::cell::Cell::new(0u32);
    let _graph = cache
        .get_or_build::<_, ()>(1, || {
            call_count.set(call_count.get() + 1);
            Ok(vec![1, 2, 3])
        })
        .unwrap();
    assert_eq!(call_count.get(), 1);
    // Same generation: build NOT called.
    let _graph2 = cache
        .get_or_build::<_, ()>(1, || {
            call_count.set(call_count.get() + 1);
            Ok(vec![4, 5, 6])
        })
        .unwrap();
    assert_eq!(call_count.get(), 1);
}

#[test]
fn test_cache_hit_returns_same_arc() {
    let cache: GenerationCache<Vec<i32>> = GenerationCache::new();
    let a1 = cache.get_or_build::<_, ()>(42, || Ok(vec![1])).unwrap();
    let a2 = cache.get_or_build::<_, ()>(42, || Ok(vec![2])).unwrap();
    assert!(Arc::ptr_eq(&a1, &a2));
}

#[test]
fn test_generation_change_triggers_rebuild() {
    let cache: GenerationCache<u64> = GenerationCache::new();
    let a1 = cache.get_or_build::<_, ()>(1, || Ok(100)).unwrap();
    assert_eq!(*a1, 100);
    let a2 = cache.get_or_build::<_, ()>(2, || Ok(200)).unwrap();
    assert_eq!(*a2, 200);
    assert!(!Arc::ptr_eq(&a1, &a2));
    assert_eq!(cache.current_generation(), Some(2));
}

#[test]
fn test_invalidate_forces_rebuild() {
    let cache: GenerationCache<u32> = GenerationCache::new();
    let a1 = cache.get_or_build::<_, ()>(1, || Ok(10)).unwrap();
    cache.invalidate();
    assert_eq!(cache.current_generation(), None);
    let a2 = cache.get_or_build::<_, ()>(1, || Ok(20)).unwrap();
    assert_eq!(*a2, 20);
    assert!(!Arc::ptr_eq(&a1, &a2));
}

#[test]
fn test_concurrent_reads() {
    use std::thread;

    let cache: Arc<GenerationCache<Vec<u8>>> = Arc::new(GenerationCache::new());
    cache
        .get_or_build::<_, ()>(1, || Ok(vec![0u8; 1024]))
        .unwrap();

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let c = Arc::clone(&cache);
            thread::spawn(move || {
                for _ in 0..100 {
                    let g = c.get_or_build::<_, ()>(1, || Ok(vec![1u8; 1024])).unwrap();
                    assert_eq!(g.len(), 1024);
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}
