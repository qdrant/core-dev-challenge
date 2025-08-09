use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::traits::Graph;

pub struct State<G: Graph> {
    graph: G,
    delta: G::Cost,
    costs: BTreeMap<G::Node, G::Cost>,
    predecessors: BTreeMap<G::Node, G::Node>,
    buckets: BTreeMap<usize, BTreeSet<G::Node>>,
}

struct Neighbor<G: Graph> {
    predecessor: G::Node,
    node: G::Node,
    cost: G::Cost,
    total_cost: G::Cost,
}

struct BucketResult<G: Graph> {
    same_bucket_neighbors: Vec<Neighbor<G>>,
    future_buckets_neighbors: Vec<Neighbor<G>>,
}

impl<G: Graph> State<G> {
    pub fn shortest_distance(
        graph: G,
        source: G::Node,
        target: G::Node,
    ) -> Option<(VecDeque<G::Node>, G::Cost)> {
        let mut state = Self {
            graph,
            delta: G::Cost::default(),
            costs: BTreeMap::new(),
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
        let mut path = VecDeque::new();
        while let Some(&predecessor) = self.predecessors.get(&target) {
            path.push_front(predecessor);
        }

        debug_assert!(path.front().cloned() == Some(source));

        Some((path, cost))
    }

    fn process_buckets(&mut self, target: G::Node) {
        while let Some((_bucket_id, bucket)) = self.buckets.pop_first() {
            self.process_next_bucket(bucket);
            if self.costs.contains_key(&target) {
                return;
            }
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

        for result in results.into_iter().flatten().flatten() {
            for neighbor in result.same_bucket_neighbors {
                self.update_same_bucket_neighor(&mut pending_bucket, &neighbor);
            }

            for neighbor in result.future_buckets_neighbors {
                self.update_future_bucket_neighor(&neighbor);
            }
        }

        pending_bucket
    }

    fn update_same_bucket_neighor(
        &mut self,
        pending_bucket: &mut BTreeSet<G::Node>,
        neighbor: &Neighbor<G>,
    ) {
        let new_cost = self.update_neighbor_cost(neighbor);
        if new_cost {
            pending_bucket.insert(neighbor.node);
        }
    }

    fn update_future_bucket_neighor(&mut self, neighbor: &Neighbor<G>) {
        let new_cost = self.update_neighbor_cost(neighbor);
        if new_cost {
            self.predecessors
                .insert(neighbor.node, neighbor.predecessor);

            let bucket_id = G::floor_cost(neighbor.total_cost / self.delta);

            let bucket = self.buckets.entry(bucket_id).or_default();
            bucket.insert(neighbor.node);
        }
    }

    fn update_neighbor_cost(&mut self, neighbor: &Neighbor<G>) -> bool {
        if let Some(current_cost) = self.costs.get_mut(&neighbor.node) {
            if neighbor.total_cost < *current_cost {
                *current_cost = neighbor.total_cost;
                true
            } else {
                false
            }
        } else {
            self.costs.insert(neighbor.node, neighbor.total_cost);
            true
        }
    }

    fn process_neighbors(&self, node: &G::Node) -> Option<BucketResult<G>> {
        let current_cost = self.costs.get(node).cloned().unwrap_or_default();

        if let Some(neighbors) = self.graph.get_neighbors(node) {
            let (same_bucket_neighbors, future_buckets_neighbors) = neighbors
                .iter()
                .map(|(neighbor, cost)| Neighbor::<G> {
                    predecessor: *node,
                    node: *neighbor,
                    cost: *cost,
                    total_cost: current_cost + *cost,
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
