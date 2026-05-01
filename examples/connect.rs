use petgraph::graph::UnGraph;
use petgraph_live::connect::{articulation_points, find_bridges};

fn main() {
    // 0----1    4
    //      | __/|
    // 5----2/---3
    let mut g = UnGraph::new_undirected();
    let n0 = g.add_node(());
    let n1 = g.add_node(());
    let n2 = g.add_node(());
    let n3 = g.add_node(());
    let n4 = g.add_node(());
    let n5 = g.add_node(());
    g.add_edge(n0, n1, ());
    g.add_edge(n1, n2, ());
    g.add_edge(n2, n3, ());
    g.add_edge(n3, n4, ());
    g.add_edge(n2, n4, ());
    g.add_edge(n5, n2, ());

    println!("Graph: 0--1--2--3--4, 2--4, 5--2");

    let aps = articulation_points(&g);
    println!(
        "Articulation points: {:?}",
        aps.iter().map(|n| n.index()).collect::<Vec<_>>()
    );

    let bridges = find_bridges(&g);
    println!(
        "Bridges: {:?}",
        bridges
            .iter()
            .map(|(a, b)| (a.index(), b.index()))
            .collect::<Vec<_>>()
    );
}
