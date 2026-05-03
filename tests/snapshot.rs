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

#[cfg(feature = "snapshot")]
#[test]
fn test_inspect_reads_meta_without_graph() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, inspect, save};
    let dir = tempfile::tempdir().unwrap();
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("sha1".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };
    let mut graph: Graph<u32, ()> = Graph::new();
    graph.add_node(1);
    graph.add_node(2);
    save(&cfg, &graph).unwrap();
    let meta = inspect(&cfg).unwrap().unwrap();
    assert_eq!(meta.node_count, 2);
    assert_eq!(meta.key, "sha1");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_inspect_none_key_most_recent() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, inspect, save};
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
    let g1: Graph<u32, ()> = Graph::new();
    save(&cfg, &g1).unwrap();
    thread::sleep(Duration::from_millis(10));
    cfg.key = Some("v2".into());
    let mut g2: Graph<u32, ()> = Graph::new();
    g2.add_node(1);
    save(&cfg, &g2).unwrap();
    cfg.key = None;
    let meta = inspect(&cfg).unwrap().unwrap();
    assert_eq!(meta.key, "v2");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_list_sorted_oldest_first() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, list, save};
    use std::{thread, time::Duration};
    let dir = tempfile::tempdir().unwrap();
    let cfg_base = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: None,
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 10,
    };
    let g: Graph<u32, ()> = Graph::new();
    for key in &["k1", "k2", "k3"] {
        let mut cfg = cfg_base.clone();
        cfg.key = Some(key.to_string());
        save(&cfg, &g).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    let entries = list(&cfg_base).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].1.key, "k1");
    assert_eq!(entries[2].1.key, "k3");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_purge_deletes_all() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, purge, save};
    let dir = tempfile::tempdir().unwrap();
    let cfg_base = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: None,
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 10,
    };
    let g: Graph<u32, ()> = Graph::new();
    for key in &["a", "b", "c", "d"] {
        let mut cfg = cfg_base.clone();
        cfg.key = Some(key.to_string());
        save(&cfg, &g).unwrap();
    }
    let count = purge(&cfg_base).unwrap();
    assert_eq!(count, 4);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[cfg(feature = "snapshot")]
#[test]
fn test_rotation_save_5_keep_3() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, list, save};
    use std::{thread, time::Duration};
    let dir = tempfile::tempdir().unwrap();
    let cfg_base = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: None,
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };
    let g: Graph<u32, ()> = Graph::new();
    for i in 1u32..=5 {
        let mut cfg = cfg_base.clone();
        cfg.key = Some(format!("key{}", i));
        save(&cfg, &g).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    let entries = list(&cfg_base).unwrap();
    assert_eq!(entries.len(), 3);
    // newest 3 retained: key3, key4, key5
    let keys: Vec<&str> = entries.iter().map(|(_, m)| m.key.as_str()).collect();
    assert!(keys.contains(&"key3"));
    assert!(keys.contains(&"key4"));
    assert!(keys.contains(&"key5"));
}

#[cfg(all(feature = "snapshot", feature = "snapshot-lz4"))]
#[test]
fn test_lz4_roundtrip() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, load, save};
    let dir = tempfile::tempdir().unwrap();
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("lz4_key".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::Lz4,
        keep: 3,
    };
    let mut graph: Graph<u32, ()> = Graph::new();
    for i in 0..50 {
        graph.add_node(i);
    }
    save(&cfg, &graph).unwrap();
    let loaded: Graph<u32, ()> = load(&cfg).unwrap().unwrap();
    assert_eq!(loaded.node_count(), 50);
}

#[cfg(all(feature = "snapshot", feature = "snapshot-lz4"))]
#[test]
fn test_lz4_extension() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, save};
    let dir = tempfile::tempdir().unwrap();
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("lz4ext".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::Lz4,
        keep: 3,
    };
    let graph: Graph<u32, ()> = Graph::new();
    save(&cfg, &graph).unwrap();
    let files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 1);
    assert!(
        files[0]
            .file_name()
            .to_string_lossy()
            .ends_with(".snap.lz4")
    );
}

