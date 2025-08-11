use core::time::Duration;
use std::sync::LazyLock;

use criterion::{Criterion, criterion_group, criterion_main};
use graph_challenge::{
    graph::InMemoryGraph, parallel_shortest_path::CanComputeParallelShortestPath,
};

/**
   This benchmark suite compares the performance of the parallel shortest path algorithm
   against the original sequential algorithm on a large, randomly generated connected graph.

   To run the benchmark, tweak the parameters below before running `cargo bench`.

   Because the graph is randomly generated, the benchmark results may vary depending on
   the specific graph that is generated each time. Sometimes, a random graph may contain
   much shorter path to the target node, resulting in the algorithms to run much faster.

   As a result, to get the full picture, it is recommended to run the benchmark multiple times
   with the same settings but with different random graphs. Within a single run of the benchmark,
   both algorithms will be run on the same graph, so the results are comparable. The important thing
   to note is the relative performance of the parallel algorithm as compared to the sequential one.
*/

/**
   The total number of vertices
*/
const TOTAL_NODES: u64 = 640000;

/**
   The additional random edges to add to the graph.
*/
const EXTRA_EDGES: u64 = TOTAL_NODES * 4;

/**
   The maximum weight of an edge in the graph.

   This will determine the number of buckets used in the parallel shortest path algorithm.
   By default, we benchmark with delta values of 1.0 and 0.5.
*/
const MAX_WEIGHT: f64 = 50.0;

static GRAPH: LazyLock<InMemoryGraph> = LazyLock::new(|| {
    InMemoryGraph::random_connected_graph(TOTAL_NODES, EXTRA_EDGES, 1.0, MAX_WEIGHT).unwrap()
});

fn bench_shortest_path(c: &mut Criterion) {
    c.bench_function("sequential shortest path on random connected graph", |b| {
        b.iter(|| {
            GRAPH.shortest_path(0, TOTAL_NODES - 1);
        })
    });
}

fn bench_parallel_shortest_path(c: &mut Criterion) {
    c.bench_function(
        "parallel shortest path on random connected graph (1.0 delta)",
        |b| {
            b.iter(|| {
                GRAPH.parallel_shortest_path(0, TOTAL_NODES - 1, 1.0);
            })
        },
    );
}

fn bench_parallel_shortest_path_half_delta(c: &mut Criterion) {
    c.bench_function(
        "parallel shortest path on random connected graph (0.5 delta)",
        |b| {
            b.iter(|| {
                GRAPH.parallel_shortest_path(0, TOTAL_NODES - 1, 0.5);
            })
        },
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(20)).sample_size(20);
    targets =
        bench_shortest_path,
        bench_parallel_shortest_path,
        bench_parallel_shortest_path_half_delta,
);
criterion_main!(benches);
