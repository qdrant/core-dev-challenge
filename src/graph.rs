use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::{self, BufReader, BufWriter};

pub type Vertex = u64;
pub type Cost = f64;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Graph {
    pub adjacency: HashMap<Vertex, HashMap<Vertex, Cost>>,
}

#[derive(Debug, Clone, PartialEq)]
struct State {
    cost: Cost,
    position: Vertex,
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

    pub fn add_vertex(&mut self, v: Vertex) {
        self.adjacency.entry(v).or_default();
    }

    pub fn add_edge(&mut self, from: Vertex, to: Vertex, weight: Cost) {
        self.add_vertex(from);
        self.add_vertex(to);
        self.adjacency.get_mut(&from).unwrap().insert(to, weight);
    }

    pub fn remove_vertex(&mut self, v: Vertex) {
        self.adjacency.remove(&v);
        for neighbors in self.adjacency.values_mut() {
            neighbors.remove(&v);
        }
    }

    pub fn remove_edge(&mut self, from: Vertex, to: Vertex) {
        if let Some(neighbors) = self.adjacency.get_mut(&from) {
            neighbors.remove(&to);
        }
    }

    pub fn neighbors(&self, v: Vertex) -> Option<&HashMap<Vertex, Cost>> {
        self.adjacency.get(&v)
    }

    pub fn get_edge_weight(&self, from: Vertex, to: Vertex) -> Option<Cost> {
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

    pub fn shortest_path(&self, start: Vertex, end: Vertex) -> Option<(Vec<Vertex>, Cost)> {
        if !self.adjacency.contains_key(&start) || !self.adjacency.contains_key(&end) {
            return None;
        }

        let mut distances_from_start: HashMap<Vertex, Cost> = HashMap::new();
        let mut predecessors: HashMap<Vertex, Vertex> = HashMap::new();
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

    // For backward compatibility with unweighted graphs
    pub fn add_unweighted_edge(&mut self, from: Vertex, to: Vertex) {
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
        num_vertices: Vertex,
        additional_edges: usize,
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
