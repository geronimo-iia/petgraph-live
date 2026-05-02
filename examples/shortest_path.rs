use petgraph::Graph;
use petgraph_live::shortest_path::{distance_map, floyd_warshall, seidel, shortest_distances};

fn main() {
    let graph = Graph::<(), f32>::from_edges([
        (0, 1, 2.0),
        (1, 2, 10.0),
        (1, 3, -5.0),
        (3, 2, 2.0),
        (2, 3, 20.0),
    ]);

    println!("--- shortest_distances (BFS, unweighted) from node 0 ---");
    println!("{:?}", shortest_distances(&graph, 0.into()));

    println!("\n--- floyd_warshall ---");
    match floyd_warshall(&graph, |e| *e.weight()) {
        Ok(dist) => {
            for row in &dist {
                println!("{:?}", row);
            }
        }
        Err(_) => println!("Negative cycle detected"),
    }

    println!("\n--- distance_map (node 0 → node 2) ---");
    match distance_map(&graph, |e| *e.weight()) {
        Ok(dm) => {
            let d = dm[&(0.into(), 2.into())];
            println!("dist(0, 2) = {}", d);
        }
        Err(_) => println!("Negative cycle"),
    }

    println!("\n--- seidel (undirected, unweighted) ---");
    use petgraph::graph::UnGraph;
    let mut ug = UnGraph::<(), ()>::new_undirected();
    for _ in 0..6 {
        ug.add_node(());
    }
    ug.extend_with_edges([(0, 1), (0, 3), (1, 2), (1, 5), (2, 4), (3, 4), (4, 5)]);
    let dist = seidel(&ug);
    for row in &dist {
        println!("{:?}", row);
    }
}
