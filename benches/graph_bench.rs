use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use graph_challenge::graph::Graph;

const BENCH_SIZE: u64 = 900000;
const THREADS: usize = 6;

fn bench_shortest_path(c: &mut Criterion) {
    // avaraging  on multiple graphs
    let graphs = vec![
        Graph::random_connected_graph(BENCH_SIZE, BENCH_SIZE as usize / 100, 1.0, 10.0),
        Graph::random_connected_graph(BENCH_SIZE, BENCH_SIZE as usize / 100, 1.0, 10.0),
        Graph::random_connected_graph(BENCH_SIZE, BENCH_SIZE as usize / 100, 1.0, 10.0),
        Graph::random_connected_graph(BENCH_SIZE, BENCH_SIZE as usize / 100, 1.0, 10.0),
        Graph::random_connected_graph(BENCH_SIZE, BENCH_SIZE as usize / 100, 1.0, 10.0),
    ];
    let mut b = c.benchmark_group("shortest path on random connected graph");
    for mode in ["seq", "full", "parallel"] {
        b.bench_with_input(
            BenchmarkId::from_parameter(mode),
            mode,
            |b, mode| match mode {
                "seq" => {
                    for g in &graphs {
                        b.iter(|| g.shortest_path(0, BENCH_SIZE - 1));
                    }
                }
                "full" => {
                    for g in &graphs {
                        b.iter(|| g.shortest_path_full(0, BENCH_SIZE - 1))
                    }
                }
                "parallel" => {
                    for g in &graphs {
                        b.iter(|| g.parallel_shortest_path(0, BENCH_SIZE - 1, THREADS))
                    }
                }
                _ => unreachable!(),
            },
        );
    }
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
