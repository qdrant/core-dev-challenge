use core::fmt::Debug;

/**
   A minimal trait to represent a graph data structure to be used
   by generic graph algorithms such as `parallel shortest path`.
*/
pub trait Graph: Sized + Send + Sync + Debug {
    /**
       The node (vertex) type of the graph.
    */
    type Node: Send + Sync + Copy + Clone + Debug;

    /**
       The cost (weight) type for the graph edges.
    */
    type Cost: Send + Sync + Copy + Clone + Debug;

    /**
       Get all outgoing neighbors from a given source node.
       Returns an iterator over the neighbors and their associated costs.

       This abstraction allows a graph implementation to retrieve the neighbors
       from different sources, such as from memory or from a persistent database.

       Note that the underlying graph data structure should not be modified when
       an algorithm such as shortest path is running, or else it may cause the
       algorithm to return incorrect results.

       As a result, the graph must not contains interior mutability that affects
       the result of `get_neighbors`. When the implementation is from a shared
       resource such as external database, the implementation may require additional
       features such as snapshotting to ensure that the neighbors
       do not change while the algorithm is running.
    */
    fn get_neighbors(
        &self,
        node: &Self::Node,
    ) -> Option<impl Iterator<Item = (Self::Node, Self::Cost)>>;
}
