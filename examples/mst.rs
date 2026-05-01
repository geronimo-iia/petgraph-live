use petgraph::data::Element;
use petgraph::graph::UnGraph;
use petgraph_live::mst::{boruvka, kruskal, prim};

fn main() {
    let mut graph: UnGraph<(), f64> = UnGraph::new_undirected();
    let n0 = graph.add_node(());
    let n1 = graph.add_node(());
    let n2 = graph.add_node(());
    let n3 = graph.add_node(());
    let n4 = graph.add_node(());
    let n5 = graph.add_node(());

    graph.add_edge(n0, n1, 10.0);
    graph.add_edge(n1, n3, 4.0);
    graph.add_edge(n2, n3, -5.0);
    graph.add_edge(n2, n0, -2.0);
    graph.add_edge(n2, n5, 6.0);
    graph.add_edge(n5, n4, 2.0);
    graph.add_edge(n3, n4, 10.0);

    println!("--- Prim MST ---");
    let prim_edges = prim(&graph, |e| *e.weight());
    for (a, b) in &prim_edges {
        println!("  ({}, {})", a.index(), b.index());
    }

    println!("\n--- Borůvka MST ---");
    let boruvka_edges = boruvka(&graph, |e| *e.weight());
    for (a, b) in &boruvka_edges {
        println!("  ({}, {})", a.index(), b.index());
    }

    println!("\n--- Kruskal MST (petgraph) ---");
    for elem in kruskal(&graph) {
        match elem {
            Element::Node { weight } => println!("  node: {:?}", weight),
            Element::Edge {
                source,
                target,
                weight,
            } => println!("  edge: ({}, {}) weight={:.1}", source, target, weight),
        }
    }
}
