use num_traits::Zero;
use rayon::prelude::*;
use std::collections::hash_map::Entry;
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};

use crate::traits::Graph;

pub trait CanComputeParallelShortestPath: Graph {
    fn parallel_shortest_path(
        &self,
        source: Self::Node,
        target: Self::Node,
        delta: Self::Cost,
    ) -> Option<(VecDeque<Self::Node>, Self::Cost)>;
}

impl<G: Graph> CanComputeParallelShortestPath for G {
    fn parallel_shortest_path(
        &self,
        source: Self::Node,
        target: Self::Node,
        delta: Self::Cost,
    ) -> Option<(VecDeque<Self::Node>, Self::Cost)> {
        State::<G>::shortest_path(self, source, target, delta)
    }
}

struct State<'a, G: Graph> {
    graph: &'a G,
    delta: G::Cost,
    lowest_costs: HashMap<G::Node, LowestCost<G>>,
    buckets: BTreeMap<usize, Vec<G::Node>>,
}

#[derive(Debug)]
struct Edge<G: Graph> {
    source: G::Node,
    target: G::Node,
    total_cost: G::Cost,
}

struct LowestCost<G: Graph> {
    cost: G::Cost,
    predecessor: G::Node,
}

impl<G: Graph> Clone for LowestCost<G> {
    fn clone(&self) -> Self {
        Self {
            cost: self.cost,
            predecessor: self.predecessor,
        }
    }
}

impl<'a, G: Graph> State<'a, G> {
    fn shortest_path(
        graph: &'a G,
        source: G::Node,
        target: G::Node,
        delta: G::Cost,
    ) -> Option<(VecDeque<G::Node>, G::Cost)> {
        let mut state = Self {
            graph,
            delta,
            lowest_costs: HashMap::from([(
                source,
                LowestCost {
                    cost: G::Cost::zero(),
                    predecessor: source,
                },
            )]),
            buckets: BTreeMap::new(),
        };

        state.process_next_bucket(vec![source]);
        state.process_buckets(target);
        state.retrieve_result(source, target)
    }

    fn retrieve_result(
        &self,
        source: G::Node,
        target: G::Node,
    ) -> Option<(VecDeque<G::Node>, G::Cost)> {
        let cost = self.lowest_costs.get(&target)?;
        let mut path = VecDeque::from([target]);

        let mut current = target;
        while let Some(cost) = self.lowest_costs.get(&current) {
            let predecessor = cost.predecessor;
            path.push_front(predecessor);
            if predecessor == source {
                break;
            } else {
                current = predecessor;
            }
        }

        debug_assert!(path.front() == Some(&source));

        Some((path, cost.cost))
    }

    fn process_buckets(&mut self, target: G::Node) {
        let mut current_bucket_id = 0;
        while let Some((bucket_id, bucket)) = self.pop_next_bucket() {
            debug_assert!(bucket_id > current_bucket_id);
            current_bucket_id = bucket_id;

            self.process_next_bucket(bucket);

            if self.lowest_costs.contains_key(&target) {
                return;
            }
        }
    }

    fn pop_next_bucket(&mut self) -> Option<(usize, Vec<G::Node>)> {
        self.buckets.pop_first()
    }

    fn process_next_bucket(&mut self, mut bucket: Vec<G::Node>) {
        while self.process_current_bucket(&mut bucket) {}

        self.process_bucket_future_neighbors(bucket);
    }

    fn process_current_bucket(&mut self, current_bucket: &mut Vec<G::Node>) -> bool {
        let delta = self.delta;
        let edges = self.get_neighbors_from_nodes(current_bucket, |cost| cost <= delta);

        let mut updated = false;
        for edge in edges {
            updated = updated || self.update_current_bucket_neighbor(current_bucket, &edge);
        }
        updated
    }

    fn update_current_bucket_neighbor(
        &mut self,
        pending_bucket: &mut Vec<G::Node>,
        neighbor: &Edge<G>,
    ) -> bool {
        let new_cost = self.update_neighbor_cost(neighbor);
        if new_cost {
            pending_bucket.push(neighbor.target);
            true
        } else {
            false
        }
    }

