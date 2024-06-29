// World chunking
// It's impossible to store large worlds in RAM, so worlds are chunked.
// Chunking stores big worlds on disk.

use std::vec;
use std::collections::{BTreeMap,BTreeSet};
use bevy::math::IVec3;
use rayon::prelude::*;

#[derive(Default)]
struct ChunkLocation {
    location: u16,
}

impl ChunkLocation{
    fn new() -> Self {
        ChunkLocation {
            location: 0,
        }
    }

    pub fn location(&self) -> u16 {
        self.location
    }

    pub fn set_location(&mut self, x: u8, y: u8, z: u8) {
        self.set_x(x);
        self.set_y(y);
        self.set_z(z);
    }
    
    pub fn x(&self) -> u8 {
        (self.location >> 10) & 0b1_1111
    }

    pub fn y(&self) -> u8 {
        (self.location >> 5) & 0b1_1111
    }

    pub fn z(&self) -> u8 {
        self.location & 0b1_1111
    }

    pub fn set_x(&mut self, x: u8) {
        self.location = (self.location & 0b0_00000_11111_11111) | ((x as u16 & 0b1_1111) << 10);
    }

    pub fn set_y(&mut self, y: u8) {
        self.location = (self.location & 0b0_11111_00000_11111) | ((y as u16 & 0b1_1111) << 5);
    }

    pub fn set_z(&mut self, z: u8) {
        self.location = (self.location & 0b0_11111_11111_00000) | (z as u16 & 0b1_1111);
    }
}
// Chunk manager provides an available chunk ID
// Component of a given grid
#[derive(Component)]
struct ChunkManager {
    counter: u16,
    free_list: Vec<u16>,
}

impl ChunkManager{
    fn new() -> Self {
        ChunkManager {
            counter: 0,
            free_list: Vec::new(),
        }
    }

    fn allocate(&mut self) -> u16 {
        if let Some(id) = self.free_list.pop() {
            id
        } else {
            // self.counter++, but rust doesn't implement ++
            let id = self.counter;
            self.counter += 1;
            id
        }
    }

    fn deallocate(&mut self, id: u16) {
        self.free_list.push(id);
    }
}

// Component of a given grid
// FIXME:  Replace ChunkObjects with specific types
// voxels:  meshable voxels, keyed by location in the chunk
// props:  non-meshable props
// delta:  list of changed locations
#[derive(Component)]
struct GridChunk {
    location: IVec3,
    id: u16,
    voxels: BTreeMap<u16, (VoxelKind, ChunkLocation)>,
    props: BTreeMap<u16, ChunkObjects>,
    delta: Vec<(ChunkLocation, Option<VoxelKind>, ChunkLocation)>,
}

impl GridChunk {
    pub fn new(&mut manager: ChunkManager, &loc: IVec3) -> Self {
        GridChunk {
            location: loc,
            id: manager.allocate(),
            voxels: BTreeMap::new(),
            props: BTreeMap::new(),
            delta: Vec::new(),
        }
    }