#[cfg(all(feature = "snapshot", feature = "snapshot-lz4"))]
#[test]
fn test_lz4_inspect() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, inspect, save};
    let dir = tempfile::tempdir().unwrap();
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("lz4meta".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::Lz4,
        keep: 3,
    };
    let mut graph: Graph<u32, ()> = Graph::new();
    graph.add_node(7);
    graph.add_node(8);
    graph.add_node(9);
    save(&cfg, &graph).unwrap();
    let meta = inspect(&cfg).unwrap().unwrap();
    assert_eq!(meta.node_count, 3);
    assert_eq!(meta.key, "lz4meta");
}

// ── TDD: lazy metadata tests (partial-read / MetaOnly) ──────────────────────

#[cfg(feature = "snapshot")]
#[test]
fn test_inspect_partial_read_bincode() {
    // Save a valid graph, then craft a file with valid meta header + garbage
    // graph bytes. inspect() must succeed; load() must fail with ParseError.
    use petgraph::Graph;
    use petgraph_live::snapshot::{
        Compression, SnapshotConfig, SnapshotError, SnapshotFormat, inspect, load, save,
    };
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "mygraph".into(),
        key: Some("goodkey".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 10,
    };
    let mut graph: Graph<String, ()> = Graph::new();
    graph.add_node("node0".into());
    graph.add_node("node1".into());
    save(&cfg, &graph).unwrap();

    // Read the valid file to extract the meta header bytes
    let good_path = dir.path().join("mygraph-goodkey.snap");
    let raw = std::fs::read(&good_path).unwrap();
    let meta_len = u64::from_le_bytes(raw[..8].try_into().unwrap()) as usize;
    let header = &raw[..8 + meta_len];

    // Build a file: valid header + garbage bytes
    let mut bad_bytes = header.to_vec();
    bad_bytes.extend_from_slice(b"THIS IS NOT VALID BINCODE GRAPH DATA !!!!");
    let bad_path = dir.path().join("mygraph-badkey.snap");
    std::fs::write(&bad_path, &bad_bytes).unwrap();

    // inspect() on bad file: must return Ok(Some(meta)) — stops at header
    cfg.key = Some("badkey".into());
    let meta = inspect(&cfg).unwrap().unwrap();
    assert_eq!(meta.node_count, 2);
    assert_eq!(meta.key, "goodkey"); // meta.key reflects what was saved in the header

    // load() on same bad file: must fail — graph bytes are garbage
    let result: Result<Option<Graph<String, ()>>, _> = load(&cfg);
    assert!(
        matches!(result, Err(SnapshotError::ParseError(_))),
        "expected ParseError, got {:?}",
        result
    );
}

#[cfg(feature = "snapshot")]
#[test]
fn test_inspect_json_meta_only() {
    // Save with JSON format. inspect() must return correct node_count and key.
    // Proves MetaOnly deserialization works without calling G::deserialize.
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, inspect, save};
    let dir = tempfile::tempdir().unwrap();
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "jgraph".into(),
        key: Some("jsonkey".into()),
        format: SnapshotFormat::Json,
        compression: Compression::None,
        keep: 10,
    };
    let mut graph: Graph<String, ()> = Graph::new();
    for i in 0..5 {
        graph.add_node(format!("n{}", i));
    }
    save(&cfg, &graph).unwrap();
    let meta = inspect(&cfg).unwrap().unwrap();
    assert_eq!(meta.node_count, 5);
    assert_eq!(meta.key, "jsonkey");
}

