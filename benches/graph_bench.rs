use criterion::{Criterion, criterion_group, criterion_main};
use graph_challenge::{graph::Graph, parallel_shortest_path::CanComputeParallelShortestPath};

const MAX: u64 = 10000;
const DELTA: f64 = 1.0;

fn random_graph() -> Graph {
    Graph::random_connected_graph(MAX, MAX * 200, 1.0, 20.0)
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

fn bench_graph_generation(c: &mut Criterion) {
    c.bench_function("generate random connected graph", |b| {
        b.iter(|| {
            random_graph();
        })
    });
}

criterion_group!(
    benches,
    // bench_graph_generation,
    bench_parallel_shortest_path,
    bench_naive_parallel_shortest_path,
    bench_shortest_path,
);
criterion_main!(benches);
