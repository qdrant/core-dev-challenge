use std::fs::exists;

use criterion::{Criterion, criterion_group, criterion_main};
use graph_challenge::{graph::Graph, parallel_shortest_path::CanComputeParallelShortestPath};

const MAX: u64 = 640000;
const DELTA: f64 = 1.0;
const EXTRA_EDGES: u64 = MAX * 2;
const GRAPH_PATH: &str = "graph.bin";

fn random_graph() -> Graph {
    Graph::random_connected_graph(MAX, EXTRA_EDGES, 1.0, 50.0)
}

fn load_graph() -> Graph {
    if exists(GRAPH_PATH).unwrap() {
        Graph::load_from_file(GRAPH_PATH).unwrap()
    } else {
        let graph = random_graph();
        graph.save_to_file(GRAPH_PATH).unwrap();
        graph
    }
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
            g.parallel_shortest_path(0, MAX - 1, DELTA);
        })
    });
}

fn bench_naive_parallel_shortest_path(c: &mut Criterion) {
    let g = load_graph();
    c.bench_function(
        "naive parallel shortest path on random connected graph",
        |b| {
            b.iter(|| {
                g.naive_parallel_shortest_path(0, MAX - 1, Some(DELTA));
            })
        },
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default().significance_level(0.1).sample_size(10);
    targets =
        bench_parallel_shortest_path,
        bench_naive_parallel_shortest_path,
        bench_shortest_path,
);
criterion_main!(benches);
