use criterion::{Criterion, criterion_group, criterion_main};
use graph_challenge::graph::Graph;

fn bench_shortest_path(c: &mut Criterion) {
    let g = Graph::random_connected_graph(100, 50, 1.0, 10.0);
    c.bench_function("shortest path on random connected graph", |b| {
        b.iter(|| {
            g.shortest_path(0, 99);
        })
    });
}

fn bench_graph_generation(c: &mut Criterion) {
    c.bench_function("generate random connected graph", |b| {
        b.iter(|| {
            Graph::random_connected_graph(100, 50, 1.0, 10.0);
        })
    });
}

criterion_group!(benches, bench_shortest_path, bench_graph_generation);
criterion_main!(benches);
