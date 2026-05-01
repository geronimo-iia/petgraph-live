use petgraph::graph::UnGraph;
use petgraph_live::metrics::{
    center, diameter, eccentricity, girth, periphery, radius, weighted_diameter, weighted_radius,
};

fn main() {
    // Undirected path graph: 0--1--2--3--4
    let mut g = UnGraph::<(), f32>::new_undirected();
    let n0 = g.add_node(());
    let n1 = g.add_node(());
    let n2 = g.add_node(());
    let n3 = g.add_node(());
    let n4 = g.add_node(());
    g.add_edge(n0, n1, 1.0);
    g.add_edge(n1, n2, 2.0);
    g.add_edge(n2, n3, 1.0);
    g.add_edge(n3, n4, 2.0);

    println!("--- Unweighted metrics (path graph 0--1--2--3--4) ---");
    for i in 0..5 {
        println!("  eccentricity({}): {}", i, eccentricity(&g, i.into()));
    }
    println!("  radius:   {:?}", radius(&g));
    println!("  diameter: {:?}", diameter(&g));
    println!("  center:   {:?}", center(&g));
    println!("  periphery:{:?}", periphery(&g));
    println!("  girth:    {:?}", girth(&g));

    // Add an edge to create a triangle 0-1-2-0
    g.add_edge(n0, n2, 3.0);
    println!("\n--- After adding edge 0--2 (creates triangle) ---");
    println!("  girth: {:?}", girth(&g));

    println!("\n--- Weighted metrics ---");
    println!(
        "  weighted_radius:   {:?}",
        weighted_radius(&g, |e| *e.weight())
    );
    println!(
        "  weighted_diameter: {:?}",
        weighted_diameter(&g, |e| *e.weight())
    );
}
