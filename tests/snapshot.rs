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
