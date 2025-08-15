pub(crate) mod worker;

use rand::Rng;
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

        // TODO single hashmap
        let mut dist = vec![(f64::INFINITY, None); self.adjacency.len() + 1];
        let mut heap = TheHeap::new();

        dist[start as usize] = (0.0, None);
        heap.push(State {
            cost: 0.0,
            position: start,
        });

        while let Some(State { cost, position }) = heap.pop() {
            if position == end {
                let mut path = vec![end];
                let mut current = end;
                while let Some(&p) = dist[current as usize].1.as_ref() {
                    path.push(p);
                    current = p;
                }
                path.reverse();
                return Some((path, cost));
            }

            if cost > dist[position as usize].0 {
                continue;
            }

            if let Some(neighbors) = self.adjacency.get(&position) {
                for (&neighbor, &weight) in neighbors {
                    let next = State {
                        cost: cost + weight,
                        position: neighbor,
                    };
                    if next.cost < dist[neighbor as usize].0 {
                        dist[neighbor as usize] = (next.cost, Some(position));
                        heap.push(next);
                    }
                }
            }
        }

        None
    }

    pub fn shortest_path_full(&self, start: u64, end: u64) -> Option<(Vec<u64>, f64)> {
        // TODO more compact vec-based implementation.
        //
        // questions:
        // 1. [x] can I change public fields?  YES, BACKWARD COMPATIBILITY NOT NEEDED
        // 2. do I need to handle negative/NaN weights?
        // 3. the examples use sequential node ids, can I rely on non-sparce node ids?
        if !self.adjacency.contains_key(&start) || !self.adjacency.contains_key(&end) {
            return None;
        }

        // let mut dist = SecMap::<u64, f64>::with_capacity(self.adjacency.len());
        // let mut prev = SecMap::<u64, u64>::with_capacity(self.adjacency.len());

        // let mut dist = SecMap::<u64, f64>::with_capacity_and_hasher(self.adjacency.len(), <_>::default());
        // let mut prev = SecMap::<u64, u64>::with_capacity_and_hasher(self.adjacency.len(), <_>::default());

        // TODO single hashmap
        let mut dist = vec![(f64::INFINITY, None); self.adjacency.len() + 1];
        let mut heap = TheHeap::new();

        dist[start as usize] = (0.0, None);
        heap.push(State {
            cost: 0.0,
            position: start,
        });

        while let Some(State { cost, position }) = heap.pop() {
            if cost > dist[position as usize].0 {
                continue;
            }

            if let Some(neighbors) = self.adjacency.get(&position) {
                for (&neighbor, &weight) in neighbors {
                    let next = State {
                        cost: cost + weight,
                        position: neighbor,
                    };
                    if next.cost < dist[neighbor as usize].0 {
                        dist[neighbor as usize] = (next.cost, Some(position));
                        heap.push(next);
                    }
                }
            }
        }

        if dist[end as usize].0 == f64::INFINITY {
            None
        } else {
            let mut path = vec![end];
            let mut current = end;
            while let Some(&p) = dist[current as usize].1.as_ref() {
                path.push(p);
                current = p;
            }
            path.reverse();
            Some((path, dist[end as usize].0))
        }
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

    pub fn parallel_shortest_path(
        &self,
        start: u64,
        end: u64,
        threads: usize,
    ) -> Option<(Vec<u64>, f64)> {
        use std::sync::atomic::Ordering;
        if !self.adjacency.contains_key(&start) || !self.adjacency.contains_key(&end) {
            return None;
        }

        let search = self::worker::Search::new(self, start);
        let workers = (0..threads)
            .map(|_| self::worker::Worker::new())
            .collect::<Vec<_>>();

        std::thread::scope(|s| {
            for id in 1..threads {
                let workers = &workers[..];
                let search = &search;
                s.spawn(move || {
                    search.run_worker(id, workers);
                    // let steals = workers[id].steals.load(Ordering::Relaxed);
                    // let steal_loops = workers[id].steal_loops.load(Ordering::Relaxed);
                    // eprintln!(
                    //     "Worker {} finished with {} steals, {} loops",
                    //     id, steals, steal_loops
                    // );
                });
            }
            search.start_work(0, &workers[..], start);
        });

        if search.prev[end as usize].load(Ordering::Relaxed) == -1 {
            None
        } else {
            let mut path = vec![end];
            let mut prev;
            while {
                prev = search.prev[*path.last().unwrap() as usize].load(Ordering::Relaxed);
                prev >= 0
            } {
                path.push(prev as u64);
            }
            path.reverse();
            Some((path, search.costs[end as usize].load(Ordering::Relaxed)))
        }
    }
}
