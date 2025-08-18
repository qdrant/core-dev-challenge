pub(crate) mod parallel_search;

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};

use dary_heap::QuaternaryHeap as TheHeap;
use nohash_hasher::IntMap as TheMap;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Graph {
    pub adjacency: TheMap<u64, TheMap<u64, f64>>,
}

#[derive(Debug, Clone, PartialEq)]
struct State {
    cost: f64,
    position: u64,
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
        <_>::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            adjacency: TheMap::with_capacity_and_hasher(capacity, <_>::default()),
        }
    }

    pub fn add_vertex(&mut self, v: u64) {
        self.adjacency.entry(v).or_default();
    }

    pub fn add_edge(&mut self, from: u64, to: u64, weight: f64) {
        self.add_vertex(from);
        self.add_vertex(to);
        self.adjacency.get_mut(&from).unwrap().insert(to, weight);
    }

    pub fn remove_vertex(&mut self, v: u64) {
        self.adjacency.remove(&v);
        for neighbors in self.adjacency.values_mut() {
            neighbors.remove(&v);
        }
    }

    pub fn remove_edge(&mut self, from: u64, to: u64) {
        if let Some(neighbors) = self.adjacency.get_mut(&from) {
            neighbors.remove(&to);
        }
    }

    pub fn neighbors(&self, v: u64) -> Option<&TheMap<u64, f64>> {
        self.adjacency.get(&v)
    }

    pub fn get_edge_weight(&self, from: u64, to: u64) -> Option<f64> {
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

    pub fn shortest_path(&self, start: u64, end: u64) -> Option<(Vec<u64>, f64)> {
        if !self.adjacency.contains_key(&start) || !self.adjacency.contains_key(&end) {
            return None;
        }

        let mut dist = vec![f64::INFINITY; self.adjacency.len() + 1];
        let mut prev = vec![None; self.adjacency.len() + 1];
        let mut heap = TheHeap::new();

        dist[start as usize] = 0.0;
        heap.push(State {
            cost: 0.0,
            position: start,
        });

        while let Some(State { cost, position }) = heap.pop() {
            if position == end {
                let mut path = vec![end];
                let mut current = end;
                while let Some(p) = prev[current as usize] {
                    path.push(p);
                    current = p;
                }
                path.reverse();
                return Some((path, cost));
            }

            if cost > dist[position as usize] {
                continue;
            }

            if let Some(neighbors) = self.adjacency.get(&position) {
                for (&neighbor, &weight) in neighbors {
                    let next = State {
                        cost: cost + weight,
                        position: neighbor,
                    };
                    if next.cost < dist[neighbor as usize] {
                        dist[neighbor as usize] = next.cost;
                        prev[neighbor as usize] = Some(position);
                        heap.push(next);
                    }
                }
            }
        }

        None
    }

    // For backward compatibility with unweighted graphs
    pub fn add_unweighted_edge(&mut self, from: u64, to: u64) {
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
        num_vertices: u64,
        additional_edges: usize,
        min_weight: f64,
        max_weight: f64,
    ) -> Self {
        let mut rng = rand::thread_rng();
        Self::random_connected_graph_with_rng(
            num_vertices,
            additional_edges,
            min_weight,
            max_weight,
            &mut rng,
        )
    }

    pub fn random_connected_graph_with_rng<Rng: rand::Rng>(
        num_vertices: u64,
        additional_edges: usize,
        min_weight: f64,
        max_weight: f64,
        rng: &mut Rng,
    ) -> Self {
        let mut graph = Graph::with_capacity(num_vertices as usize);

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

    pub fn parallel_shortest_path(
        &self,
        start: u64,
        end: u64,
        threads: usize,
    ) -> Option<(Vec<u64>, f64)> {
        if !self.adjacency.contains_key(&start) || !self.adjacency.contains_key(&end) {
            return None;
        }

        let search = self::parallel_search::Search::new(self, start, end);
        search.run(threads)
    }
}
