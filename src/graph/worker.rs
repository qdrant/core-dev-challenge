use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};

use crate::atomic::{AtomicF64, AtomicLockGuard, AtomicMutex, AtomicSemaphoreGuard};
use super::State;

// 4096 bytes should be enough for everone.
#[derive(Debug)]
#[repr(align(4096))]
pub(crate) struct Worker {
    // Important invariant for the queue: the only the owning worker pushes to it, but anyone can pop (steal) from it.
    queue: AtomicMutex<super::TheHeap<State>>,
    size: AtomicUsize,
    pub(crate) steal_attempts: AtomicUsize,
    pub(crate) steal_loops: AtomicUsize,
}

impl Worker {
    pub(crate) fn new() -> Self {
        Self {
            queue: AtomicMutex::new(super::TheHeap::new()),
            size: AtomicUsize::new(0),
            steal_attempts: AtomicUsize::new(0),
            steal_loops: AtomicUsize::new(0),
        }
    }

    fn push(&self, state: State) {
        self.size.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.queue.lock();
        guard.push(state);
    }

    fn pop(&self) -> Option<State> {
        self.size.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.queue.lock();
        guard.pop()
    }

    fn clear(&self) {
        let mut guard = self.queue.lock();
        guard.clear();
        self.size.store(0, Ordering::Relaxed);
    }

    fn try_pop(&self) -> Option<State> {
        if self.size.load(Ordering::Relaxed) == 0 {
            return None;
        }
        self.queue
            .try_lock()
            .and_then(|mut guard| guard.pop())
            .inspect(|_| {
                // If we successfully popped, we should decrease the size.
                self.size.fetch_sub(1, Ordering::Relaxed);
            })
    }
}

struct Node<'search> {
    // the lock determines "happens after relationship" for the rest of the fields.
    // the other fields are atomics to be able to read them without a lock and write without UnsafeCell.
    lock: &'search AtomicBool,
    cost: &'search AtomicF64,
    prev: &'search AtomicI64,
}

impl<'search> Node<'search> {
    pub fn new(search: &'search Search, position: u64) -> Self {
        let position = position as usize;
        Self {
            lock: &search.locks[position],
            cost: &search.costs[position],
            prev: &search.prev[position],
        }
    }

    pub(crate) fn try_to_supercede(&self, cost: f64, prev: u64) -> bool {
        if cost < self.cost.load(Ordering::Relaxed) {
            let _cell_guard = AtomicLockGuard::lock(self.lock);
            if cost < self.cost.load(Ordering::Relaxed) {
                // we are still better!
                self.cost.store(cost, Ordering::Relaxed);
                self.prev.store(prev as i64, Ordering::Relaxed);
                return true;
            }
        }

        false
    }
}

#[derive(Debug)]
pub(crate) struct Search<'graph> {
    graph: &'graph super::Graph,
    waiters: AtomicU64,
    start: u64,
    end: u64,

    // See the `Node` struct comments for some explanations.
    locks: Box<[AtomicBool]>,
    pub(crate) costs: Box<[AtomicF64]>,
    pub(crate) prev: Box<[AtomicI64]>,
}

impl<'ctx> Search<'ctx> {
    pub fn new(graph: &'ctx super::Graph, start: u64, end: u64) -> Self {
        let size = graph.adjacency.len() + 1;
        let mut locks = Vec::with_capacity(size);
        let mut costs = Vec::with_capacity(size);
        let mut prev = Vec::with_capacity(size);
        for _ in 0..size {
            locks.push(AtomicBool::new(false));
            costs.push(AtomicF64::new(f64::INFINITY));
            prev.push(AtomicI64::new(-1));
        }
        costs[start as usize].store(0.0, Ordering::Relaxed);

        Self {
            graph,
            waiters: AtomicU64::new(0),
            start,
            end,
            locks: locks.into_boxed_slice(),
            costs: costs.into_boxed_slice(),
            prev: prev.into_boxed_slice(),
        }
    }

    pub fn start_work(&'ctx self, id: usize, workers: &'ctx [Worker]) {
        workers[0].push(State {
            cost: 0.0,
            position: self.start,
        });
        self.run_worker(id, workers);
    }

    pub fn run_worker(&'ctx self, id: usize, workers: &'ctx [Worker]) {
        let mut steals = 0;
        let mut steal_loops = 0;
        let me = &workers[id];
        let mut stolen_state = None;

        'main: loop {
            // Try to do all the work we have
            'work: loop {
                let state = match stolen_state.take() {
                    Some(state) => state,
                    None => {
                        // If we have no local state, try to pop from our queue.
                        if let Some(state) = me.pop() {
                            state
                        } else {
                            // If we have no local work, we may try to steal it.
                            break 'work;
                        }
                    }
                };

                let end_cost = self.costs[self.end as usize].load(Ordering::Relaxed);
                if state.cost >= end_cost {
                    me.clear();
                    break 'work;
                }

                let node = Node::new(self, state.position);
                // if we are still relevant, i.e. our cost is still the best known.
                if state.cost <= node.cost.load(Ordering::Relaxed) {
                    for (&neighbor, &weight) in self
                        .graph
                        .adjacency
                        .get(&state.position)
                        .into_iter()
                        .flatten()
                    {
                        let next_cost = state.cost + weight;
                        let nei_node = Node::new(self, neighbor);

                        if nei_node.try_to_supercede(next_cost, state.position) {
                            let next = State {
                                cost: next_cost,
                                position: neighbor,
                            };
                            me.push(next);
                        }
                    }
                }
            }

            // Try to steal some work.
            let waiting_guard = AtomicSemaphoreGuard::increment(
                &self.waiters,
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
            if let Some(state) = self.steal_some_work(id, workers, &mut steals, &mut steal_loops) {
                // else handle the new work
                std::mem::drop(waiting_guard);
                // we do not put it into our worker queue to avoid repeated stealing.
                stolen_state = Some(state);
                continue 'main;
            } else {
                // If we are here, all the threads are waiting for work, i.e. there is no more work, and we are
                // terminating. Keep the counter incremented to avoid a race conditions.
                std::mem::forget(waiting_guard);
                workers[id].steal_attempts.fetch_add(steals, Ordering::Relaxed);
                workers[id]
                    .steal_loops
                    .fetch_add(steal_loops, Ordering::Relaxed);
                return;
            }
        }
    }

    fn steal_some_work(
        &'ctx self,
        id: usize,
        workers: &'ctx [Worker],
        steals: &mut usize,
        steal_loops: &mut usize,
    ) -> Option<State> {
        *steal_loops += 1;
        let workers_len = workers.len();
        while self.waiters.load(Ordering::Relaxed) != workers_len as u64 {
            for n_offset in 1..workers_len {
                *steals += 1;
                let neighbor_id = (id + n_offset) % workers_len;
                if let Some(state) = workers[neighbor_id].try_pop() {
                    return Some(state);
                }
                std::thread::yield_now();
            }
        }
        None
    }
}
