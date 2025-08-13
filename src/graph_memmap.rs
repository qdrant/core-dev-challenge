use memmap2::MmapMut;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::OpenOptions;
use std::io;

#[derive(Debug)]
pub struct Graph {
    mmap: MmapMut,
}

#[derive(Debug, Clone, PartialEq)]
struct State {
    cost: f64,
    position: u64,
}

impl Eq for State {}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap()
    }
}

impl Graph {
    // First u64 is vertex id
    // Second u64 is the neighbor count
    // Third u64 is offset for neighbors
    //
    // Alignment isn't there for u64s so bytes will need to be casted
    fn vertices_neighbor_counts_offsets(&self) -> &[[[u8; std::mem::size_of::<u64>()]; 3]] {
        // First bytes of the mmap are the vertex count
        let vertex_count =
            u64::from_le_bytes(self.mmap[0..std::mem::size_of::<u64>()].try_into().unwrap())
                as usize;

        bytemuck::cast_slice(
            &self.mmap[std::mem::size_of::<u64>()
                ..(std::mem::size_of::<u64>() + vertex_count * 3 * std::mem::size_of::<u64>())],
        )
    }

    // Neighbor count and offset into the memmap for neighbors/costs
    fn neighbor_count_offset(&self, v: u64) -> (usize, usize) {
        let vertices_neighbor_counts_offsets = self.vertices_neighbor_counts_offsets();

        let i = vertices_neighbor_counts_offsets
            .binary_search_by(
                |&[vertex_bytes, _neighbor_count_bytes, _offset_bytes]: &[[u8; std::mem::size_of::<u64>()]; 3]| {
                    let x = u64::from_le_bytes(vertex_bytes);
                    x.cmp(&v)
                },
            )
            .unwrap();

        let neighbor_count = u64::from_le_bytes(vertices_neighbor_counts_offsets[i][1]) as usize;
        let offset = u64::from_le_bytes(vertices_neighbor_counts_offsets[i][2]) as usize;

        (neighbor_count, offset)
    }

    // First u64 is neighbor id
    // Second u64 is the cost (needs to be casted to an f64)
    fn neighbors_costs(&self, v: u64) -> &[[u64; 2]] {
        let (neighbor_count, offset) = self.neighbor_count_offset(v);

        let neighbors_costs = &self.mmap[offset
            ..(offset
                + neighbor_count * (std::mem::size_of::<u64>() + std::mem::size_of::<f64>()))];

        bytemuck::cast_slice(neighbors_costs)
    }

    // First u64 is neighbor id
    // Second u64 is the cost (needs to be casted to an f64)
    fn neighbors_costs_mut(&mut self, v: u64) -> &mut [[u64; 2]] {
        let (neighbor_count, offset) = self.neighbor_count_offset(v);

        let neighbors_costs = &mut self.mmap[offset
            ..(offset
                + neighbor_count * (std::mem::size_of::<u64>() + std::mem::size_of::<f64>()))];

        bytemuck::cast_slice_mut(neighbors_costs)
    }

    pub fn vertices(&self) -> impl Iterator<Item = u64> {
        self.vertices_neighbor_counts_offsets().iter().map(
            |&[vertex_bytes, _neighbor_count_bytes, _offset_bytes]: &[[u8; std::mem::size_of::<u64>()]; 3]| {
                u64::from_le_bytes(vertex_bytes)
            },
        )
    }

    pub fn neighbors(&self, v: u64) -> impl Iterator<Item = (u64, f64)> {
        self.neighbors_costs(v)
            .iter()
            .copied()
            .map(|[neighbor, cost]| (neighbor, f64::from_le_bytes(cost.to_le_bytes())))
    }

    pub fn get_edge_weight(&self, from: u64, to: u64) -> Option<f64> {
        let neighbors_costs = self.neighbors_costs(from);
        if let Ok(i) = neighbors_costs.binary_search_by(|[neighbor, _]| neighbor.cmp(&to)) {
            let cost = neighbors_costs[i][1];
            Some(f64::from_le_bytes(cost.to_le_bytes()))
        } else {
            None
        }
    }

    pub fn set_edge_weight(&mut self, from: u64, to: u64, cost: f64) -> bool {
        let neighbors_costs = self.neighbors_costs_mut(from);
        if let Ok(i) = neighbors_costs.binary_search_by(|[neighbor, _]| neighbor.cmp(&to)) {
            neighbors_costs[i][1] = u64::from_le_bytes(cost.to_le_bytes());
            true
        } else {
            false
        }
    }

    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        Ok(Self { mmap })
    }

    pub fn save_to_file(self) -> io::Result<()> {
        self.mmap.flush()
    }

    pub fn shortest_path(&self, start: u64, end: u64) -> Option<(Vec<u64>, f64)> {
        if !self.vertices().any(|v| v == start) || !self.vertices().any(|v| v == end) {
            return None;
        }

        let mut prev_dist: HashMap<u64, (u64, f64)> = HashMap::new();
        let mut heap = BinaryHeap::new();

        prev_dist.insert(start, (u64::MAX, 0.0));
        heap.push(State {
            cost: 0.0,
            position: start,
        });

        while let Some(State { cost, position }) = heap.pop() {
            if position == end {
                let mut path = vec![end];
                let mut current = end;
                while let Some(&(p, _)) = prev_dist.get(&current)
                    && p != u64::MAX
                {
                    path.push(p);
                    current = p;
                }
                path.reverse();
                return Some((path, cost));
            }

            if cost > prev_dist[&position].1 {
                continue;
            }

            for (neighbor, weight) in self.neighbors(position) {
                let next = State {
                    cost: cost + weight,
                    position: neighbor,
                };
                if next.cost
                    < *prev_dist
                        .get(&neighbor)
                        .map(|(_, c)| c)
                        .unwrap_or(&f64::INFINITY)
                {
                    prev_dist.insert(neighbor, (position, next.cost));
                    heap.push(next);
                }
            }
        }

        None
    }
}
