#[cfg(feature = "snapshot")]
#[test]
fn test_config_defaults() {
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};
    use std::path::PathBuf;
    let cfg = SnapshotConfig {
        dir: PathBuf::from("/tmp/test-snapshots"),
        name: "mygraph".to_string(),
        key: Some("abc123".to_string()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };
    assert_eq!(cfg.keep, 3);
    assert_eq!(cfg.key.as_deref(), Some("abc123"));
}

#[cfg(feature = "snapshot")]
#[test]
fn test_sanitize_key() {
    use petgraph_live::snapshot::sanitize_key;
    assert_eq!(sanitize_key("abc123"), Ok("abc123".to_string()));
    assert_eq!(sanitize_key("a/b c"), Ok("a_b_c".to_string()));
    assert!(sanitize_key("   ").is_err());
}

#[cfg(feature = "snapshot")]
#[test]
fn test_config_serde_roundtrip() {
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};
    use std::path::PathBuf;
    let cfg = SnapshotConfig {
        dir: PathBuf::from("/tmp"),
        name: "g".into(),
        key: Some("should-be-skipped".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 5,
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let back: SnapshotConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, "g");
    assert_eq!(back.keep, 5);
    assert_eq!(back.key, None);
}

#[cfg(feature = "snapshot")]
#[test]
fn test_meta_new() {
    use petgraph_live::snapshot::{Compression, SnapshotFormat, SnapshotMeta};
    let meta = SnapshotMeta::new("sha123", SnapshotFormat::Bincode, Compression::None, 10, 5);
    assert_eq!(meta.node_count, 10);
    assert_eq!(meta.edge_count, 5);
    assert_eq!(meta.key, "sha123");
    assert!(!meta.petgraph_live_version.is_empty());
}

#[cfg(feature = "snapshot")]
#[test]
fn test_error_display() {
    use petgraph_live::snapshot::SnapshotError;
    let e = SnapshotError::KeyNotFound {
        key: "sha_abc".into(),
    };
    assert!(e.to_string().contains("sha_abc"));
    let e2 = SnapshotError::InvalidKey("   ".into());
    assert!(e2.to_string().contains("invalid key"));
    assert!(
        SnapshotError::NoSnapshotFound
            .to_string()
            .contains("no snapshot")
    );
}

#[cfg(feature = "snapshot")]
#[test]
fn test_rotation_keep_3() {
    use petgraph_live::snapshot::rotation::{keep_n, list_snapshot_files};
    use std::{
        fs,
        time::{Duration, SystemTime},
    };
    let dir = tempfile::tempdir().unwrap();
    for i in 1u64..=5 {
        let fname = format!("mygraph-key{}.snap", i);
        let path = dir.path().join(&fname);
        fs::write(&path, b"data").unwrap();
        let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(i * 1000);
        filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(mtime)).unwrap();
    }
    let files = list_snapshot_files(dir.path(), "mygraph").unwrap();
    assert_eq!(files.len(), 5);
    keep_n(dir.path(), "mygraph", 3).unwrap();
    let remaining = list_snapshot_files(dir.path(), "mygraph").unwrap();
    assert_eq!(remaining.len(), 3);
    for i in 3u64..=5 {
        assert!(dir.path().join(format!("mygraph-key{}.snap", i)).exists());
    }
}

#[cfg(feature = "snapshot")]
#[test]
fn test_save_creates_file() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, save};
    let dir = tempfile::tempdir().unwrap();
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("sha1abc".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };
    let mut graph: Graph<(), ()> = Graph::new();
    graph.add_node(());
    save(&cfg, &graph).unwrap();
    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1);
    let name = entries[0].file_name().to_string_lossy().into_owned();
    assert_eq!(name, "g-sha1abc.snap");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_save_same_key_idempotent() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, save};
    let dir = tempfile::tempdir().unwrap();
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("v1".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };
    let graph: Graph<(), ()> = Graph::new();
    save(&cfg, &graph).unwrap();
    save(&cfg, &graph).unwrap();
    let count = std::fs::read_dir(dir.path()).unwrap().count();
    assert_eq!(count, 1);
}

#[cfg(feature = "snapshot")]
#[test]
fn test_save_load_roundtrip_bincode() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, load, save};
    let dir = tempfile::tempdir().unwrap();
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("v1".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };
    let mut graph: Graph<String, ()> = Graph::new();
    graph.add_node("a".into());
    graph.add_node("b".into());
    graph.add_node("c".into());
    save(&cfg, &graph).unwrap();
    let loaded: Graph<String, ()> = load(&cfg).unwrap().unwrap();
    assert_eq!(loaded.node_count(), 3);
}

