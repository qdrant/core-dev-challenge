use core::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use graph_challenge::{
    graph::InMemoryGraph, parallel_shortest_path::CanComputeParallelShortestPath,
};

const MAX: u64 = 160000;
const EXTRA_EDGES: u64 = MAX * 4;
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
    c.bench_function("sequential shortest path on random connected graph", |b| {
        b.iter(|| {
            g.shortest_path(0, MAX - 1);
        })
    });
}

fn bench_parallel_shortest_path_50(c: &mut Criterion) {
    let g = load_graph();
    c.bench_function("parallel shortest path on random connected graph (50 buckets)", |b| {
        b.iter(|| {
            g.parallel_shortest_path(0, MAX - 1, 1.0);
        })
    });
}

fn bench_parallel_shortest_path_100(c: &mut Criterion) {
    let g = load_graph();
    c.bench_function("parallel shortest path on random connected graph (100 buckets)", |b| {
        b.iter(|| {
            g.parallel_shortest_path(0, MAX - 1, 0.5);
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(20)).sample_size(20);
    targets =
        bench_shortest_path,
        bench_parallel_shortest_path_50,
        bench_parallel_shortest_path_100,
);
criterion_main!(benches);