    fn process_bucket_future_neighbors(&mut self, current_bucket_acc: Vec<G::Node>) {
        let delta = self.delta;
        let edges = self.get_neighbors_from_nodes(&current_bucket_acc, |cost| cost > delta);

        for edge in edges {
            self.update_future_bucket_neighbor(&edge);
        }
    }

    /**
       Update a neighbor edge that belongs to a future bucket.

       If the neighbor node has a lower total cost than the current lowest cost, it is added to the
       future bucket to be processed later, based on the bucket ID that is derived from the delta value.
    */
    fn update_future_bucket_neighbor(&mut self, edge: &Edge<G>) {
        let new_cost = self.update_neighbor_cost(edge);
        if new_cost {
            let bucket_id = G::floor_cost(edge.total_cost / self.delta);
            let bucket = self.buckets.entry(bucket_id).or_default();
            bucket.push(edge.target);
        }
    }

    /**
       Update the lowest cost to a given neighbor edge, if the total cost in the given edge is
       lower than the current lowest cost. Returns true if an update was made.
    */
    fn update_neighbor_cost(&mut self, edge: &Edge<G>) -> bool {
        let entry = self.lowest_costs.entry(edge.target);
        match entry {
            Entry::Occupied(mut entry) => {
                let current_cost = entry.get_mut();
                if edge.total_cost < current_cost.cost {
                    *current_cost = LowestCost {
                        cost: edge.total_cost,
                        predecessor: edge.source,
                    };
                    true
                } else {
                    false
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(LowestCost {
                    cost: edge.total_cost,
                    predecessor: edge.source,
                });
                true
            }
        }
    }

    /**
       In parallel, get the neighbors of the given nodes, together with the total cost to reach them.

       A predicate function is given to filter the neighbor nodes based on their cost from the given node in `nodes`.
    */
    fn get_neighbors_from_nodes(
        &self,
        nodes: &[G::Node],
        predicate: impl Fn(G::Cost) -> bool + Send + Sync,
    ) -> Vec<Edge<G>> {
        // Create a mutex of the result edges as a sink for each parallel task to write to.
        // We choose this approach instead of using `ParallelIterator`'s `collect` or `collect_vec_list`,
        // because benchmark shows that this results in better performance.
        let sink = Arc::new(Mutex::new(Vec::new()));

        // Use a `HashSet` to deduplicate nodes in the `nodes` list.
        // We use this approach instead of passing a `HashSet` directly, because benchmark shows that
        // it is faster to insert the nodes into a `Vec` and then only deduplicate them just before processing.
        HashSet::<G::Node>::from_iter(nodes.iter().cloned())
            .par_iter()
            .for_each(|node| self.get_neighbors_from_node(node, sink.clone(), &predicate));

        // Take the result edges out from the mutex sink and return them.
        let mut edges = sink.lock().unwrap();
        core::mem::take(&mut *edges)
    }

    /**
       Get the neighbors from a single node. This is called from [`get_neighbors_from_nodes`] by each parallel task.

       A mutex sink is given to store the neighbor edges result. A predicate function is given to filter the neighbors
       based on their cost from `node`.
    */
    fn get_neighbors_from_node(
        &self,
        node: &G::Node,
        sink: Arc<Mutex<Vec<Edge<G>>>>,
        predicate: impl Fn(G::Cost) -> bool,
    ) {
        // Get the current lowest cost from the global source to the given `node`.
        let current_cost = self.lowest_costs.get(node).unwrap();

        if let Some(neighbors) = self.graph.get_neighbors(node) {
            let mut sink = sink.lock().unwrap();
            for (neighbor, cost) in neighbors {
                if predicate(cost) {
                    // The total cost to the given neighbor node is the current cost to the `node`
                    // plus the cost from `node` to the `neighbor`.
                    let total_cost = current_cost.cost + cost;

                    sink.push(Edge {
                        source: *node,
                        target: neighbor,
                        total_cost,
                    });
                }
            }
        }
    }
}
