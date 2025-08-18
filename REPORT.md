# Report

## Base version improvements

Single-thread version was improved in the following ways:

1. Instead of std's binary heap, `dary_heap::QuaternaryHeap` was used.  It
   utilises cache better, and gives significant performance for
   `Graph::shortest_path`.
2. `Graph::adjacency` uses a hashmap with much simpler hasher from crate
   `nohash_hasher`.  `rustc-hash` can also be used, but its speedup is not as
   large.  It improves both `Graph::random_connected_graph` and
   `Graph::shortest_path`.
3. Presuming that vertex IDs are in range 0..n, vectors are used to store
   weights and previous vertex index.

   We may reorganize the API, keep an `bimap` table to keep a correspondence
   between arbitrary vertex IDs and positions, or just require that the IDs
   cannot be sparse.

|                                             | old       | new       |         |
|---------------------------------------------|-----------|-----------|---------|
|shortest path on random connected graph, ARM | 3.4230 µs | 1.1198 µs | -67.452%|
| -// -, x64                                  | 6.3577 µs | 1.9425 µs | -69.664%|
|generate random connected graph, ARM         | 18.312 µs | 10.796 µs | -41.080%|
| -//-, x64                                   | 24.573 µs | 14.935 µs | -39.254%|

(ARM: MacOS M1, x64: Intel(R) Xeon(R) Platinum 8168 CPU @ 2.70GHz at DigitalOcean).

## Parallel version

The parallel version consists of `threads` workers each having own priority
queue (the very same `dary_heap::QuaternaryHeap`).  They share the same
read-only `Graph` instance, and also `costs` (i.e. `dist`), a slice of custom
`&[AtomicF64]` type, `prev` of `&[AtomicIsize]`, and `locks` of `&[AtomicBool]`
for coordinated update of the two formers.

The idea of working with these values is:

1. Reading atomically an element of `costs` with relaxed ordering is safe
   because the cost is never increasing.  If we get a stale value, worst thing
   that would happen is trying to lock one of the `locks` only to found that
   actual value is smaller.  But it is a rare event.
2. Reading of `prev` only happens after all the threads are finished their work.
   Updating `prev` is done together with `costs` under lock `locks[i]` that
   induce "happens before" for both fields.

The workers's queues are also guarded by an `AtomicBool`, but most of the time
they do work with own data uncontended.  However, when a worker has no work, it
tries to steal some work from other worker, locking on his queue, in round-robin
fashion (it also happens at start: only the main thread gets a seed value).

When a worker runs out of work, it increases an atomic `waitings` counter. When
the counter is equal to number of threads, all the threads except the main one,
terminate.

### Benchmarking parallel version

Parallel version makes sense only on larger graph; unfortunately, large graph's
search time varies greatly depending on graph's structure. To mitigate this
problem, 5 random graphs are generated, and each implementation is run on all of
them.

|                | ARM        | x64       |
|----------------|------------|-----------|
| seq-5x         | 199.76 ms  | 455.39 ms |
| par-5x         | 113.67 ms  | 308.58 ms |
| speedup        | 1.7        | 1.47      |
| efficiency     | 0.43       | 0.38      |

(ARM: MacOS M1, x64: Intel(R) Xeon(R) Platinum 8168 CPU @ 2.70GHz at DigitalOcean).

The efficiency is way below 1 is not only because of synchronization costs, but
because threads, having the priority queue split, do calculations for suboptimal paths
too. But it is still faster overall.