#[cfg(feature = "snapshot")]
#[test]
fn test_save_load_roundtrip_json() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, load, save};
    let dir = tempfile::tempdir().unwrap();
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("v1".into()),
        format: SnapshotFormat::Json,
        compression: Compression::None,
        keep: 3,
    };
    let mut graph: Graph<String, ()> = Graph::new();
    graph.add_node("a".to_string());
    graph.add_node("b".to_string());
    graph.add_node("c".to_string());
    save(&cfg, &graph).unwrap();
    let loaded: Graph<String, ()> = load(&cfg).unwrap().unwrap();
    assert_eq!(loaded.node_count(), 3);
}

#[cfg(feature = "snapshot")]
#[test]
fn test_load_key_not_found() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{
        Compression, SnapshotConfig, SnapshotError, SnapshotFormat, load, save,
    };
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("v1".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };
    let graph: Graph<(), ()> = Graph::new();
    save(&cfg, &graph).unwrap();
    cfg.key = Some("v2".into());
    let result: Result<Option<Graph<(), ()>>, _> = load(&cfg);
    assert!(matches!(result, Err(SnapshotError::KeyNotFound { .. })));
}

#[cfg(feature = "snapshot")]
#[test]
fn test_load_no_snapshot_returns_none() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{
        Compression, SnapshotConfig, SnapshotError, SnapshotFormat, load,
    };
    let dir = tempfile::tempdir().unwrap();
    // key=Some, empty dir → KeyNotFound
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("v1".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };
    let result: Result<Option<Graph<(), ()>>, _> = load(&cfg);
    assert!(matches!(result, Err(SnapshotError::KeyNotFound { .. })));
    // key=None, empty dir → Ok(None)
    let mut cfg2 = cfg.clone();
    cfg2.key = None;
    let result2: Result<Option<Graph<(), ()>>, _> = load(&cfg2);
    assert!(matches!(result2, Ok(None)));
}

#[cfg(feature = "snapshot")]
#[test]
fn test_load_none_key_returns_most_recent() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, load, save};
    use std::{thread, time::Duration};
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("v1".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 10,
    };
    let mut g1: Graph<u32, ()> = Graph::new();
    g1.add_node(1);
    save(&cfg, &g1).unwrap();
    thread::sleep(Duration::from_millis(10));
    cfg.key = Some("v2".into());
    let mut g2: Graph<u32, ()> = Graph::new();
    g2.add_node(1);
    g2.add_node(2);
    save(&cfg, &g2).unwrap();
    cfg.key = None;
    let loaded: Graph<u32, ()> = load(&cfg).unwrap().unwrap();
    assert_eq!(loaded.node_count(), 2);
}

#[cfg(feature = "snapshot")]
#[test]
fn test_load_or_build_falls_back_on_empty() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{
        Compression, SnapshotConfig, SnapshotFormat, load, load_or_build,
    };
    let dir = tempfile::tempdir().unwrap();
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("v1".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };
    let mut called = false;
    let g: Graph<u32, ()> = load_or_build(&cfg, || {
        called = true;
        let mut g = Graph::new();
        g.add_node(42u32);
        Ok(g)
    })
    .unwrap();
    assert!(called);
    assert_eq!(g.node_count(), 1);
    // file saved → can load now
    let loaded: Graph<u32, ()> = load(&cfg).unwrap().unwrap();
    assert_eq!(loaded.node_count(), 1);
}

#[cfg(feature = "snapshot")]
#[test]
fn test_load_or_build_falls_back_on_key_not_found() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{
        Compression, SnapshotConfig, SnapshotFormat, load, load_or_build, save,
    };
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("v1".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 10,
    };
    let g1: Graph<u32, ()> = Graph::new();
    save(&cfg, &g1).unwrap();
    cfg.key = Some("v2".into());
    let mut build_called = false;
    let _g: Graph<u32, ()> = load_or_build(&cfg, || {
        build_called = true;
        let mut g = Graph::new();
        g.add_node(99u32);
        Ok(g)
    })
    .unwrap();
    assert!(build_called);
    // v1 still present
    cfg.key = Some("v1".into());
    let v1: Graph<u32, ()> = load(&cfg).unwrap().unwrap();
    assert_eq!(v1.node_count(), 0);
}