#[cfg(feature = "snapshot")]
#[test]
fn test_list_partial_read_bincode() {
    // Save 3 bincode snapshots, list() must return all 3 with correct node counts.
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, list, save};
    use std::{thread, time::Duration};
    let dir = tempfile::tempdir().unwrap();
    let cfg_base = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "lg".into(),
        key: None,
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 10,
    };
    for i in 0u32..3 {
        let mut cfg = cfg_base.clone();
        cfg.key = Some(format!("lkey{}", i));
        let mut g: Graph<String, ()> = Graph::new();
        for j in 0..i {
            g.add_node(format!("n{}", j));
        }
        save(&cfg, &g).unwrap();
        thread::sleep(Duration::from_millis(5));
    }
    let entries = list(&cfg_base).unwrap();
    assert_eq!(entries.len(), 3);
    // node counts: 0, 1, 2
    assert_eq!(entries[0].1.node_count, 0);
    assert_eq!(entries[1].1.node_count, 1);
    assert_eq!(entries[2].1.node_count, 2);
}

#[cfg(feature = "snapshot")]
#[test]
fn test_list_json_meta_only() {
    // Same as test_list_partial_read_bincode but with JSON format.
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, list, save};
    use std::{thread, time::Duration};
    let dir = tempfile::tempdir().unwrap();
    let cfg_base = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "jlg".into(),
        key: None,
        format: SnapshotFormat::Json,
        compression: Compression::None,
        keep: 10,
    };
    for i in 0u32..3 {
        let mut cfg = cfg_base.clone();
        cfg.key = Some(format!("jlkey{}", i));
        let mut g: Graph<String, ()> = Graph::new();
        for j in 0..i {
            g.add_node(format!("n{}", j));
        }
        save(&cfg, &g).unwrap();
        thread::sleep(Duration::from_millis(5));
    }
    let entries = list(&cfg_base).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].1.node_count, 0);
    assert_eq!(entries[1].1.node_count, 1);
    assert_eq!(entries[2].1.node_count, 2);
}

// ────────────────────────────────────────────────────────────────────────────

#[cfg(all(feature = "snapshot", feature = "snapshot-zstd"))]
#[test]
fn test_zstd_roundtrip() {
    use petgraph::Graph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, load, save};
    let dir = tempfile::tempdir().unwrap();
    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "g".into(),
        key: Some("zstd_key".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::Zstd { level: 3 },
        keep: 3,
    };
    let mut graph: Graph<u32, ()> = Graph::new();
    for i in 0..100 {
        graph.add_node(i);
    }
    save(&cfg, &graph).unwrap();
    // verify file ends with .snap.zst
    let files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 1);
    assert!(
        files[0]
            .file_name()
            .to_string_lossy()
            .ends_with(".snap.zst")
    );
    let loaded: Graph<u32, ()> = load(&cfg).unwrap().unwrap();
    assert_eq!(loaded.node_count(), 100);
}

#[cfg(feature = "snapshot")]
#[test]
fn list_snapshot_files_missing_dir_returns_empty() {
    use petgraph_live::snapshot::rotation::list_snapshot_files;
    use std::path::Path;

    let result = list_snapshot_files(Path::new("/tmp/petgraph_live_nonexistent_xyz"), "mygraph");
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
    assert!(result.unwrap().is_empty());
}

#[cfg(feature = "snapshot")]
#[test]
fn save_creates_missing_directory() {
    use petgraph::graph::DiGraph;
    use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, save};

    let dir = std::env::temp_dir().join(format!("petgraph_live_autodir_{}", std::process::id()));
    assert!(!dir.exists(), "dir should not exist before save");

    let cfg = SnapshotConfig {
        dir: dir.clone(),
        name: "g".into(),
        key: Some("testkey".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 3,
    };

    let graph: DiGraph<(), ()> = DiGraph::new();
    let result = save(&cfg, &graph);

    assert!(result.is_ok(), "save() failed: {:?}", result);
    assert!(dir.exists(), "save() did not create the directory");

    let _ = std::fs::remove_dir_all(&dir);
}
