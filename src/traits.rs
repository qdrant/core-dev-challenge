use core::fmt::Debug;
use core::ops::{Add, Div};

use num_traits::Float;

pub trait Graph: Sized + Send + Sync + Debug {
    type Node: Send + Sync + Copy + Debug + Ord;
    type Cost: Send
        + Sync
        + Copy
        + Debug
        + Float
        + Default
        + PartialOrd
        + Add<Output = Self::Cost>
        + Div<Output = Self::Cost>;

    fn get_neighbors(
        &self,
        node: &Self::Node,
    ) -> Option<impl Iterator<Item = (Self::Node, Self::Cost)>>;

    fn floor_cost(cost: Self::Cost) -> usize;
}

pub type NodeOf<G> = <G as Graph>::Node;
pub type CostOf<G> = <G as Graph>::Cost;

pub trait HasGraph {
    type Graph: Graph;

    fn graph(&self) -> &Self::Graph;
}
