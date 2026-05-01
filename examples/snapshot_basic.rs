use petgraph::Graph;
use petgraph_live::snapshot::{
    Compression, SnapshotConfig, SnapshotFormat, inspect, list, load_or_build, purge,
};

fn main() {
    let dir = tempfile::tempdir().expect("tempdir");

    let cfg = SnapshotConfig {
        dir: dir.path().to_path_buf(),
        name: "cities".into(),
        key: Some("v1".into()),
        format: SnapshotFormat::Bincode,
        compression: Compression::None,
        keep: 5,
    };

    // load_or_build: build closure called on first run (empty dir)
    let mut build_calls = 0u32;
    let graph: Graph<String, String> = load_or_build(&cfg, || {
        build_calls += 1;
        println!("build closure called — building graph from scratch");
        let mut g: Graph<String, String> = Graph::new();
        let paris = g.add_node("Paris".into());
        let london = g.add_node("London".into());
        let berlin = g.add_node("Berlin".into());
        g.add_edge(paris, london, "eurostar".into());
        g.add_edge(london, berlin, "flight".into());
        Ok(g)
    })
    .expect("load_or_build");

    println!(
        "Graph: {} nodes, {} edges",
        graph.node_count(),
        graph.edge_count()
    );
    assert_eq!(build_calls, 1, "build should have been called once");

    // inspect: read metadata without loading graph
    let meta = inspect(&cfg).expect("inspect").expect("meta present");
    println!(
        "inspect → key={}, nodes={}, created_at={}",
        meta.key, meta.node_count, meta.created_at
    );
    assert_eq!(meta.node_count, 3);

    // save a second snapshot under key v2
    let mut cfg2 = cfg.clone();
    cfg2.key = Some("v2".into());
    let graph2: Graph<String, String> = load_or_build(&cfg2, || {
        let mut g: Graph<String, String> = Graph::new();
        g.add_node("Madrid".into());
        Ok(g)
    })
    .expect("load_or_build v2");
    println!("graph2 nodes: {}", graph2.node_count());

    // list: enumerate snapshots (ascending mtime)
    let cfg_list = SnapshotConfig {
        key: None,
        ..cfg.clone()
    };
    let entries = list(&cfg_list).expect("list");
    println!("list → {} snapshots:", entries.len());
    for (path, m) in &entries {
        println!(
            "  {} (nodes={})",
            path.file_name().unwrap().to_string_lossy(),
            m.node_count
        );
    }
    assert_eq!(entries.len(), 2);

    // purge: delete all snapshots
    let deleted = purge(&cfg_list).expect("purge");
    println!("purge → deleted {} files", deleted);
    assert_eq!(deleted, 2);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);

    println!("snapshot_basic: all assertions passed");
}
