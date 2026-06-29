use petgraph::stable_graph::StableDiGraph;
use petgraph_live::hebbian::{OjaConfig, oja_update};

fn main() {
    let mut graph = StableDiGraph::<&str, f64>::new();
    let x1 = graph.add_node("x1");
    let x2 = graph.add_node("x2");
    let y = graph.add_node("y");

    // Two inputs feeding into one output
    graph.add_edge(x1, y, 0.3);
    graph.add_edge(x2, y, 0.7);

    let config = OjaConfig { learning_rate: 0.05 };

    println!("--- Oja's rule: weights converge to principal component ---");
    println!("Initial:");
    print_edges(&graph);

    // Repeated activations — weights self-normalize
    for epoch in 1..=50 {
        oja_update(&mut graph, &[(x1, 0.6), (x2, 0.8)], &[(y, 1.0)], &config);
        if epoch % 10 == 0 {
            println!("\nAfter epoch {epoch}:");
            print_edges(&graph);
        }
    }
}

fn print_edges(graph: &StableDiGraph<&str, f64>) {
    for idx in graph.edge_indices() {
        let (src, dst) = graph.edge_endpoints(idx).unwrap();
        let w = graph.edge_weight(idx).unwrap();
        println!("  {} -> {} : {:.6}", graph[src], graph[dst], w);
    }
}
