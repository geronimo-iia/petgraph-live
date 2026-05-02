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
