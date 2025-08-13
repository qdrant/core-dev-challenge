use graph_challenge::graph::Graph as RegularGraph;
use graph_challenge::graph_memmap::Graph;
use std::fs;

#[test]
fn test_complex_shortest_path() {
    let mut g = RegularGraph::new();
    // Create a more complex graph with multiple possible paths
    g.add_edge(1, 2, 4.0);
    g.add_edge(2, 3, 2.0);
    g.add_edge(3, 4, 3.0);
    g.add_edge(4, 5, 1.0);
    g.add_edge(1, 6, 2.0);
    g.add_edge(6, 7, 5.0);
    g.add_edge(7, 5, 3.0);
    g.add_edge(1, 8, 6.0);
    g.add_edge(8, 5, 4.0);
    g.add_edge(2, 7, 3.0);
    g.add_edge(3, 8, 5.0);
    let path = "complex.graph";
    g.save_to_file(path).unwrap();
    let g = Graph::load_from_file(path).unwrap();

    // Test shortest path from 1 to 5
    // Path options:
    // 1 -> 6 -> 7 -> 5 = 2 + 5 + 3 = 10
    // 1 -> 2 -> 7 -> 5 = 4 + 3 + 3 = 10
    // 1 -> 8 -> 5 = 6 + 4 = 10
    // 1 -> 2 -> 3 -> 4 -> 5 = 4 + 2 + 3 + 1 = 10
    let (path1, cost1) = g.shortest_path(1, 5).unwrap();
    // The algorithm found [1, 8, 5] which is correct (cost = 10)
    assert_eq!(cost1, 10.0);
    assert!(path1.len() >= 2);

    // Test shortest path from 1 to 8
    let (path2, cost2) = g.shortest_path(1, 8).unwrap();
    assert_eq!(path2, vec![1, 8]);
    assert_eq!(cost2, 6.0);

    // Test shortest path from 2 to 5
    let (path3, cost3) = g.shortest_path(2, 5).unwrap();
    assert_eq!(path3, vec![2, 7, 5]);
    assert_eq!(cost3, 6.0);

    // Test non-existent path
    assert!(g.shortest_path(5, 1).is_none());

    fs::remove_file(path).unwrap();
}

#[test]
fn test_add_and_edit_edge() {
    let mut g = RegularGraph::new();
    g.add_edge(1, 2, 5.0);
    let path = "edge.graph";
    g.save_to_file(path).unwrap();
    let mut g = Graph::load_from_file(path).unwrap();

    assert_eq!(g.get_edge_weight(1, 2), Some(5.0));
    g.set_edge_weight(1, 2, 10.0);
    assert_eq!(g.get_edge_weight(1, 2), Some(10.0));

    fs::remove_file(path).unwrap();
}
