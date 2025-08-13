use criterion::{Criterion, criterion_group, criterion_main};
use graph_challenge::graph::Graph;
use graph_challenge::graph_memmap::Graph as GraphMmap;
use graph_challenge::graph_vec::Graph as GraphVec;

const SEED: u128 = 1_000_000;

fn bench_shortest_path(c: &mut Criterion) {
    let g = Graph::random_connected_graph(100, 50, 1.0, 10.0, Some(SEED));
    c.bench_function("shortest path on random connected graph", |b| {
        b.iter(|| {
            g.shortest_path(0, 99);
        })
    });
}

fn bench_shortest_path_vec(c: &mut Criterion) {
    let g = GraphVec::random_connected_graph(100, 50, 1.0, 10.0, Some(SEED));
    c.bench_function("shortest path on random connected vec-backed graph", |b| {
        b.iter(|| {
            g.shortest_path(0, 99);
        })
    });
}

fn bench_shortest_path_mmap(c: &mut Criterion) {
    let g = Graph::random_connected_graph(100, 50, 1.0, 10.0, Some(SEED));
    let path = "tmp.graph";
    g.save_to_file(path).unwrap();
    let g = GraphMmap::load_from_file(path).unwrap();

    c.bench_function("shortest path on random connected mmap-backed graph", |b| {
        b.iter(|| {
            g.shortest_path(0, 99);
        })
    });

    std::fs::remove_file(path).unwrap();
}

fn bench_shortest_path_huge(c: &mut Criterion) {
    let g = Graph::random_connected_graph(1_000_000, 100_000, 1.0, 10.0, Some(SEED));
    c.bench_function("shortest path on random connected huge graph", |b| {
        b.iter(|| {
            g.shortest_path(0, 999_999);
        })
    });
}

fn bench_shortest_path_huge_vec(c: &mut Criterion) {
    let g = GraphVec::random_connected_graph(1_000_000, 100_000, 1.0, 10.0, Some(SEED));
    c.bench_function(
        "shortest path on random connected huge vec-backed graph",
        |b| {
            b.iter(|| {
                g.shortest_path(0, 999_999);
            })
        },
    );
}

fn bench_shortest_path_huge_mmap(c: &mut Criterion) {
    let g = Graph::random_connected_graph(1_000_000, 100_000, 1.0, 10.0, Some(SEED));
    let path = "tmp.graph";
    g.save_to_file(path).unwrap();
    let g = GraphMmap::load_from_file(path).unwrap();

    c.bench_function(
        "shortest path on random connected huge mmap-backed graph",
        |b| {
            b.iter(|| {
                g.shortest_path(0, 999_999);
            })
        },
    );
}

fn bench_graph_generation(c: &mut Criterion) {
    c.bench_function("generate random connected graph", |b| {
        b.iter(|| {
            Graph::random_connected_graph(100, 50, 1.0, 10.0, Some(SEED));
        })
    });
}

fn bench_graph_generation_vec(c: &mut Criterion) {
    c.bench_function("generate random connected vec-backed graph", |b| {
        b.iter(|| {
            GraphVec::random_connected_graph(100, 50, 1.0, 10.0, Some(SEED));
        })
    });
}

fn bench_graph_generation_large(c: &mut Criterion) {
    c.bench_function("generate random connected large graph", |b| {
        b.iter(|| {
            Graph::random_connected_graph(1_000_000, 10_000, 1.0, 10.0, Some(SEED));
        })
    });
}

fn bench_graph_generation_large_vec(c: &mut Criterion) {
    c.bench_function("generate random connected large vec-backed graph", |b| {
        b.iter(|| {
            GraphVec::random_connected_graph(1_000_000, 10_000, 1.0, 10.0, Some(SEED));
        })
    });
}

criterion_group!(
    benches,
    bench_shortest_path,
    bench_shortest_path_vec,
    bench_shortest_path_mmap,
    bench_shortest_path_huge,
    bench_shortest_path_huge_vec,
    bench_shortest_path_huge_mmap,
    bench_graph_generation,
    bench_graph_generation_vec,
    bench_graph_generation_large,
    bench_graph_generation_large_vec,
);
criterion_main!(benches);
