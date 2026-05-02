//! Basic usage of GraphState.
//! Run with: cargo run --example live_basic --features snapshot
use petgraph::Graph;
use petgraph_live::{
    live::{GraphState, GraphStateConfig},
    snapshot::{Compression, SnapshotConfig, SnapshotFormat},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

fn main() {
    let dir = tempfile::tempdir().unwrap();
    let version = Arc::new(AtomicU32::new(1));

    let snap = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "graph".into(),
        key: None,
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };
    let config = GraphStateConfig::new(snap);

    let v = Arc::clone(&version);
    let v2 = Arc::clone(&version);
    let state: GraphState<Graph<u32, ()>> = GraphState::builder(config)
        .key_fn(move || Ok(v.load(Ordering::SeqCst).to_string()))
        .build_fn(move || {
            let ver = v2.load(Ordering::SeqCst);
            let mut g = Graph::new();
            for i in 0..ver {
                g.add_node(i);
            }
            Ok(g)
        })
        .init()
        .unwrap();

    let g1 = state.get().unwrap();
    println!(
        "init: {} nodes, key={}",
        g1.node_count(),
        state.current_key()
    );

    // Same key → cached
    let g2 = state.get_fresh().unwrap();
    println!("get_fresh (same key): same Arc? {}", Arc::ptr_eq(&g1, &g2));

    // Bump version → get_fresh rebuilds
    version.store(5, Ordering::SeqCst);
    let g3 = state.get_fresh().unwrap();
    println!(
        "get_fresh (new key): {} nodes, key={}",
        g3.node_count(),
        state.current_key()
    );

    // Force rebuild
    version.store(10, Ordering::SeqCst);
    let g4 = state.rebuild().unwrap();
    println!(
        "rebuild: {} nodes, key={}",
        g4.node_count(),
        state.current_key()
    );
}
