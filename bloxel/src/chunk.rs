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
// voxels:  meshable voxels, keyed by location in the chunk
// props:  non-meshable props
// culled:  Bitmap of voxels deleted from memory, bit position = x,
//          u32 = y, plane of u32 = z
// delta:  list of changed locations
#[derive(Component)]
struct GridChunk {
    location: IVec3,
    id: u16,
    voxels: BTreeMap<u16, (VoxelKind, ChunkLocation)>,
    props: BTreeMap<u16, ChunkObjects>,
    delta: Vec<u16>,
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

    // TODO:  Perform the greedy meshing, discard the xy yz xz arrays,
    // move this to some appropriate place.
    // Not worried about extra faces appearing between chunks for now, maybe
    // improve on that later.
    fn gen_mesh_map(&mut self) {
        let mut xy = [[0u32; 32]; 32];
        let mut yz = [[0u32; 32]; 32];
        let mut xz = [[0u32; 32]; 32];

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
    
        // TODO: Greedy meshing using TanTan's binary greedy meshing algorithm

        // TODO: T-junctions will create cracks.  Generate a set of boxes here
        // resolving T-junctions.  Generate the mesh from that first, then from
        // the voxel data excluding any locations of resolved boxes.  Hitboxes
        // and on-disk storage use the unresolved voxel data.
    }
}

