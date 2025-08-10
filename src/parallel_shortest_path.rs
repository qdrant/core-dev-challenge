use num_traits::Zero;
use rayon::prelude::*;
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
    buckets: BTreeMap<usize, HashSet<G::Node>>,
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
    is_tentative: bool,
}

impl<G: Graph> Clone for LowestCost<G> {
    fn clone(&self) -> Self {
        Self {
            cost: self.cost,
            predecessor: self.predecessor,
            is_tentative: self.is_tentative,
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
                    is_tentative: false,
                },
            )]),
            buckets: BTreeMap::new(),
        };

        state.process_next_bucket(HashSet::from([source]));
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

        debug_assert!(path.front().cloned() == Some(source));

        Some((path, cost.cost))
    }

    fn process_buckets(&mut self, target: G::Node) {
        let mut current_bucket_id = 0;
        while let Some((bucket_id, bucket)) = self.pop_next_bucket() {
            debug_assert!(bucket_id > current_bucket_id);
            current_bucket_id = bucket_id;

            self.process_next_bucket(bucket);

            if let Some(cost) = self.lowest_costs.get(&target)
                && !cost.is_tentative
            {
                return;
            }
        }
    }

    fn pop_next_bucket(&mut self) -> Option<(usize, HashSet<G::Node>)> {
        self.buckets.pop_first()
    }

    fn process_next_bucket(&mut self, mut bucket: HashSet<G::Node>) {
        loop {
            let new_nodes = self.process_current_bucket(&bucket);
            if new_nodes.is_empty() {
                break;
            } else {
                bucket.extend(new_nodes.into_iter());
            }
        }

        self.process_bucket_future_neighbors(bucket);
    }

    fn process_current_bucket(&mut self, current_bucket: &HashSet<G::Node>) -> HashSet<G::Node> {
        let delta = self.delta;
        let sink = Arc::new(Mutex::new(Vec::new()));

        current_bucket
            .par_iter()
            .for_each(|node| self.get_neighbors(node, sink.clone(), |cost| cost <= delta));

        let mut pending_bucket = HashSet::new();
        for edge in sink.lock().unwrap().drain(..) {
            self.update_same_bucket_neighbor(&mut pending_bucket, &edge);
        }

        pending_bucket
    }

    fn process_bucket_future_neighbors(&mut self, current_bucket_acc: HashSet<G::Node>) {
        let delta = self.delta;
        let sink = Arc::new(Mutex::new(Vec::new()));

        current_bucket_acc
            .par_iter()
            .for_each(|node| self.get_neighbors(node, sink.clone(), |cost| cost > delta));

        for edge in sink.lock().unwrap().drain(..) {
            self.update_future_bucket_neighbor(&edge);
        }
    }

    fn update_same_bucket_neighbor(
        &mut self,
        pending_bucket: &mut HashSet<G::Node>,
        neighbor: &Edge<G>,
    ) {
        let new_cost = self.update_neighbor_cost(neighbor, false);
        if new_cost {
            pending_bucket.insert(neighbor.target);
        }
    }

    fn update_future_bucket_neighbor(&mut self, neighbor: &Edge<G>) {
        let new_cost = self.update_neighbor_cost(neighbor, true);
        if new_cost {
            let bucket_id = G::floor_cost(neighbor.total_cost / self.delta);
            let bucket = self.buckets.entry(bucket_id).or_default();
            bucket.insert(neighbor.target);
        }
    }

    fn update_neighbor_cost(&mut self, neighbor: &Edge<G>, is_tentative: bool) -> bool {
        if let Some(current_cost) = self.lowest_costs.get_mut(&neighbor.target) {
            if neighbor.total_cost < current_cost.cost {
                *current_cost = LowestCost {
                    cost: neighbor.total_cost,
                    predecessor: neighbor.source,
                    is_tentative,
                };
                true
            } else {
                false
            }
        } else {
            self.lowest_costs.insert(
                neighbor.target,
                LowestCost {
                    cost: neighbor.total_cost,
                    predecessor: neighbor.source,
                    is_tentative,
                },
            );
            true
        }
    }

    fn get_neighbors(
        &self,
        node: &G::Node,
        sink: Arc<Mutex<Vec<Edge<G>>>>,
        filter: impl Fn(G::Cost) -> bool,
    ) {
        let current_cost = self.lowest_costs.get(node).unwrap();

        if let Some(neighbors) = self.graph.get_neighbors(node) {
            let mut sink = sink.lock().unwrap();
            for (neighbor, cost) in neighbors {
                if filter(cost) {
                    sink.push(Edge {
                        source: *node,
                        target: neighbor,
                        total_cost: current_cost.cost + cost,
                    });
                }
            }
        }
    }
}
