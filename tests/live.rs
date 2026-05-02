#[cfg(feature = "snapshot")]
#[test]
fn test_config_new() {
    use petgraph_live::{live::GraphStateConfig, snapshot::{Compression, SnapshotConfig, SnapshotFormat}};
    use std::path::PathBuf;
    let snap = SnapshotConfig {
        dir: PathBuf::from("/tmp"), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let cfg = GraphStateConfig::new(snap);
    assert_eq!(cfg.snapshot.name, "g");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_builder_missing_key_fn_errors() {
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat},
    };
    use std::path::PathBuf;
    let snap = SnapshotConfig {
        dir: PathBuf::from("/tmp"), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let cfg = GraphStateConfig::new(snap);
    let result: Result<GraphState<Vec<u32>>, _> = GraphState::builder(cfg)
        .build_fn(|| Ok(vec![]))
        .init();
    assert!(result.is_err());
}

#[cfg(feature = "snapshot")]
#[test]
fn test_builder_missing_build_fn_errors() {
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat},
    };
    use std::path::PathBuf;
    let snap = SnapshotConfig {
        dir: PathBuf::from("/tmp"), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let cfg = GraphStateConfig::new(snap);
    let result: Result<GraphState<Vec<u32>>, _> = GraphState::builder(cfg)
        .key_fn(|| Ok("v1".into()))
        .init();
    assert!(result.is_err());
}

#[cfg(feature = "snapshot")]
#[test]
fn test_builder_key_some_errors() {
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat},
    };
    use std::path::PathBuf;
    let snap = SnapshotConfig {
        dir: PathBuf::from("/tmp"), name: "g".into(), key: Some("k".into()),
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let cfg = GraphStateConfig::new(snap);
    let result: Result<GraphState<Vec<u32>>, _> = GraphState::builder(cfg)
        .key_fn(|| Ok("v1".into()))
        .build_fn(|| Ok(vec![]))
        .init();
    assert!(result.is_err());
}

#[cfg(feature = "snapshot")]
#[test]
fn test_init_cold_start_no_snapshot() {
    use petgraph::Graph;
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat},
    };
    let dir = tempfile::tempdir().unwrap();
    let snap = SnapshotConfig {
        dir: dir.path().to_path_buf(), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let cfg = GraphStateConfig::new(snap);
    let state: GraphState<Graph<u32, ()>> = GraphState::builder(cfg)
        .key_fn(|| Ok("v1".into()))
        .build_fn(|| {
            let mut g = Graph::new();
            for i in 0..5 { g.add_node(i); }
            Ok(g)
        })
        .init()
        .unwrap();
    let g = state.get().unwrap();
    assert_eq!(g.node_count(), 5);
    // snapshot file created
    let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(!entries.is_empty(), "expected snapshot file in dir");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_init_warm_start_from_snapshot() {
    use petgraph::Graph;
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat, save},
    };
    let dir = tempfile::tempdir().unwrap();
    // Pre-save a 3-node graph with key "v1"
    let snap_cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(), name: "g".into(), key: Some("v1".into()),
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let mut pre: Graph<u32, ()> = Graph::new();
    for i in 0..3 { pre.add_node(i); }
    save(&snap_cfg, &pre).unwrap();

    let build_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let bc = std::sync::Arc::clone(&build_called);
    let snap = SnapshotConfig {
        dir: dir.path().to_path_buf(), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
        .key_fn(|| Ok("v1".into()))
        .build_fn(move || {
            bc.store(true, std::sync::atomic::Ordering::SeqCst);
            let mut g = Graph::new();
            for i in 0..9 { g.add_node(i); }
            Ok(g)
        })
        .init()
        .unwrap();

    let g = state.get().unwrap();
    assert_eq!(g.node_count(), 3, "should load from snapshot, not build");
    assert!(!build_called.load(std::sync::atomic::Ordering::SeqCst), "build_fn must not be called on warm start");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_init_snapshot_key_mismatch_rebuilds() {
    use petgraph::Graph;
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat, save},
    };
    let dir = tempfile::tempdir().unwrap();
    // Pre-save with key "v1"
    let snap_cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(), name: "g".into(), key: Some("v1".into()),
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let mut pre: Graph<u32, ()> = Graph::new();
    for i in 0..3 { pre.add_node(i); }
    save(&snap_cfg, &pre).unwrap();

    // init with key "v2" — mismatch, must call build_fn
    let snap = SnapshotConfig {
        dir: dir.path().to_path_buf(), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let build_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let bc = std::sync::Arc::clone(&build_called);
    let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
        .key_fn(|| Ok("v2".into()))
        .build_fn(move || {
            bc.store(true, std::sync::atomic::Ordering::SeqCst);
            let mut g = Graph::new();
            for i in 0..7 { g.add_node(i); }
            Ok(g)
        })
        .init()
        .unwrap();

    assert!(build_called.load(std::sync::atomic::Ordering::SeqCst), "build_fn must be called on key mismatch");
    let g = state.get().unwrap();
    assert_eq!(g.node_count(), 7);
}

