# Qdrant Core Dev Challenge Solution

# Summary

This solution is developed by Soares Chen to improve the shortest path algorithm that is implemented in the original code base.

For the challenge, I have specifically chose to implement a parallel version of the shortest path algorithm using the delta-stepping algorithm. This document outlines the improvement that I have made, and the benchmarking results.

# Motivation

I chose to work on parallelizing the algorithm, as it provides relatively clear potential benefits while facing unknown requirements, compared to other options available. For instance, the performance gained the parallel algorithm become clear relatively early on, when there are around 640,000 vertices in a graph. This allows me to run relatively simple benchmarks to demonstrate the impact of my work.

On the other hand, the other options such as working with a graph from disk requires more assumptions to be made about the requirements. For instance, since modern computers have gigabytes of memory, it is difficult to justify not loading an entire graph into memory, if it only consume less than a gigabyte of memory. And while there are clear benefits of persisting a graph on disk, the majority of such work would relate more to building a database than improving a graph algorithm.

Furthermore, if we tweak the algorithm to accomodate concurrent read/write or lazy loading from disk, it would likely result in significant performance overhead. Hence if we don't know whether performance is a priority, it wouldn't be clear whether such effort would be considered an improvement.

# Algorithm Overview

The delta-stepping algorithm is implemented at [`parallel_shortest_path.rs`](src/parallel_shortest_path.rs). At a high level, the algorithm works with a `delta` value that divides nodes into multiple buckets based on their tentative lowest cost divided by `delta`.

The algorithm repeatedly process the lowest bucket, until no new neighbor is found to fit in that bucket, before moving on to the next bucket. The processing of each bucket is split into multiple rounds for light edges, i.e. neighbors that may be added back to the same bucket. After that, another round of processing is done for the heavy edges, i.e. for all neighbors in the finalized bucket with cost higher than `delta`.

The main parallelism that is achieved with this algorithm is that it is able to retrieve all neighbors of a given list of nodes, parallelized by each node in the list. With this, the performance improvement gets higher as the number of nodes in each bucket grows.

## Previous Attempts

The implementation of the delta-stepping algorithm follows closely to the original algorithm described on the Internet. When implementing the algorithm, I have also tried several approaches to achieve more parallelism. However, my benchmark showed that these attempts resulted in worse performance, so I reverted the changes and keep the implementation close to the original algorithm. In this section, I will briefly describe some of my attempts.

I tried to update the lowest cost for the node neighbors inside the parallel task, however the performance drops due to lock contention. Even when I tried to use fine grained locks, the performance is still not as good as updating them sequentially. It may still be possible to improve performance with a different granularity for the locks, such as grouping the locks based on the vertex ID. But I didn't have the time to try them out.

I tried to process both light and heavy edges within the same round of parallel processing. However this resulted in worse performance. This is likely due to there being many rounds of processing for the light edges, and that statistically there are much more heavy edges than light edges. So grouping the two processing together likely resulted in too many unnecessary processing of the heavy edges.

# Other Improvements

Aside from implementing the parallel shortest path algorithm, I have also made a few minor improvements to the code base, which I will briefly describe below.

## Forbid Negative Weights

I have modified the graph methods to forbid negative weights (costs) to be added. This is because both the sequential and parallel versions of the algorithm can only work correctly when there is no negative weight.

## Terminology

In my implementation, I used the terms "node" and "cost" instead of "vertex" and "weight", as I find them fit better with my mental model while coding. I leave the terminology used in the final submission, to avoid inconsistencies in case I missed any renaming.

In practice, when working with a team, I would use this as an opportunity to discuss with the team about the appropriate terminology to be used, and rename all alternative terms to the canonical term before merging the pull request.

## `Graph` Trait

I have defined a simple `Graph` trait to allow the parallel graph algorithm to work with any concrete graph data structure. For example, one can have an alternative `Graph` implementation that retrieves the node neighbors from disk instead of from memory. The trait also allows different `Node` and `Cost` types to be used. With that, we can for example define a newtype wrapper for `f64` to enforce that no negative weight value can be created.

# Benchmark

To test the performance of the parallel algorithm, I have modified the existing benchmark code, and make it produce benchmark comparison between the parallel algorithm and the original sequential algorithm.

