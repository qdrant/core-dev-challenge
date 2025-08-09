use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::sync::{Arc, Mutex};

pub type Node = u64;
pub type Cost = f64;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Graph {
    pub adjacency: HashMap<Node, HashMap<Node, Cost>>,
}

impl crate::traits::Graph for Graph {
    type Node = Node;
    type Cost = Cost;

    fn get_neighbors(
        &self,
        node: &Self::Node,
    ) -> Option<impl Iterator<Item = (Self::Node, Self::Cost)>> {
        let neighbors = self.adjacency.get(node)?;
        Some(neighbors.iter().map(|(node, cost)| (*node, *cost)))
    }

    fn floor_cost(cost: Self::Cost) -> usize {
        cost.floor() as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
struct State {
    cost: Cost,
    position: Node,
}

impl Eq for State {}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap()
    }
}

impl Graph {
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
        }
    }

    pub fn add_vertex(&mut self, v: Node) {
        self.adjacency.entry(v).or_default();
    }

    pub fn add_edge(&mut self, from: Node, to: Node, weight: Cost) {
        self.add_vertex(from);
        self.add_vertex(to);
        self.adjacency.get_mut(&from).unwrap().insert(to, weight);
    }

    pub fn remove_vertex(&mut self, v: Node) {
        self.adjacency.remove(&v);
        for neighbors in self.adjacency.values_mut() {
            neighbors.remove(&v);
        }
    }

    pub fn remove_edge(&mut self, from: Node, to: Node) {
        if let Some(neighbors) = self.adjacency.get_mut(&from) {
            neighbors.remove(&to);
        }
    }

    pub fn neighbors(&self, v: Node) -> Option<&HashMap<Node, Cost>> {
        self.adjacency.get(&v)
    }

    pub fn get_edge_weight(&self, from: Node, to: Node) -> Option<Cost> {
        self.adjacency.get(&from)?.get(&to).copied()
    }

    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self).map_err(io::Error::other)
    }

    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        bincode::deserialize_from(reader).map_err(io::Error::other)
    }

    pub fn shortest_path(&self, start: Node, end: Node) -> Option<(Vec<Node>, Cost)> {
        if !self.adjacency.contains_key(&start) || !self.adjacency.contains_key(&end) {
            return None;
        }

        let mut distances_from_start: HashMap<Node, Cost> = HashMap::new();
        let mut predecessors: HashMap<Node, Node> = HashMap::new();
        let mut heap = BinaryHeap::new();

        distances_from_start.insert(start, 0.0);
        heap.push(State {
            cost: 0.0,
            position: start,
        });

        while let Some(State { cost, position }) = heap.pop() {
            if position == end {
                let mut path = vec![end];
                let mut current = end;
                while let Some(&p) = predecessors.get(&current) {
                    path.push(p);
                    current = p;
                }
                path.reverse();
                return Some((path, cost));
            }

            if cost > distances_from_start[&position] {
                continue;
            }

            if let Some(neighbors) = self.adjacency.get(&position) {
                for (&neighbor, &weight) in neighbors {
                    let next = State {
                        cost: cost + weight,
                        position: neighbor,
                    };
                    if next.cost
                        < *distances_from_start
                            .get(&neighbor)
                            .unwrap_or(&Cost::INFINITY)
                    {
                        distances_from_start.insert(neighbor, next.cost);
                        predecessors.insert(neighbor, position);
                        heap.push(next);
                    }
                }
            }
        }

        None
    }

    /// Parallel shortest path using Delta-Stepping algorithm
    ///
    /// # Arguments
    /// * `start` - Starting vertex
    /// * `end` - Target vertex
    /// * `delta` - Delta parameter for bucketing (if None, auto-calculated)
    ///
    /// # Returns
    /// Optional tuple of (path, total_cost)
    pub fn naive_parallel_shortest_path(
        &self,
        start: Node,
        end: Node,
        delta: Option<Cost>,
    ) -> Option<(Vec<Node>, Cost)> {
        if !self.adjacency.contains_key(&start) || !self.adjacency.contains_key(&end) {
            return None;
        }

        // Calculate delta if not provided
        let delta = delta.unwrap_or_else(|| self.calculate_optimal_delta());

        // Initialize data structures
        let mut distances: HashMap<Node, Cost> = HashMap::new();
        let mut predecessors: HashMap<Node, Node> = HashMap::new();
        let mut buckets: Vec<Vec<Node>> = vec![vec![]]; // Start with one bucket

        distances.insert(start, 0.0);
        buckets[0].push(start);

        let mut current_bucket_idx = 0;

        while current_bucket_idx < buckets.len() {
            // Ensure we have a non-empty bucket
            if buckets[current_bucket_idx].is_empty() {
                current_bucket_idx += 1;
                continue;
            }

            // Process light edges until no changes in current bucket
            loop {
                let current_bucket = &buckets[current_bucket_idx];
                if current_bucket.is_empty() {
                    break;
                }

                // Collect light edge requests in parallel
                let light_requests: Arc<Mutex<Vec<(Node, Cost, Node)>>> =
                    Arc::new(Mutex::new(vec![]));

                current_bucket.par_iter().for_each(|&vertex| {
                    let vertex_distance = distances[&vertex];
                    if let Some(neighbors) = self.adjacency.get(&vertex) {
                        let mut local_requests = vec![];
                        for (&neighbor, &weight) in neighbors {
                            if weight <= delta {
                                let new_distance = vertex_distance + weight;
                                local_requests.push((neighbor, new_distance, vertex));
                            }
                        }
                        if !local_requests.is_empty() {
                            light_requests.lock().unwrap().extend(local_requests);
                        }
                    }
                });

                // Apply light edge updates
                let requests = light_requests.lock().unwrap();
                let mut bucket_changed = false;
                let mut new_vertices_for_current = vec![];

                for &(neighbor, new_distance, predecessor) in requests.iter() {
                    let current_distance =
                        distances.get(&neighbor).copied().unwrap_or(Cost::INFINITY);
                    if new_distance < current_distance {
                        distances.insert(neighbor, new_distance);
                        predecessors.insert(neighbor, predecessor);

                        let bucket_idx = (new_distance / delta).floor() as usize;

                        // Ensure we have enough buckets
                        while buckets.len() <= bucket_idx {
                            buckets.push(vec![]);
                        }

                        if bucket_idx == current_bucket_idx {
                            new_vertices_for_current.push(neighbor);
                            bucket_changed = true;
                        } else {
                            buckets[bucket_idx].push(neighbor);
                        }
                    }
                }

                // Add new vertices to current bucket
                buckets[current_bucket_idx].extend(new_vertices_for_current);

                if !bucket_changed {
                    break;
                }
            }

            // Process heavy edges from current bucket
            let current_bucket = &buckets[current_bucket_idx];
            let heavy_requests: Arc<Mutex<Vec<(Node, Cost, Node)>>> = Arc::new(Mutex::new(vec![]));

            current_bucket.par_iter().for_each(|&vertex| {
                let vertex_dist = distances[&vertex];
                if let Some(neighbors) = self.adjacency.get(&vertex) {
                    let mut local_requests = vec![];
                    for (&neighbor, &weight) in neighbors {
                        if weight > delta {
                            let new_dist = vertex_dist + weight;
                            local_requests.push((neighbor, new_dist, vertex));
                        }
                    }
                    if !local_requests.is_empty() {
                        heavy_requests.lock().unwrap().extend(local_requests);
                    }
                }
            });

            // Apply heavy edge updates
            let requests = heavy_requests.lock().unwrap();
            for &(neighbor, new_distance, predecessor) in requests.iter() {
                let current_distance = distances.get(&neighbor).copied().unwrap_or(Cost::INFINITY);
                if new_distance < current_distance {
                    distances.insert(neighbor, new_distance);
                    predecessors.insert(neighbor, predecessor);

                    let bucket_idx = (new_distance / delta).floor() as usize;

                    // Ensure we have enough buckets
                    while buckets.len() <= bucket_idx {
                        buckets.push(vec![]);
                    }

                    buckets[bucket_idx].push(neighbor);
                }
            }

            // Clear current bucket and move to next
            buckets[current_bucket_idx].clear();
            current_bucket_idx += 1;
        }

        // Reconstruct path if end vertex was reached
        if let Some(&final_cost) = distances.get(&end) {
            let mut path = vec![end];
            let mut current = end;
            while let Some(&predecessor) = predecessors.get(&current) {
                path.push(predecessor);
                current = predecessor;
            }
            path.reverse();
            Some((path, final_cost))
        } else {
            None
        }
    }

    /// Calculate optimal delta parameter based on graph characteristics
    fn calculate_optimal_delta(&self) -> Cost {
        if self.adjacency.is_empty() {
            return 1.0;
        }

        // Calculate average edge weight
        let mut total_weight = 0.0;
        let mut edge_count = 0;

        for neighbors in self.adjacency.values() {
            for &weight in neighbors.values() {
                total_weight += weight;
                edge_count += 1;
            }
        }

        if edge_count == 0 {
            return 1.0;
        }

        let avg_weight = total_weight / edge_count as f64;

        // Use a fraction of average weight as delta
        // This balances parallelism vs overhead
        (avg_weight * 0.5).max(0.1)
    }

    // For backward compatibility with unweighted graphs
    pub fn add_unweighted_edge(&mut self, from: Node, to: Node) {
        self.add_edge(from, to, 1.0);
    }

    /// Generate a random connected graph with specified number of vertices
    ///
    /// # Arguments
    /// * `num_vertices` - Number of vertices in the graph
    /// * `additional_edges` - Additional random edges beyond the minimum for connectivity
    /// * `min_weight` - Minimum edge weight (inclusive)
    /// * `max_weight` - Maximum edge weight (exclusive)
    ///
    /// # Returns
    /// A new connected Graph with random edges
    pub fn random_connected_graph(
        num_vertices: Node,
        additional_edges: u64,
        min_weight: Cost,
        max_weight: Cost,
    ) -> Self {
        let mut graph = Graph::new();
        let mut rng = rand::thread_rng();

        // Add all vertices first
        for i in 0..num_vertices {
            graph.add_vertex(i);
        }

        // Create a spanning tree to ensure connectivity
        for i in 1..num_vertices {
            let parent = rng.gen_range(0..i);
            let weight = rng.gen_range(min_weight..max_weight);
            graph.add_edge(parent, i, weight);
        }

        // Add additional random edges
        let mut edges_added = 0;
        let max_attempts = additional_edges * 10;
        let mut attempts = 0;

        while edges_added < additional_edges && attempts < max_attempts {
            let from = rng.gen_range(0..num_vertices);
            let to = rng.gen_range(0..num_vertices);

            if from != to && graph.get_edge_weight(from, to).is_none() {
                let weight = rng.gen_range(min_weight..max_weight);
                graph.add_edge(from, to, weight);
                edges_added += 1;
            }
            attempts += 1;
        }

        graph
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
