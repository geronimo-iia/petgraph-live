use petgraph::stable_graph::StableDiGraph;
use petgraph_live::hebbian::{BcmConfig, BcmState, bcm_update};

fn main() {
    let mut graph = StableDiGraph::<&str, f64>::new();
    let input = graph.add_node("input");
    let neuron_a = graph.add_node("neuron_a");
    let neuron_b = graph.add_node("neuron_b");

    graph.add_edge(input, neuron_a, 0.5);
    graph.add_edge(input, neuron_b, 0.5);

    let config = BcmConfig {
        learning_rate: 0.05,
        threshold_rate: 0.1,
    };
    let mut state = BcmState::new(3, 0.5);

    println!("--- BCM: homeostatic plasticity ---");
    println!("Initial (threshold=0.5 for all):");
    print_state(&graph, &state);

    // neuron_a is highly active → threshold rises → harder to strengthen
    for epoch in 1..=20 {
        bcm_update(
            &mut graph,
            &[(input, 0.9), (neuron_a, 0.95)],
            &mut state,
            &config,
        );
        // neuron_b gets low activation → threshold drops → easier to strengthen
        bcm_update(
            &mut graph,
            &[(input, 0.9), (neuron_b, 0.3)],
            &mut state,
            &config,
        );
        if epoch % 5 == 0 {
            println!("\nAfter epoch {epoch}:");
            print_state(&graph, &state);
        }
    }
}

fn print_state(graph: &StableDiGraph<&str, f64>, state: &BcmState) {
    for idx in graph.edge_indices() {
        let (src, dst) = graph.edge_endpoints(idx).unwrap();
        let w = graph.edge_weight(idx).unwrap();
        println!("  {} -> {} : {:.6}", graph[src], graph[dst], w);
    }
    for (i, &theta) in state.thresholds.iter().enumerate() {
        println!("  θ[{i}] = {theta:.6}");
    }
}
