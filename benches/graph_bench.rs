use criterion::{Criterion, criterion_group, criterion_main};
use graph_challenge::{graph::Graph, parallel_shortest_path::CanComputeParallelShortestPath};

const MAX: u64 = 1280000;
const DELTA: f64 = 1.0;

fn random_graph() -> Graph {
    Graph::random_connected_graph(MAX, MAX * 2, 1.0, 50.0)
}

fn bench_shortest_path(c: &mut Criterion) {
    let g = random_graph();
    c.bench_function("shortest path on random connected graph", |b| {
        b.iter(|| {
            g.shortest_path(0, MAX - 1);
        })
    });
}

fn bench_parallel_shortest_path(c: &mut Criterion) {
    let g = random_graph();
    c.bench_function("parallel shortest path on random connected graph", |b| {
        b.iter(|| {
            g.parallel_shortest_path(0, MAX - 1, DELTA);
        })
    });
}

fn bench_naive_parallel_shortest_path(c: &mut Criterion) {
    let g = random_graph();
    c.bench_function(
        "naive parallel shortest path on random connected graph",
        |b| {
            b.iter(|| {
                g.naive_parallel_shortest_path(0, MAX - 1, Some(DELTA));
            })
        },
    );
}

pub fn bench_graph_generation(c: &mut Criterion) {
    c.bench_function("generate random connected graph", |b| {
        b.iter(|| {
            random_graph();
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().significance_level(0.1).sample_size(10);
    targets =
        // bench_graph_generation,
        bench_parallel_shortest_path,
        bench_naive_parallel_shortest_path,
        bench_shortest_path,
);
criterion_main!(benches);
