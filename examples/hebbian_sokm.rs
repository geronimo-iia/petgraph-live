use petgraph::stable_graph::StableDiGraph;
use petgraph_live::hebbian::{sokm_tick, SokmConfig};

fn main() {
    // Build a small knowledge graph with weighted edges
    let mut graph = StableDiGraph::<&str, f64>::new();
    let alice = graph.add_node("alice");
    let bob = graph.add_node("bob");
    let carol = graph.add_node("carol");
    let dave = graph.add_node("dave");

    graph.add_edge(alice, bob, 0.5);
    graph.add_edge(bob, carol, 0.3);
    graph.add_edge(carol, dave, 0.1);
    graph.add_edge(alice, carol, 0.2);

    let config = SokmConfig::default();

    println!("--- Initial graph ({} edges) ---", graph.edge_count());
    print_edges(&graph);

    // Tick 1: alice and bob co-activate
    let report = sokm_tick(&mut graph, &[(alice, 1.0), (bob, 0.9)], &config);
    println!("\n--- After tick 1 (alice + bob active) ---");
    println!("  {report:?}");
    print_edges(&graph);

    // Tick 2: bob and carol co-activate
    let report = sokm_tick(&mut graph, &[(bob, 0.8), (carol, 0.7)], &config);
    println!("\n--- After tick 2 (bob + carol active) ---");
    println!("  {report:?}");
    print_edges(&graph);

    // Tick 3-10: no activation — pure decay + prune
    for tick in 3..=10 {
        let report = sokm_tick(&mut graph, &[], &config);
        if report.pruned > 0 {
            println!("\n--- Tick {tick}: pruned {} edges ---", report.pruned);
            print_edges(&graph);
        }
    }

    println!("\n--- Final graph ({} edges) ---", graph.edge_count());
    print_edges(&graph);
}

fn print_edges(graph: &StableDiGraph<&str, f64>) {
    for idx in graph.edge_indices() {
        let (src, dst) = graph.edge_endpoints(idx).unwrap();
        let w = graph.edge_weight(idx).unwrap();
        println!(
            "  {} -> {} : {:.6}",
            graph[src], graph[dst], w
        );
    }
}
