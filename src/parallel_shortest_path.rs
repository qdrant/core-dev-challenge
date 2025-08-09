use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

use crate::traits::Graph;

pub struct State<G: Graph> {
    pub graph: G,
    pub delta: G::Cost,
    pub costs: BTreeMap<G::Node, G::Cost>,
    pub predecessors: BTreeMap<G::Node, G::Node>,
    pub current_bucket_id: usize,
    pub current_bucket: BTreeSet<G::Node>,
    pub buckets: BTreeMap<usize, BTreeSet<G::Node>>,
}

pub struct Neighbor<G: Graph> {
    pub predecessor: G::Node,
    pub node: G::Node,
    pub cost: G::Cost,
    pub total_cost: G::Cost,
}

pub struct BucketResult<G: Graph> {
    pub same_bucket_neighbors: Vec<Neighbor<G>>,
    pub future_buckets_neighbors: Vec<Neighbor<G>>,
}

impl<G: Graph> State<G> {
    pub fn process_bucket(&mut self, bucket: BTreeSet<G::Node>) {
        let results = bucket
            .into_par_iter()
            .map(|node| self.process_neighbors(&node))
            .collect_vec_list();

        for result in results.into_iter().flatten().flatten() {
            for neighbor in result.same_bucket_neighbors {
                self.update_neighbor_of_node(&neighbor);
            }
        }
    }

    pub fn update_neighbor(&mut self, neighbor: &Neighbor<G>) -> bool {
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

    pub fn update_neighbor_of_node(&mut self, neighbor: &Neighbor<G>) {
        let new_cost = self.update_neighbor(neighbor);
        if new_cost {
            self.predecessors
                .insert(neighbor.node, neighbor.predecessor);
            let bucket_id = G::floor_cost(neighbor.total_cost / self.delta);

            if bucket_id == self.current_bucket_id {
                self.current_bucket.insert(neighbor.node);
            } else {
                let bucket = self.buckets.entry(bucket_id).or_default();
                bucket.insert(neighbor.node);
            }
        }
    }

    pub fn process_neighbors(&self, node: &G::Node) -> Option<BucketResult<G>> {
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