    // Use a greedy mesher to merge the deltas into the voxel map.  Do this
    // before storing to disk!
    fn encode_voxels(&mut self) {
        let mut voxel_kinds: BTreeSet<VoxelKind> = BTreeSet::new();

        // Make note of all the unique kinds of voxels in this chunk.
        // We will use this when completing greedy meshing.
        for (kind, s) in voxels.values() {
            voxel_kinds.insert(kind);
        }
        for (l, kind, s) in delta.iter() {
            voxel_kinds.insert(kind);
        }

        // First, merge the deltas in and greedy mesh to create the new RLE
        let mut new_voxel_rle: BTreeMap<u16, (VoxelKind, ChunkLocation)> = BTreeMap::new();
        for kind in voxel_kinds.par_iter() {
            let mut xy_kind = [[0u32; 32]; 32];
            let mut yz_kind = [[0u32; 32]; 32];
            let mut xz_kind = [[0u32; 32]; 32];
            for (k, (this_kind, r)) in self.voxels.iter() {
                if this_kind != kind {
                    continue;
                }
                let location: ChunkLocation = ChunkLocation { location: k };
                let rle: ChunkLocation = ChunkLocation { location: r };
                // These are run across x with a length of rle.x(), so create a
                // string of 1 bits rle.x long, then put the left-most bit at x
                let mask = ((2 << rle.x()) - 1) << location.x();

                // These need to propagate across all y and z locations
                for y in location.y()..=location.y()+rle.y() {
                    for z in location.z()..=location.z()+rle.z() {
                        xy_kind[z][y] |= mask;
                    }
                }
            }
            // Merge deltas
            for (k, this_kind, r) in self.deltas.iter() {
                if Some(this_kind) && this_kind != kind {
                    continue;
                }
                let location: ChunkLocation = ChunkLocation { location: k };
                let rle: ChunkLocation = ChunkLocation { location: r };
                // These are run across x with a length of rle.x(), so create a
                // string of 1 bits rle.x long, then put the left-most bit at x
                let mut mask: u32 = ((2 << rle.x()) - 1) << location.x();
                // If this_kind is empty, then delete these bits.
                if !Some(this_kind) {
                    mask = !mask;
                }

                // These need to propagate across all y and z locations
                for y in location.y()..=location.y()+rle.y() {
                    for z in location.z()..=location.z()+rle.z() {
                        xy_kind[z][y] |= mask;
                    }
                }
            }
            // TODO:  Greedy mesh and store in new_voxel_rle
        }

        self.voxels = new_voxel_rle;
    }

    // Performs binary meshing to create a mesh for the chunk.
    // Will eventually need some way to address cracks between chunks.
    fn create_mesh(&mut self) {
        let mut xy = [[0u32; 32]; 32];
        let mut yz = [[0u32; 32]; 32];
        let mut xz = [[0u32; 32]; 32];
        // Use the prepared voxel data for binary meshing
        // Thread for generating the xy array
        // These arrays are 4096 bytes and using one thread per array gives
        // better cache performance than breaking it into further threads
        // FIXME:  Also play back the deltas in chronological order.  The
        // greedy mesher will merge the deltas into the RLE.
        let voxels = self.voxels.iter();
        let xy_handle = thread::spawn(move || {
            for (k, (kind, r)) in voxels {
                let location: ChunkLocation = ChunkLocation { location: k };
                let rle: ChunkLocation = ChunkLocation { location: r };
                // These are run across x with a length of rle.x(), so create a
                // string of 1 bits rle.x long, then put the left-most bit at x
                let mask = ((2 << rle.x()) - 1) << location.x();

                // These need to propagate across all y and z locations
                for y in location.y()..=location.y()+rle.y() {
                    for z in location.z()..=location.z()+rle.z() {
                        xy[z][y] |= mask;
                    }
                }
            }
            xy
        });
    
        let voxels = self.voxels.iter();
        let yz_handle = thread::spawn(move || {
            for (k, (kind, r)) in voxels {
                let location: ChunkLocation = ChunkLocation { location: k };
                let rle: ChunkLocation = ChunkLocation { location: r };
                // These are run across x with a length of rle.x(), so create a
                // string of 1 bits rle.x long, then put the left-most bit at x
                let mask = ((2 << rle.y()) - 1) << location.y();

                // These need to propagate across all y and z locations
                for x in location.x()..=location.x()+rle.x() {
                    for z in location.z()..=location.z()+rle.z() {
                        yz[x][z] |= mask;
                    }
                }
            }
            yz
        });

        let voxels = self.voxels.iter();
        let xz_handle = thread::spawn(move || {
            for (k, (kind, r)) in voxels {
                let location: ChunkLocation = ChunkLocation { location: k };
                let rle: ChunkLocation = ChunkLocation { location: r };
                // These are run across x with a length of rle.x(), so create a
                // string of 1 bits rle.x long, then put the left-most bit at x
                let mask = ((2 << rle.x()) - 1) << location.x();

                // These need to propagate across all y and z locations
                for y in location.y()..=location.y()+rle.y() {
                    for z in location.z()..=location.z()+rle.z() {
                        xz[y][z] |= mask;
                    }
                }
            }
            xz
        });
    
        // Reclaim ownership of the arrays
        let xy = xy_handle.join().unwrap();
        let yz = yz_handle.join().unwrap();
        let xz = xz_handle.join().unwrap();

        // TODO: Make mesh
    }
}

