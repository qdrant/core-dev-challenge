use num_traits::Zero;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
    costs: BTreeMap<G::Node, G::Cost>,
    predecessors: BTreeMap<G::Node, G::Node>,
    buckets: BTreeMap<usize, BTreeSet<G::Node>>,
}

struct Edge<G: Graph> {
    source: G::Node,
    target: G::Node,
    cost: G::Cost,
    total_cost: G::Cost,
}

struct BucketResult<G: Graph> {
    same_bucket_neighbors: Vec<Edge<G>>,
    future_buckets_neighbors: Vec<Edge<G>>,
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
            costs: BTreeMap::from([(source, G::Cost::zero())]),
            predecessors: BTreeMap::new(),
            buckets: BTreeMap::new(),
        };

        state.process_next_bucket(BTreeSet::from([source]));
        state.process_buckets(target);
        state.retrieve_result(source, target)
    }

    fn retrieve_result(
        &mut self,
        source: G::Node,
        target: G::Node,
    ) -> Option<(VecDeque<G::Node>, G::Cost)> {
        let &cost = self.costs.get(&target)?;
        let mut path = VecDeque::from([target]);

        let mut current = target;
        while let Some(&predecessor) = self.predecessors.get(&current) {
            path.push_front(predecessor);
            if predecessor == source {
                break;
            } else {
                current = predecessor;
            }
        }

        debug_assert!(path.front().cloned() == Some(source));

        Some((path, cost))
    }

    fn process_buckets(&mut self, _target: G::Node) {
        let mut current_bucket_id = 0;
        while let Some((bucket_id, bucket)) = self.buckets.pop_first() {
            debug_assert!(bucket_id > current_bucket_id);
            current_bucket_id = bucket_id;

            self.process_next_bucket(bucket);
            // if self.costs.contains_key(&target) {
            //     return;
            // }
        }
    }

    fn process_next_bucket(&mut self, mut bucket: BTreeSet<G::Node>) {
        loop {
            bucket = self.process_bucket(bucket);
            if bucket.is_empty() {
                break;
            }
        }
    }

    fn process_bucket(&mut self, current_bucket: BTreeSet<G::Node>) -> BTreeSet<G::Node> {
        let results = current_bucket
            .into_par_iter()
            .map(|node| self.process_neighbors(&node))
            .collect_vec_list();

        let mut pending_bucket = BTreeSet::new();

        for result in results.iter().flatten().flatten() {
            for neighbor in result.same_bucket_neighbors.iter() {
                self.update_same_bucket_neighbor(&mut pending_bucket, neighbor);
            }
        }

        if pending_bucket.is_empty() {
            for result in results.iter().flatten().flatten() {
                for neighbor in result.future_buckets_neighbors.iter() {
                    self.update_future_bucket_neighbor(neighbor);
                }
            }
        }

        pending_bucket
    }

    fn update_same_bucket_neighbor(
        &mut self,
        pending_bucket: &mut BTreeSet<G::Node>,
        neighbor: &Edge<G>,
    ) {
        let new_cost = self.update_neighbor_cost(neighbor);
        if new_cost {
            self.predecessors.insert(neighbor.target, neighbor.source);
            pending_bucket.insert(neighbor.target);
        }
    }

    fn update_future_bucket_neighbor(&mut self, neighbor: &Edge<G>) {
        let new_cost = self.update_neighbor_cost(neighbor);
        if new_cost {
            self.predecessors.insert(neighbor.target, neighbor.source);
            let bucket_id = G::floor_cost(neighbor.total_cost / self.delta);
            let bucket = self.buckets.entry(bucket_id).or_default();
            bucket.insert(neighbor.target);
        }
    }

    fn update_neighbor_cost(&mut self, neighbor: &Edge<G>) -> bool {
        if let Some(current_cost) = self.costs.get_mut(&neighbor.target) {
            println!(
                "Updating cost for node {:?} from {:?} to {:?}",
                neighbor.target, current_cost, neighbor.total_cost
            );

            if neighbor.total_cost < *current_cost {
                *current_cost = neighbor.total_cost;
                true
            } else {
                false
            }
        } else {
            println!(
                "Inserting new node {:?} with cost {:?}",
                neighbor.target, neighbor.total_cost
            );

            self.costs.insert(neighbor.target, neighbor.total_cost);
            true
        }
    }

    fn process_neighbors(&self, node: &G::Node) -> Option<BucketResult<G>> {
        let current_cost = self.costs.get(node).cloned().unwrap();

        if let Some(neighbors) = self.graph.get_neighbors(node) {
            let (same_bucket_neighbors, future_buckets_neighbors) = neighbors
                .map(|(neighbor, cost)| Edge::<G> {
                    source: *node,
                    target: neighbor,
                    cost: cost,
                    total_cost: current_cost + cost,
                })
                .partition::<Vec<_>, _>(|neighbor| neighbor.cost <= self.delta);

            Some(BucketResult {
                same_bucket_neighbors,
                future_buckets_neighbors,
            })
        } else {
            None
        }
    }
}
