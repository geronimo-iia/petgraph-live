use petgraph::stable_graph::StableDiGraph;
use petgraph_live::hebbian::{StdpConfig, stdp_update};

fn main() {
    let mut graph = StableDiGraph::<&str, f64>::new();
    let sensor = graph.add_node("sensor");
    let hidden = graph.add_node("hidden");
    let output = graph.add_node("output");

    graph.add_edge(sensor, hidden, 0.5);
    graph.add_edge(hidden, output, 0.5);
    graph.add_edge(sensor, output, 0.3);

    let config = StdpConfig::default();

    println!("--- Initial graph ---");
    print_edges(&graph);

    // Causal chain: sensor fires first, then hidden, then output
    let activations = vec![(sensor, 1.0, 1), (hidden, 0.9, 3), (output, 0.8, 5)];
    let modified = stdp_update(&mut graph, &activations, &config);
    println!("\n--- After causal chain (sensor@1 → hidden@3 → output@5) ---");
    println!("  Edges modified: {modified}");
    print_edges(&graph);

    // Anti-causal: output fires before sensor (prediction error)
    let activations = vec![(output, 1.0, 10), (sensor, 0.9, 12)];
    let modified = stdp_update(&mut graph, &activations, &config);
    println!("\n--- After anti-causal (output@10 → sensor@12) ---");
    println!("  Edges modified: {modified}");
    print_edges(&graph);

    // Simultaneous firing: no effect
    let activations = vec![(sensor, 1.0, 20), (hidden, 1.0, 20)];
    let modified = stdp_update(&mut graph, &activations, &config);
    println!("\n--- After simultaneous firing (same tick) ---");
    println!("  Edges modified: {modified}");
    print_edges(&graph);
}

fn print_edges(graph: &StableDiGraph<&str, f64>) {
    for idx in graph.edge_indices() {
        let (src, dst) = graph.edge_endpoints(idx).unwrap();
        let w = graph.edge_weight(idx).unwrap();
        println!("  {} -> {} : {:.6}", graph[src], graph[dst], w);
    }
}
