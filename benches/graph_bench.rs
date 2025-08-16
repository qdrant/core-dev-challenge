use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use graph_challenge::graph::Graph;

const BENCH_SIZE: u64 = 400000;
const THREADS: usize = 4;

fn bench_shortest_path(c: &mut Criterion) {
    use rand::SeedableRng;
    // avaraging  on multiple graphs
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    let graphs = vec![
        Graph::random_connected_graph_with_rng(BENCH_SIZE, BENCH_SIZE as usize / 100, 1.0, 10.0, &mut rng),
        Graph::random_connected_graph_with_rng(BENCH_SIZE, BENCH_SIZE as usize / 100, 1.0, 10.0, &mut rng),
        Graph::random_connected_graph_with_rng(BENCH_SIZE, BENCH_SIZE as usize / 100, 1.0, 10.0, &mut rng),
        Graph::random_connected_graph_with_rng(BENCH_SIZE, BENCH_SIZE as usize / 100, 1.0, 10.0, &mut rng),
        Graph::random_connected_graph_with_rng(BENCH_SIZE, BENCH_SIZE as usize / 100, 1.0, 10.0, &mut rng),
    ];
    let mut b = c.benchmark_group("shortest path on random connected graph");
    for mode in ["seq", "full", "parallel"] {
        b.bench_with_input(
            BenchmarkId::from_parameter(mode),
            mode,
            |b, mode| match mode {
                "seq" => {
                    b.iter(|| {
                        (
                            graphs[0].shortest_path(0, BENCH_SIZE - 1),
                            graphs[1].shortest_path(0, BENCH_SIZE - 1),
                            graphs[2].shortest_path(0, BENCH_SIZE - 1),
                            graphs[3].shortest_path(0, BENCH_SIZE - 1),
                            graphs[4].shortest_path(0, BENCH_SIZE - 1),
                        )
                    });
                }
                "full" => b.iter(|| {
                    (
                        graphs[0].shortest_path_full(0, BENCH_SIZE - 1),
                        graphs[1].shortest_path_full(0, BENCH_SIZE - 1),
                        graphs[2].shortest_path_full(0, BENCH_SIZE - 1),
                        graphs[3].shortest_path_full(0, BENCH_SIZE - 1),
                        graphs[4].shortest_path_full(0, BENCH_SIZE - 1),
                    )
                }),
                "parallel" => b.iter(|| {
                    (
                        graphs[0].parallel_shortest_path(0, BENCH_SIZE - 1, THREADS),
                        graphs[1].parallel_shortest_path(0, BENCH_SIZE - 1, THREADS),
                        graphs[2].parallel_shortest_path(0, BENCH_SIZE - 1, THREADS),
                        graphs[3].parallel_shortest_path(0, BENCH_SIZE - 1, THREADS),
                        graphs[4].parallel_shortest_path(0, BENCH_SIZE - 1, THREADS),
                    )
                }),
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
