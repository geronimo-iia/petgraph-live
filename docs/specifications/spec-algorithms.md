---
title: "Algorithm modules"
summary: "Graph algorithms ported from graphalgs (MIT) — metrics, connectivity, shortest paths, MST — all on petgraph 0.8 with no heavy deps."
read_when:
  - Porting or modifying metrics, connect, shortest_path, or mst
  - Understanding deviations from graphalgs return types and signatures
  - Writing tests for algorithm modules
status: implemented
last_updated: "2026-05-02"
---

# Specification: algorithm modules

**Modules:** `metrics`, `connect`, `shortest_path`, `mst`

**License notice:** All implementations are ported from
[graphalgs](https://github.com/starovoid/graphalgs) (MIT). Original author
credited in each module's top-level doc comment.

## Module: `metrics`

Distance-based graph characteristics. All unweighted functions use BFS
(O(n·(n+e))). All weighted functions use Floyd-Warshall (O(n³)).

### Return value conventions

- `None` on empty graph, disconnected graph, or negative cycle.
- Unweighted variants return `f32` (except `girth` — see below).
- Weighted variants return `Option<K>` where `K: FloatMeasure`.
- All weighted variants accept an `edge_cost: F` closure — consistent interface,
  no `G::EdgeWeight: FloatMeasure` constraint.

### Deviations from graphalgs

| Function                | graphalgs                            | petgraph-live                    |
| ----------------------- | ------------------------------------ | -------------------------------- |
| `girth`                 | `f32`, `INFINITY` if acyclic         | `Option<u32>`, `None` if acyclic |
| `weighted_eccentricity` | no closure, bound on `G::EdgeWeight` | takes `edge_cost: F` closure     |

### Public API

```rust
// Unweighted
pub fn eccentricity<G>(graph: G, node: G::NodeId) -> f32
pub fn radius<G>(graph: G) -> Option<f32>
pub fn diameter<G>(graph: G) -> Option<f32>
pub fn center<G>(graph: G) -> Vec<G::NodeId>
pub fn periphery<G>(graph: G) -> Vec<G::NodeId>
pub fn girth<G>(graph: G) -> Option<u32>

// Weighted
pub fn weighted_eccentricity<G, F, K>(graph: G, node: G::NodeId, edge_cost: F) -> Option<K>
pub fn weighted_radius<G, F, K>(graph: G, edge_cost: F) -> Option<K>
pub fn weighted_diameter<G, F, K>(graph: G, edge_cost: F) -> Option<K>
pub fn weighted_center<G, F, K>(graph: G, edge_cost: F) -> Vec<G::NodeId>
pub fn weighted_periphery<G, F, K>(graph: G, edge_cost: F) -> Vec<G::NodeId>
```

Where `K: FloatMeasure + PartialOrd` and `F: FnMut(G::EdgeRef) -> K`.

## Module: `connect`

Structural connectivity analysis. Undirected graphs only (articulation points
and bridges are undirected concepts). DFS-based, O(n+e).

No trait bound enforces undirectedness — caller responsibility, documented.

### Public API

```rust
pub fn articulation_points<G>(graph: G) -> Vec<G::NodeId>
pub fn find_bridges<G>(graph: G) -> Vec<(G::NodeId, G::NodeId)>
```

## Module: `shortest_path`

Own implementations plus petgraph re-exports.

### Public API

```rust
// Own implementations
pub fn shortest_distances<G>(graph: G, start: G::NodeId) -> Vec<f32>
pub fn floyd_warshall<G, F, K>(graph: G, edge_cost: F) -> Result<Vec<Vec<K>>, NegativeCycle>
pub fn distance_map<G, F, K>(graph: G, edge_cost: F) -> Result<HashMap<(G::NodeId, G::NodeId), K>, NegativeCycle>
pub fn seidel<G>(graph: G) -> Vec<Vec<u32>>
pub(crate) fn apd(a: &[Vec<u32>]) -> Vec<Vec<u32>>

// Re-exports from petgraph
pub use petgraph::algo::{
    astar, bellman_ford, dijkstra, johnson, k_shortest_path, spfa, NegativeCycle,
};
```

`shortest_distances`: BFS from `start`, returns `Vec<f32>` indexed by
`NodeIndexable::to_index`. Unreachable nodes → `f32::INFINITY`.

`seidel`: unweighted APSP for undirected graphs (Seidel/APD algorithm),
O(n^ω log n). No nalgebra — matrix operations over `Vec<Vec<u32>>`.

## Module: `hebbian`

Dynamic graph learning — Hebbian algorithms that modify edge weights based on
node co-activation patterns. Pure computation, zero additional dependencies.

Operates on `StableGraph<N, f64, Ty>` where `Ty: EdgeType` (both directed and
undirected). Requires `StableGraph` because `prune` removes edges during
iteration.

### Design principles

- Free functions (same style as `metrics`, `connect`)
- Config as plain `#[derive(Debug, Clone, Copy)]` structs with `Default`
- Operations return counts/diagnostics (`HebbianReport`), not `Result`
- For undirected graphs, `strengthen` processes each unordered pair once

### Public API

```rust
/// Configuration for SOKM dynamics.
pub struct SokmConfig {
    pub decay_factor: f64,    // default: 0.95
    pub delta: f64,           // default: 0.02
    pub min_weight: f64,      // default: 0.001
    pub formula: StrengthFormula,
}

pub enum StrengthFormula { Product, Min, Average }

pub struct HebbianReport {
    pub decayed: usize,
    pub strengthened: usize,
    pub pruned: usize,
}

pub fn decay<N, Ty: EdgeType>(graph: &mut StableGraph<N, f64, Ty>, factor: f64) -> usize
pub fn strengthen<N, Ty: EdgeType>(graph: &mut StableGraph<N, f64, Ty>, activated: &[(NodeIndex, f64)], config: &SokmConfig) -> usize
pub fn prune<N, Ty: EdgeType>(graph: &mut StableGraph<N, f64, Ty>, threshold: f64) -> usize
pub fn sokm_tick<N, Ty: EdgeType>(graph: &mut StableGraph<N, f64, Ty>, activated: &[(NodeIndex, f64)], config: &SokmConfig) -> HebbianReport
```

### SOKM algorithm

Each tick executes three phases in order:
1. **Decay** — multiply all edge weights by `decay_factor`
2. **Strengthen** — for each co-activated pair, increment edge weight by formula
3. **Prune** — remove edges with weight below `min_weight`

### STDP algorithm

Spiking-Timing Dependent Plasticity. For each directed edge (pre → post)
where both endpoints fired:
- Causal (pre fires before post): Δw = +a_plus × exp(-Δt / tau_plus)
- Anti-causal (post fires before pre): Δw = -a_minus × exp(-Δt / tau_minus)

```rust
pub struct StdpConfig {
    pub a_plus: f64,     // default: 0.01
    pub a_minus: f64,    // default: 0.005
    pub tau_plus: f64,   // default: 5.0
    pub tau_minus: f64,  // default: 5.0
}

pub type TimedActivation = (NodeIndex, f64, u64);

pub fn stdp_update<N, Ty: EdgeType>(graph: &mut StableGraph<N, f64, Ty>, activations: &[TimedActivation], config: &StdpConfig) -> usize
```

### Anti-Hebbian algorithm

Lateral inhibition — weakens edges between co-activated nodes. Opposite of
SOKM strengthen. Forces specialization.

```rust
pub struct AntiHebbianConfig {
    pub beta: f64,  // default: 0.005
}

pub fn anti_hebbian_update<N, Ty: EdgeType>(graph: &mut StableGraph<N, f64, Ty>, activated: &[(NodeIndex, f64)], config: &AntiHebbianConfig) -> usize
```

### Oja's rule

Normalized Hebbian. Weights self-converge without manual capping.
Δw = η × y × (x − w × y)

```rust
pub struct OjaConfig {
    pub learning_rate: f64,  // default: 0.01
}

pub fn oja_update<N, Ty: EdgeType>(graph: &mut StableGraph<N, f64, Ty>, pre_activations: &[(NodeIndex, f64)], post_activations: &[(NodeIndex, f64)], config: &OjaConfig) -> usize
```

### BCM rule

Homeostatic plasticity with per-node sliding threshold. Prevents runaway
strengthening.

```rust
pub struct BcmConfig {
    pub learning_rate: f64,   // default: 0.01
    pub threshold_rate: f64,  // default: 0.001
}

pub struct BcmState { pub thresholds: Vec<f64> }

pub fn bcm_update<N, Ty: EdgeType>(graph: &mut StableGraph<N, f64, Ty>, activated: &[(NodeIndex, f64)], state: &mut BcmState, config: &BcmConfig) -> usize
```

## Module: `mst`

### Deviations from graphalgs

| Function  | graphalgs                 | petgraph-live                        |
| --------- | ------------------------- | ------------------------------------ |
| `prim`    | `Vec<(usize, usize)>`     | `Vec<(G::NodeId, G::NodeId)>`        |
| `boruvka` | `HashSet<(usize, usize)>` | `Vec<(G::NodeId, G::NodeId)>` sorted |

Raw `usize` indices are unsafe across graph mutations; `HashSet` is
non-deterministic in tests.

### Public API

```rust
pub fn prim<G, F, K>(graph: G, edge_cost: F) -> Vec<(G::NodeId, G::NodeId)>
pub fn boruvka<G, F, K>(graph: G, edge_cost: F) -> Vec<(G::NodeId, G::NodeId)>
pub use petgraph::algo::min_spanning_tree as kruskal;
```

Where `K: FloatMeasure` and `F: FnMut(G::EdgeRef) -> K`.

