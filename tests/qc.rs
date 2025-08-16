#[quickcheck_macros::quickcheck]
fn qc_parallel_vs_single_thread(seed: u64) {
    use graph_challenge::graph::Graph;
    use rand::{Rng as _, SeedableRng as _};

    const GRAPH_SIZE: u64 = 90000;
    const THREADS: usize = 4;

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let g = Graph::random_connected_graph_with_rng(
        GRAPH_SIZE,
        GRAPH_SIZE as usize / 100,
        1.0,
        10.0,
        &mut rng,
    );
    let end = rng.gen_range((GRAPH_SIZE - GRAPH_SIZE / 9)..GRAPH_SIZE);

    let single = dbg!(g.shortest_path(0, end));
    let parallel = dbg!(g.parallel_shortest_path(0, end, THREADS));

    assert_eq!(single, parallel);
}
