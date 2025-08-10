use num_traits::Zero;
use rayon::{join, prelude::*};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, Mutex, RwLock},
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
    lowest_costs: Arc<RwLock<BTreeMap<G::Node, LowestCost<G>>>>,
    buckets: Arc<RwLock<BTreeMap<usize, Arc<Mutex<BTreeSet<G::Node>>>>>>,
}

#[derive(Debug)]
struct Edge<G: Graph> {
    source: G::Node,
    target: G::Node,
    cost: G::Cost,
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
        let state = Self {
            graph,
            delta,
            lowest_costs: Arc::new(RwLock::new(BTreeMap::from([(
                source,
                LowestCost {
                    cost: G::Cost::zero(),
                    predecessor: source,
                    is_tentative: false,
                },
            )]))),
            buckets: Arc::new(RwLock::new(BTreeMap::new())),
        };

        state.process_next_bucket(BTreeSet::from([source]));
        state.process_buckets(target);
        state.retrieve_result(source, target)
    }

    fn retrieve_result(
        &self,
        source: G::Node,
        target: G::Node,
    ) -> Option<(VecDeque<G::Node>, G::Cost)> {
        let lowest_costs = self.lowest_costs.read().unwrap();
        let cost = lowest_costs.get(&target)?;
        let mut path = VecDeque::from([target]);

        let mut current = target;
        while let Some(cost) = lowest_costs.get(&current) {
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

    fn process_buckets(&self, target: G::Node) {
        let mut current_bucket_id = 0;
        while let Some((bucket_id, bucket)) = self.pop_next_bucket() {
            debug_assert!(bucket_id > current_bucket_id);
            current_bucket_id = bucket_id;

            self.process_next_bucket(bucket);

            let lowest_costs = self.lowest_costs.read().unwrap();
            if let Some(cost) = lowest_costs.get(&target)
                && !cost.is_tentative
            {
                return;
            }
        }
    }

    fn pop_next_bucket(&self) -> Option<(usize, BTreeSet<G::Node>)> {
        let mut buckets = self.buckets.write().unwrap();
        let (bucket_id, bucket) = buckets.pop_first()?;
        let mut bucket = bucket.lock().unwrap();
        let bucket = core::mem::take(&mut *bucket);
        Some((bucket_id, bucket))
    }

    fn process_next_bucket(&self, mut bucket: BTreeSet<G::Node>) {
        loop {
            bucket = self.process_bucket(bucket);
            if bucket.is_empty() {
                break;
            }
        }
    }

    fn process_bucket(&self, current_bucket: BTreeSet<G::Node>) -> BTreeSet<G::Node> {
        let pending_bucket = Arc::new(Mutex::new(BTreeSet::new()));

        current_bucket
            .into_par_iter()
            .for_each(|node| self.process_neighbors(pending_bucket.clone(), &node));

        core::mem::take(&mut *pending_bucket.lock().unwrap())
    }

    fn update_same_bucket_neighbor(
        &self,
        pending_bucket: &mut BTreeSet<G::Node>,
        neighbor: &Edge<G>,
    ) {
        let new_cost = self.update_neighbor_cost(neighbor, false);
        if new_cost {
            pending_bucket.insert(neighbor.target);
        }
    }

    fn update_future_bucket_neighbor(&self, neighbor: &Edge<G>) {
        let new_cost = self.update_neighbor_cost(neighbor, true);
        if new_cost {
            let bucket_id = G::floor_cost(neighbor.total_cost / self.delta);
            let bucket = {
                let mut buckets = self.buckets.write().unwrap();
                buckets.entry(bucket_id).or_default().clone()
            };
            let mut bucket = bucket.lock().unwrap();
            bucket.insert(neighbor.target);
        }
    }

    fn update_neighbor_cost(&self, neighbor: &Edge<G>, is_tentative: bool) -> bool {
        let mut lowest_costs = self.lowest_costs.write().unwrap();
        if let Some(current_cost) = lowest_costs.get_mut(&neighbor.target) {
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
            lowest_costs.insert(
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

    fn process_neighbors(&self, pending_bucket: Arc<Mutex<BTreeSet<G::Node>>>, node: &G::Node) {
        let current_cost = {
            let lowest_costs = self.lowest_costs.read().unwrap();
            lowest_costs.get(node).cloned().unwrap()
        };

        if let Some(neighbors) = self.graph.get_neighbors(node) {
            let (current_neighbors, future_neighbors) = neighbors
                .into_iter()
                .map(|(neighbor, cost)| Edge::<G> {
                    source: *node,
                    target: neighbor,
                    cost: cost,
                    total_cost: current_cost.cost + cost,
                })
                .partition::<Vec<_>, _>(|edge| edge.cost <= self.delta);

            let task_a = || {
                let mut pending_bucket = pending_bucket.lock().unwrap();
                for edge in current_neighbors {
                    self.update_same_bucket_neighbor(&mut pending_bucket, &edge);
                }
            };

            let task_b = || {
                for edge in future_neighbors {
                    self.update_future_bucket_neighbor(&edge);
                }
            };

            join(task_a, task_b);
        }
    }
}
