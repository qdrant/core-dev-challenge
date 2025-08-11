# Qdrant Core Dev Challenge Solution

## Summary

This solution is developed by Soares Chen to improve the shortest path algorithm that is implemented in the original code base.

For the challenge, I have specifically chose to implement a parallel version of the shortest path algorithm using the delta-stepping algorithm.

## Benchmark


| Vertices  | Extra Edges | Sequential | Parallel (50 buckets) | Parallel (100 buckets) |
|-----------|-------------|------------|-----------------------|------------------------|
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