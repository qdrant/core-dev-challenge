use criterion::{Criterion, criterion_group, criterion_main};
use graph_challenge::{
    graph::InMemoryGraph, parallel_shortest_path::CanComputeParallelShortestPath,
};

const MAX: u64 = 640000;
const EXTRA_EDGES: u64 = MAX * 2;
const GRAPH_PATH: &str = "graph.bin";
const BUCKETS: usize = 50;

fn load_graph() -> InMemoryGraph {
    InMemoryGraph::load_or_generate_random_connected_graph(
        GRAPH_PATH,
        MAX,
        EXTRA_EDGES,
        1.0,
        BUCKETS as f64,
    )
    .unwrap()
}

fn bench_shortest_path(c: &mut Criterion) {
    let g = load_graph();
    c.bench_function("shortest path on random connected graph", |b| {
        b.iter(|| {
            g.shortest_path(0, MAX - 1);
        })
    });
}

fn bench_parallel_shortest_path(c: &mut Criterion) {
    let g = load_graph();
    c.bench_function("parallel shortest path on random connected graph", |b| {
        b.iter(|| {
            g.parallel_shortest_path(0, MAX - 1, 1.0);
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().significance_level(0.1).sample_size(10);
    targets =
        bench_parallel_shortest_path,
        bench_shortest_path,
);
criterion_main!(benches);