#[cfg(feature = "snapshot")]
#[test]
fn test_get_returns_cached() {
    use petgraph::Graph;
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat},
    };
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    let snap = SnapshotConfig {
        dir: dir.path().to_path_buf(), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
        .key_fn(|| Ok("v1".into()))
        .build_fn(|| {
            let mut g = Graph::new();
            g.add_node(1u32);
            Ok(g)
        })
        .init()
        .unwrap();
    let g1 = state.get().unwrap();
    let g2 = state.get().unwrap();
    assert!(Arc::ptr_eq(&g1, &g2), "get() must return same Arc on repeated calls");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_current_key_and_generation() {
    use petgraph::Graph;
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat},
    };
    let dir = tempfile::tempdir().unwrap();
    let snap = SnapshotConfig {
        dir: dir.path().to_path_buf(), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
        .key_fn(|| Ok("sha1abc".into()))
        .build_fn(|| Ok(Graph::new()))
        .init()
        .unwrap();
    assert_eq!(state.current_key(), "sha1abc");
    assert_eq!(state.generation(), 1);
}

#[cfg(feature = "snapshot")]
#[test]
fn test_get_fresh_same_key_no_rebuild() {
    use petgraph::Graph;
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat},
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    let dir = tempfile::tempdir().unwrap();
    let call_count = Arc::new(AtomicU32::new(0));
    let cc = Arc::clone(&call_count);
    let snap = SnapshotConfig {
        dir: dir.path().to_path_buf(), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
        .key_fn(|| Ok("v1".into()))
        .build_fn(move || {
            cc.fetch_add(1, Ordering::SeqCst);
            Ok(Graph::new())
        })
        .init()
        .unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1, "build_fn called once at init");
    let _ = state.get_fresh().unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1, "build_fn not called again: same key");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_get_fresh_new_key_triggers_rebuild() {
    use petgraph::Graph;
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat},
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    let dir = tempfile::tempdir().unwrap();
    let call_count = Arc::new(AtomicU32::new(0));
    let cc = Arc::clone(&call_count);
    let key_counter = Arc::new(AtomicU32::new(0));
    let kc = Arc::clone(&key_counter);
    let snap = SnapshotConfig {
        dir: dir.path().to_path_buf(), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
        .key_fn(move || {
            let n = kc.fetch_add(1, Ordering::SeqCst);
            Ok(format!("v{n}"))
        })
        .build_fn(move || {
            cc.fetch_add(1, Ordering::SeqCst);
            Ok(Graph::new())
        })
        .init()
        .unwrap();
    // init called key_fn once (returns "v0") and build_fn once
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
    assert_eq!(state.current_key(), "v0");
    // get_fresh calls key_fn again → "v1" ≠ "v0" → rebuild
    let _ = state.get_fresh().unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
    assert_eq!(state.current_key(), "v1");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_get_fresh_saves_snapshot() {
    use petgraph::Graph;
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat},
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    let dir = tempfile::tempdir().unwrap();
    let key_counter = Arc::new(AtomicU32::new(0));
    let kc = Arc::clone(&key_counter);
    let snap = SnapshotConfig {
        dir: dir.path().to_path_buf(), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 10,
    };
    let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
        .key_fn(move || {
            let n = kc.fetch_add(1, Ordering::SeqCst);
            Ok(format!("key{n}"))
        })
        .build_fn(|| Ok(Graph::new()))
        .init()
        .unwrap();
    // There's 1 snapshot from init (key0)
    let before: Vec<_> = std::fs::read_dir(dir.path()).unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(before.len(), 1);
    // get_fresh → new key → rebuild → new snapshot
    let _ = state.get_fresh().unwrap();
    let after: Vec<_> = std::fs::read_dir(dir.path()).unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(after.len(), 2, "get_fresh must save new snapshot for new key");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_rebuild_forces_new_graph() {
    use petgraph::Graph;
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat},
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    let dir = tempfile::tempdir().unwrap();
    let counter = Arc::new(AtomicU32::new(0));
    let c1 = Arc::clone(&counter);
    let c2 = Arc::clone(&counter);
    let snap = SnapshotConfig {
        dir: dir.path().to_path_buf(), name: "g".into(), key: None,
        format: SnapshotFormat::Bincode, compression: Compression::None, keep: 3,
    };
    let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
        .key_fn(|| Ok("v1".into()))
        .build_fn(move || {
            let n = c1.fetch_add(1, Ordering::SeqCst);
            let mut g = Graph::new();
            for i in 0..n { g.add_node(i); }
            Ok(g)
        })
        .init()
        .unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1, "init called build_fn once");
    let g1 = state.get().unwrap();
    state.rebuild().unwrap();
    let g2 = state.get().unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 2, "rebuild must call build_fn again");
    assert!(!Arc::ptr_eq(&g1, &g2), "get() after rebuild must return new Arc");
    drop(c2); // silence unused warning
}

#[cfg(feature = "snapshot")]
#[test]
fn test_concurrent_get() {
    use petgraph::Graph;
    use petgraph_live::{
        live::{GraphState, GraphStateConfig},
        snapshot::{Compression, SnapshotConfig, SnapshotFormat},
    };
    use std::sync::Arc as StdArc;
    use std::thread;
    let dir = tempfile::tempdir().unwrap();
    let snap = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: None,
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };
    let state: GraphState<Graph<u32, ()>> = GraphState::builder(GraphStateConfig::new(snap))
        .key_fn(|| Ok("v1".into()))
        .build_fn(|| {
            let mut g = Graph::new();
            for i in 0..1024 { g.add_node(i); }
            Ok(g)
        })
        .init()
        .unwrap();
    let state = StdArc::new(state);
    let handles: Vec<_> = (0..8).map(|_| {
        let s = StdArc::clone(&state);
        thread::spawn(move || {
            for _ in 0..100 {
                let g = s.get().unwrap();
                assert_eq!(g.node_count(), 1024);
            }
        })
    }).collect();
    for h in handles { h.join().unwrap(); }
}
