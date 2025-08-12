use graph_challenge::graph_vec::Graph;
use std::fs;

#[test]
fn test_add_and_remove_vertex() {
    let mut g = Graph::new();
    g.add_vertex(1);
    assert!(g.adjacency.contains_key(&1));
    g.remove_vertex(1);
    assert!(!g.adjacency.contains_key(&1));
}

#[test]
fn test_add_and_remove_edge() {
    let mut g = Graph::new();
    g.add_edge(1, 2, 5.0);
    assert_eq!(g.get_edge_weight(1, 2), Some(5.0));
    g.remove_edge(1, 2);
    assert_eq!(g.get_edge_weight(1, 2), None);
}

#[test]
fn test_neighbors() {
    let mut g = Graph::new();
    g.add_edge(1, 2, 3.0);
    g.add_edge(1, 3, 7.0);
    let neighbors = g.neighbors(1).unwrap();
    assert_eq!(neighbors.iter().find(|(v, _)| v == &2), Some(&(2, 3.0)));
    assert_eq!(neighbors.iter().find(|(v, _)| v == &3), Some(&(3, 7.0)));
}

#[test]
fn test_persistence() {
    let mut g = Graph::new();
    g.add_edge(1, 2, 4.5);
    let path = "test_graph.bin";
    g.save_to_file(path).unwrap();
    let loaded = Graph::load_from_file(path).unwrap();
    assert_eq!(g.adjacency, loaded.adjacency);
    fs::remove_file(path).unwrap();
}

#[test]
fn test_shortest_path() {
    let mut g = Graph::new();
    g.add_edge(1, 2, 1.0);
    g.add_edge(2, 3, 2.0);
    g.add_edge(1, 4, 4.0);
    g.add_edge(4, 3, 1.0);
    let (path, cost) = g.shortest_path(1, 3).unwrap();
    assert_eq!(path, vec![1, 2, 3]);
    assert_eq!(cost, 3.0);
    assert!(g.shortest_path(3, 1).is_none());
}

#[test]
fn test_weighted_shortest_path() {
    let mut g = Graph::new();
    g.add_edge(1, 2, 5.0);
    g.add_edge(2, 3, 2.0);
    g.add_edge(1, 3, 10.0);
    let (path, cost) = g.shortest_path(1, 3).unwrap();
    assert_eq!(path, vec![1, 2, 3]);
    assert_eq!(cost, 7.0);
}

#[test]
fn test_complex_shortest_path() {
    let mut g = Graph::new();
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
}

#[test]
fn test_unweighted_edge() {
    let mut g = Graph::new();
    g.add_unweighted_edge(1, 2);
    assert_eq!(g.get_edge_weight(1, 2), Some(1.0));
}

#[test]
fn test_random_connected_graph() {
    let graph = Graph::random_connected_graph(10, 5, 1.0, 10.0, Some(1));

    // Check that we have the right number of vertices
    assert_eq!(graph.adjacency.len(), 10);

    // Count edges (should be at least 9 for connectivity + 5 additional)
    let edge_count: usize = graph
        .adjacency
        .values()
        .map(|neighbors| neighbors.len())
        .sum();
    assert!(edge_count >= 14); // 9 for spanning tree + 5 additional

    // Check connectivity by ensuring there's a path from 0 to 9
    let (path, _cost) = graph.shortest_path(0, 9).unwrap();
    assert!(!path.is_empty());
}
