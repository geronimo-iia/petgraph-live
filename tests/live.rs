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
