/// Demonstrates all three `Compression` variants: None, Zstd, and Lz4.
///
/// Run with:
///   cargo run --example snapshot_compression --features snapshot-zstd,snapshot-lz4
use petgraph::Graph;
use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, inspect, load, save};

fn make_graph() -> Graph<String, String> {
    let mut g: Graph<String, String> = Graph::new();
    let a = g.add_node("Paris".into());
    let b = g.add_node("London".into());
    let c = g.add_node("Berlin".into());
    g.add_edge(a, b, "eurostar".into());
    g.add_edge(b, c, "flight".into());
    g
}

fn roundtrip(dir: &std::path::Path, label: &str, compression: Compression, ext: &str) {
    let cfg = SnapshotConfig {
        dir: dir.to_path_buf(),
        name: label.into(),
        key: Some("v1".into()),
        format: SnapshotFormat::Bincode,
        compression,
        keep: 5,
    };

    let graph = make_graph();
    save(&cfg, &graph).expect("save");

    // Verify the file extension matches the compression variant
    let path = dir.join(format!("{}-v1{}", label, ext));
    assert!(path.exists(), "expected file: {}", path.display());
    println!("  saved → {}", path.file_name().unwrap().to_string_lossy());

    // inspect: metadata only — graph bytes never deserialized
    let meta = inspect(&cfg).expect("inspect").expect("meta present");
    assert_eq!(meta.node_count, 3);
    assert_eq!(meta.compression, cfg.compression.clone());
    println!(
        "  inspect → nodes={}, compression={:?}",
        meta.node_count, meta.compression
    );

    // load: full roundtrip
    let loaded: Graph<String, String> = load(&cfg).expect("load").expect("present");
    assert_eq!(loaded.node_count(), graph.node_count());
    assert_eq!(loaded.edge_count(), graph.edge_count());
    println!("  load   → nodes={} ✓", loaded.node_count());
}

fn main() {
    let dir = tempfile::tempdir().expect("tempdir");

    println!("--- Compression::None (bincode, .snap) ---");
    roundtrip(dir.path(), "none", Compression::None, ".snap");

    #[cfg(feature = "snapshot-zstd")]
    {
        println!("--- Compression::Zstd {{ level: 3 }} (.snap.zst) ---");
        roundtrip(
            dir.path(),
            "zstd",
            Compression::Zstd { level: 3 },
            ".snap.zst",
        );
    }

    #[cfg(feature = "snapshot-lz4")]
    {
        println!("--- Compression::Lz4 (.snap.lz4) ---");
        roundtrip(dir.path(), "lz4", Compression::Lz4, ".snap.lz4");
    }

    println!("snapshot_compression: all assertions passed");
}
