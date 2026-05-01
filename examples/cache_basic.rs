//! Basic usage of GenerationCache.
//! Run with: cargo run --example cache_basic
use petgraph::Graph;
use petgraph_live::cache::GenerationCache;
use std::sync::Arc;

fn build_graph(seed: u64) -> Result<Graph<u64, ()>, ()> {
    let mut g = Graph::new();
    let a = g.add_node(seed);
    let b = g.add_node(seed * 2);
    g.add_edge(a, b, ());
    Ok(g)
}

fn main() {
    let cache: GenerationCache<Graph<u64, ()>> = GenerationCache::new();

    let g1 = cache.get_or_build(1, || build_graph(1)).unwrap();
    println!("generation=1 nodes={}", g1.node_count());

    // Same generation: cached.
    let g2 = cache.get_or_build(1, || build_graph(99)).unwrap();
    println!("generation=1 again: same Arc? {}", Arc::ptr_eq(&g1, &g2));

    // Bump generation: rebuilds.
    let g3 = cache.get_or_build(2, || build_graph(2)).unwrap();
    println!("generation=2 nodes={}", g3.node_count());
    println!("current_generation={:?}", cache.current_generation());

    // Invalidate: forces rebuild even with same generation.
    cache.invalidate();
    println!(
        "after invalidate: current_generation={:?}",
        cache.current_generation()
    );
    let g4 = cache.get_or_build(2, || build_graph(2)).unwrap();
    println!("rebuilt generation=2 nodes={}", g4.node_count());
}
