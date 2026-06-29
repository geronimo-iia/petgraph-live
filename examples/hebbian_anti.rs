use petgraph::stable_graph::StableDiGraph;
use petgraph_live::hebbian::{AntiHebbianConfig, anti_hebbian_update, prune};

fn main() {
    let mut graph = StableDiGraph::<&str, f64>::new();
    let cat = graph.add_node("cat");
    let dog = graph.add_node("dog");
    let pet = graph.add_node("pet");

    // Both cat and dog connect to "pet" — they compete for specialization
    graph.add_edge(cat, pet, 0.5);
    graph.add_edge(dog, pet, 0.5);
    graph.add_edge(cat, dog, 0.3);
    graph.add_edge(dog, cat, 0.3);

    let config = AntiHebbianConfig { beta: 0.05 };

    println!("--- Initial graph ---");
    print_edges(&graph);

    // cat and dog co-activate repeatedly — lateral inhibition weakens their link
    for tick in 1..=10 {
        let weakened = anti_hebbian_update(&mut graph, &[(cat, 1.0), (dog, 0.9)], &config);
        if tick % 5 == 0 {
            println!("\n--- After tick {tick} (cat + dog co-active, {weakened} weakened) ---");
            print_edges(&graph);
        }
    }

    // Prune dead connections
    let pruned = prune(&mut graph, 0.01);
    println!("\n--- After prune (removed {pruned} edges) ---");
    print_edges(&graph);
}

fn print_edges(graph: &StableDiGraph<&str, f64>) {
    for idx in graph.edge_indices() {
        let (src, dst) = graph.edge_endpoints(idx).unwrap();
        let w = graph.edge_weight(idx).unwrap();
        println!("  {} -> {} : {:.6}", graph[src], graph[dst], w);
    }
}
