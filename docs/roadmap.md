# petgraph-live — Roadmap

**Current status:** v0.3.0 released.

## Futur improvments

Extended algorithms

| Feature                                              | Notes                                                    |
| ---------------------------------------------------- | -------------------------------------------------------- |
| `elementary_circuits` — Tarjan's circuit enumeration | Needs careful cycle detection, expensive on dense graphs |
| `tournament` module                                  | Tournament-specific algorithms from graphalgs            |
| `coloring::dsatur`                                   | DSATUR greedy graph colouring                            |


## Permanently out of scope

| Item                                | Reason                                                        |
| ----------------------------------- | ------------------------------------------------------------- |
| `adj_matrix` / `spec` (nalgebra)    | Heavy dep; petgraph already has `to_adjacency_matrix()`       |
| `generate` (random graphs)          | Pulls `rand`; niche use, not a core concern                   |
| Async runtime support               | No `tokio`/`async-std` — library stays runtime-agnostic       |
| petgraph < 0.8                      | Not maintained here; `graphalgs` covers older versions        |
| Community detection (Louvain, etc.) | Domain-specific; belongs in consumer crate (e.g. `llm-wiki`)  |
| Live operational improvements       | Rebuild coalescing, background rebuild — caller responsibility |


## Consumer

Primary consumer: [`llm-wiki`](https://github.com/geronimo-iia/llm-wiki), a
git-backed wiki engine with MCP server. The specific mapping:

| llm-wiki concept                     | petgraph-live API                                       |
| ------------------------------------ | ------------------------------------------------------- |
| `get_or_build_graph` + `CachedGraph` | `cache::GenerationCache<WikiGraph>`                     |
| graph snapshot across restarts       | `snapshot::save` / `snapshot::load`                     |
| managed cache + snapshot lifecycle   | `live::GraphState<WikiGraph>`                           |
| structural analysis of wiki pages    | `connect::articulation_points`, `connect::find_bridges` |
| wiki graph health                    | `metrics::diameter`, `metrics::center`                  |
