use criterion::{Criterion, criterion_group, criterion_main};
use petgraph::stable_graph::StableDiGraph;
use petgraph_live::hebbian::{
    AntiHebbianConfig, BcmConfig, BcmState, OjaConfig, SokmConfig, StdpConfig, anti_hebbian_update,
    bcm_update, decay, oja_update, prune, sokm_tick, stdp_update, strengthen,
};

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
    c.bench_function("decay_5k_edges", |b| {
        b.iter(|| decay(&mut g, 0.95));
    });
}

fn bench_decay_100k(c: &mut Criterion) {
    let mut g = build_graph(20_000); // 20K nodes × 5 = 100K edges
    c.bench_function("decay_100k_edges", |b| {
        b.iter(|| decay(&mut g, 0.95));
    });
}

fn bench_strengthen(c: &mut Criterion) {
    let mut g = build_graph(1000);
    let nodes: Vec<_> = g.node_indices().take(20).map(|n| (n, 0.8)).collect();
    let config = SokmConfig::default();
    c.bench_function("strengthen_20_activated_5k_edges", |b| {
        b.iter(|| strengthen(&mut g, &nodes, &config));
    });
}

fn bench_strengthen_10(c: &mut Criterion) {
    let mut g = build_graph(1000);
    let nodes: Vec<_> = g.node_indices().take(10).map(|n| (n, 0.8)).collect();
    let config = SokmConfig::default();
    c.bench_function("strengthen_10_activated_5k_edges", |b| {
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

fn bench_stdp(c: &mut Criterion) {
    let mut g = build_graph(1000);
    let activations: Vec<_> = g
        .node_indices()
        .take(20)
        .enumerate()
        .map(|(i, n)| (n, 0.8, i as u64))
        .collect();
    let config = StdpConfig::default();
    c.bench_function("stdp_20_activated_1000_nodes", |b| {
        b.iter(|| stdp_update(&mut g, &activations, &config));
    });
}

fn bench_anti_hebbian(c: &mut Criterion) {
    let mut g = build_graph(1000);
    let nodes: Vec<_> = g.node_indices().take(20).map(|n| (n, 0.8)).collect();
    let config = AntiHebbianConfig::default();
    c.bench_function("anti_hebbian_20_activated_5k_edges", |b| {
        b.iter(|| anti_hebbian_update(&mut g, &nodes, &config));
    });
}

fn bench_oja(c: &mut Criterion) {
    let mut g = build_graph(1000);
    let pre: Vec<_> = g.node_indices().take(10).map(|n| (n, 0.8)).collect();
    let post: Vec<_> = g
        .node_indices()
        .skip(5)
        .take(10)
        .map(|n| (n, 0.7))
        .collect();
    let config = OjaConfig::default();
    c.bench_function("oja_10x10_activated_5k_edges", |b| {
        b.iter(|| oja_update(&mut g, &pre, &post, &config));
    });
}

fn bench_bcm(c: &mut Criterion) {
    let mut g = build_graph(1000);
    let nodes: Vec<_> = g.node_indices().take(20).map(|n| (n, 0.8)).collect();
    let mut state = BcmState::new(1000, 0.5);
    let config = BcmConfig::default();
    c.bench_function("bcm_20_activated_5k_edges", |b| {
        b.iter(|| bcm_update(&mut g, &nodes, &mut state, &config));
    });
}

criterion_group!(
    benches,
    bench_decay,
    bench_decay_100k,
    bench_strengthen,
    bench_strengthen_10,
    bench_prune,
    bench_sokm_tick,
    bench_stdp,
    bench_anti_hebbian,
    bench_oja,
    bench_bcm
);
criterion_main!(benches);
