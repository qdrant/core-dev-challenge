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

        state.process_bucket(vec![source]);
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

    /**
       Repeatedly process all nodes in all buckets, until all pending buckets have been processed.
    */
    fn process_buckets(&mut self, target: G::Node) {
        let mut current_bucket_id = 0;
        while let Some((bucket_id, bucket)) = self.buckets.pop_first() {
            debug_assert!(bucket_id > current_bucket_id);

            current_bucket_id = bucket_id;

            self.process_bucket(bucket);

            if self.lowest_costs.contains_key(&target) {
                return;
            }
        }
    }

    /**
       Process nodes that are stored in the given bucket.
    */
    fn process_bucket(&mut self, mut bucket: Vec<G::Node>) {
        /*
           Repeatedly process the neighbors of the nodes that belong to the current bucket.
           If there are new nodes added to the current bucket, then all nodes in the bucket
           has to be processed again.

           TODO: we may be able to skip processing some nodes in the current bucket,
           if their lowest cost is not updated.
        */
        while self.process_current_bucket(&mut bucket) {}

        /*
           The neighbors of the nodes that belong to the future buckets should be processed
           only later on.

           In theory, we may be able to process them together in the earlier step. However,
           benchmark shows that there might be significant overhead of doing this, due to
           `process_current_bucket` being repeated for many times until no new node is added
           to the current bucket.

           By processing the neighbors of the future buckets later on, we can reduce the overhead
           of processing them multiple times, even if this results in an additional round of processing.
        */
        self.process_future_bucket_neighbors(bucket);
    }

    /**
       Process neighbors of the given nodes that belong to the current bucket. Returns true if new nodes
       were added to the current bucket for further processing.S

       The neighbors are filtered based on whether the immediate cost to the neighbor from the given node
       is less than or equal to the delta value.
    */
    fn process_current_bucket(&mut self, current_bucket: &mut Vec<G::Node>) -> bool {
        let delta = self.delta;

        // Get the neighbors of the nodes in parallel
        let edges = self.get_neighbors_from_nodes(current_bucket, |cost| cost <= delta);

        /*
           We have to perform the update on the current bucket sequentially, as benchmark shows that
           there are too much overhead and insufficient parallelism gains when we try to do this in parallel
           within the earlier parallel retrieval of neighbors, due to lock contention in acquiring
           the RwLock for `lowest_costs`.
        */
        let mut updated = false;
        for edge in edges {
            updated = updated || self.update_current_bucket_neighbor(current_bucket, &edge);
        }
        updated
    }

    /**
       Process neighbors of the given nodes that belong to the future buckets.

       The neighbors are filtered based on whether the immediate cost to the neighbor from the given node
       is greater than the delta value.
    */
    fn process_future_bucket_neighbors(&mut self, nodes: Vec<G::Node>) {
        let delta = self.delta;

        // Get the neighbors of the nodes in parallel
        let edges = self.get_neighbors_from_nodes(&nodes, |cost| cost > delta);

        /*
           We have to perform the update on the future buckets sequentially, as benchmark shows that
           there are too much overhead and insufficient parallelism gains when we try to do this in parallel
           within the earlier parallel retrieval of neighbors, even when we make use of fine grained locks
           for each bucket.
        */
        for edge in edges {
            self.update_future_bucket_neighbor(&edge);
        }
    }

    /**
       Update a neighbor edge that belongs to the current bucket. Returns true if an update was made.

       If the neighbor node has a lower total cost than the current lowest cost, it is added to the
       current bucket to be processed again in the next iteration of processing the same bucket.
    */
    fn update_current_bucket_neighbor(
        &mut self,
        pending_bucket: &mut Vec<G::Node>,
        neighbor: &Edge<G>,
    ) -> bool {
        let new_cost = self.update_neighbor_cost(neighbor);
        if new_cost {
            pending_bucket.push(neighbor.target);
        }
        new_cost
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
