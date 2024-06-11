// World chunking
// It's impossible to store large worlds in RAM, so worlds are chunked.
// Chunking stores big worlds on disk.

use std::vec;
use std::collections::BTreeMap;
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
#[derive(Component)]
struct GridChunk {
    location: IVec3,
    id: u16,
    voxels: BTreeMap<u16, ChunkObject>,
    props: BTreeMap<u16, ChunkObjects>,
}

impl GridChunk {
    pub fn new(&mut manager: ChunkManager, &loc: IVec3) -> Self {
        GridChunk {
            location: loc,
            id: manager.allocate(),
            voxels: BTreeMap::new(),
            props: BTreeMap::new(),
        }
    }

    // TODO:  Perform the greedy meshing, discard the xy yz xz arrays,
    // move this to some appropriate place.
    // Not worried about extra faces appearing between chunks for now, maybe
    // improve on that later.
    fn gen_mesh_map(&self) {
        let mut xy = [[0u32; 32]; 32];
        let mut yz = [[0u32; 32]; 32];
        let mut xz = [[0u32; 32]; 32];
    
        // Collect keys from voxels to avoid multiple borrow issues
        let voxels_keys: Vec<_> = self.voxels.keys().cloned().collect();

        // Thread for generating the xy array
        let xy_handle = thread::spawn(move || {
            for k in &voxels_keys {
                let location: ChunkLocation = ChunkLocation { location: *k };
                let x = location.x();
                let y = location.y();
                let z = location.z();
                xy[z][y] |= 1 << (31 - x);
            }
            xy
        });
    
        // Thread for generating the yz array
        let yz_handle = thread::spawn(move || {
            for k in &voxels_keys {
                let location: ChunkLocation = ChunkLocation { location: *k };
                let x = location.x();
                let y = location.y();
                let z = location.z();
                yz[x][z] |= 1 << (31 - y);
            }
            yz
        });
    
        // Thread for generating the xz array
        let xz_handle = thread::spawn(move || {
            for k in &voxels_keys {
                let location: ChunkLocation = ChunkLocation { location: *k };
                let x = location.x();
                let y = location.y();
                let z = location.z();
                xz[y][z] |= 1 << (31 - x);
            }
            xz
        });
    
        // Reclaim ownership of the arrays
        let xy = xy_handle.join().unwrap();
        let yz = yz_handle.join().unwrap();
        let xz = xz_handle.join().unwrap();
    
        // TODO: Greedy meshing using TanTan's binary greedy meshing algorithm
    }
}

// chunk:  the ID of the chunk.  Up to 2^16 chunks can be loaded, and when
// unloaded the given chunk ID is freed up.
// local_position:  the position inside the chunk
// FIXME:  Probably not needed since we have everything in the chunk itself
#[derive(Component)]
struct ChunkObject {
    chunk: u16,
    local_position: u8,
}