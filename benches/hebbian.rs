use criterion::{Criterion, criterion_group, criterion_main};
use petgraph::stable_graph::StableDiGraph;
use petgraph_live::hebbian::{SokmConfig, decay, prune, sokm_tick, strengthen};

fn build_graph(n: usize) -> StableDiGraph<(), f64> {
    let mut g = StableDiGraph::new();
    let nodes: Vec<_> = (0..n).map(|_| g.add_node(())).collect();
    // Create a dense-ish graph: each node connects to next 5
    for i in 0..n {
        for j in 1..=5 {
            let target = (i + j) % n;
            g.add_edge(nodes[i], nodes[target], 0.5);
        }
    }
    g
}

fn bench_decay(c: &mut Criterion) {
    let mut g = build_graph(1000);
    c.bench_function("decay_1000_nodes", |b| {
        b.iter(|| decay(&mut g, 0.95));
    });
}

fn bench_strengthen(c: &mut Criterion) {
    let mut g = build_graph(1000);
    let nodes: Vec<_> = g.node_indices().take(20).map(|n| (n, 0.8)).collect();
    let config = SokmConfig::default();
    c.bench_function("strengthen_20_activated_1000_nodes", |b| {
        b.iter(|| strengthen(&mut g, &nodes, &config));
    });
}

fn bench_prune(c: &mut Criterion) {
    let mut g = build_graph(1000);
    c.bench_function("prune_1000_nodes", |b| {
        b.iter(|| prune(&mut g, 0.001));
    });
}

fn bench_sokm_tick(c: &mut Criterion) {
    let mut g = build_graph(1000);
    let nodes: Vec<_> = g.node_indices().take(20).map(|n| (n, 0.8)).collect();
    let config = SokmConfig::default();
    c.bench_function("sokm_tick_20_activated_1000_nodes", |b| {
        b.iter(|| sokm_tick(&mut g, &nodes, &config));
    });
}

criterion_group!(benches, bench_decay, bench_strengthen, bench_prune, bench_sokm_tick);
criterion_main!(benches);