The benchmark result shows that the parallel algorithm only surpasses the sequential algorithm when there are a significant number of vertices and edges in the graph, i.e. with at least half a million vertices. In particular, parallel algorithm significantly outperforms the sequential algorithm after around 2 million vertices.

The parallel algorithm also performs better with very dense graph, with there being at least as many edges as the number of vertices. This is because this would result in better parallelism, with more neighbor nodes to be processed in parallel for each bucket.

This shows that the parallel algorithm does not always perform better in all use cases. When working with smaller graphs or sparse graphs, it may be better to simply use the sequential algorithm.

The parallel algorithm also performs better if the nodes are evenly distributed to around 50 buckets. The number of buckets depend on the delta value, and the weight statistics. If there are too few buckets, it may result in too many nodes to be processed during the sequential parts of the parallel algorithm. If there are too many buckets, it may result in too few nodes in each bucket to take advantage of the parallelism. Ultimately, this shows that a good heuristic is needed to determine an appropriate delta value depending on the specific graph.

## Considerations

In each run of the benchmark, a new random graph is generated with the specified parameters. The same graph is then used to benchmark both the parallel and sequential algorithms.

Because different graphs are generated between each run of the benchmark, they may result in very different time measurement even when the parameters are different. This is because a random graph may contain shorter path that can be reached much sooner by the algorithm. As a result, the important thing to observe is the relative performance between the sequential and parallel algorithms.

If we want the random graphs to produce consistent performance, we may need better strategy for how the random graph is generated. For example, after the graph is generated, we may want to perform few rounds of shortest path calculation, and then add or remove edges on the graph to ensure that the number of nodes and total cost in the shortest path stay consistent for each random graph generation.

## Hardware Specs

The benchmark is run on my local desktop computer with 4 cores. The parallel algorithm may have better performance on computers with more cores, but I didn't have the time to test that out.

## Benchmark Results

I have manually run a few benchmarks shown below. The benchmark is run with increasing number of vertices, with either 2x or 4x number of edges. The benchmark is run with two different delta values for the parallel algorithm, resulting in either 50 or 100 buckets. All results are measured in milliseconds, with lower values being better.

Due to time limit, I didn't automate the benchmark or run it with more parameters. I also didn't plot the results onto graphs, which would help visualize how the performance changes based on the parameters. Nevertheless, I hope the table provides sufficient details to understand the performance of the parallel algorithm.

| Vertices  | Edges       | Sequential | Parallel (50 buckets) | Parallel (100 buckets) |
|-----------|-------------|------------|-----------------------|------------------------|
|    80,000 | x2          |   44.6     |   86.5                |  119.0                 |
|    80,000 | x2          |   36.3     |   61.9                |   86.5                 |
|    80,000 | x4          |   34.8     |   51.1                |   59.7                 |
|    80,000 | x4          |   59.4     |   89.7                |   99.8                 |
|   160,000 | x2          |   88.0     |  112.6                |  142.1                 |
|   160,000 | x2          |   53.6     |   71.4                |   89.2                 |
|   160,000 | x4          |  128.3     |  174.2                |  196.4                 |
|   160,000 | x4          |   61.4     |   77.2                |   90.3                 |
|   320,000 | x2          |  171.8     |  169.0                |  198.5                 |
|   320,000 | x2          |  260.5     |  282.3                |  306.6                 |
|   320,000 | x4          |  130.9     |  138.3                |  149.8                 |
|   320,000 | x4          |   75.5     |   81.3                |  100.4                 |
|   640,000 | x2          |  398.5     |  323.4                |  353.0                 |
|   640,000 | x2          |  217.9     |  197.0                |  221.4                 |
|   640,000 | x4          |  844.4     |  721.8                |  749.6                 |
|   640,000 | x4          |  775.1     |  688.3                |  711.4                 |
| 1,280,000 | x2          | 1012.3     |  706.7                |  758.2                 |
| 1,280,000 | x2          | 1422.3     | 1086.4                | 1139.3                 |
| 1,280,000 | x4          |  515.2     |  409.6                |  426.9                 |
| 1,280,000 | x4          | 2323.3     | 1719.8                | 1794.1                 |
| 2,560,000 | x2          | 2742.1     | 1718.9                | 1778.6                 |
| 2,560,000 | x2          | 2252.5     | 1384.1                | 1431.9                 |