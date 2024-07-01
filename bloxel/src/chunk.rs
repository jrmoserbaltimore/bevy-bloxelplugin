// World chunking
// It's impossible to store large worlds in RAM, so worlds are chunked.
// Chunking stores big worlds on disk.

use std::vec;
use std::collections::{BTreeMap,BTreeSet};
use bevy::math::IVec3;
use rayon::prelude::*;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct PackedXYZ32(u16);

impl PackedXYZ32{
    pub fn set_location(&mut self, x: u8, y: u8, z: u8) {
        self.set_x(x);
        self.set_y(y);
        self.set_z(z);
    }
    
    pub fn x(&self) -> u8 {
        (self.0 >> 10) & 0b1_1111
    }

    pub fn y(&self) -> u8 {
        (self.0 >> 5) & 0b1_1111
    }

    pub fn z(&self) -> u8 {
        self.0 & 0b1_1111
    }

    pub fn set_x(&mut self, x: u8) {
        self.0 = (self.0 & 0b0_00000_11111_11111) | ((x as u16 & 0b1_1111) << 10);
    }

    pub fn set_y(&mut self, y: u8) {
        self.0 = (self.0 & 0b0_11111_00000_11111) | ((y as u16 & 0b1_1111) << 5);
    }

    pub fn set_z(&mut self, z: u8) {
        self.0 = (self.0 & 0b0_11111_11111_00000) | (z as u16 & 0b1_1111);
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
    voxels: BTreeMap<u16, (&VoxelKind, PackedXYZ32)>,
    props: BTreeMap<u16, ChunkObjects>,
    delta: Vec<(PackedXYZ32, Option<&VoxelKind>, PackedXYZ32)>,
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

    // This became horribly repetitive so it's a nested function now
    fn build_bitmap(&mut uv: [[u32; 32]; 32], l: u16, &k: Option<&VoxelKind>, r: u16, &axis: str, clear: bool) {
        let location: PackedXYZ32 = PackedXYZ32(l);
        let rle: PackedXYZ32 = PackedXYZ32(r);

        let (rle_u, rle_v, rle_w) = match axis {
            "xy" => (rle.x(), rle.y(), rle.z()),
            "yz" => (rle.y(), rle.z(), rle.x()),
            "xz" => (rle.x(), rle.z(), rle.y()),
            _ => unreachable!(),
        };
        let (location_u, location_v, location_w) = match axis {
            "xy" => (location.x(), location.y(), location.z()),
            "yz" => (location.y(), location.z(), location.x()),
            "xz" => (location.x(), location.z(), location.y()),
            _ => unreachable!(),
        };

        // These are run across u with a length of rle_u, so create a
        // string of `1` bits rle_u long, then put the left-most bit at u
        let mask: u32 = ((2 << rle_u) - 1) << location_u;

        // These need to propagate across all v and w locations
        for u in location_v..=location_u+rle_u {
            for w in location_w()..=location_w+rle_w {
                match clear {
                    false => uv[w][v] |= mask,
                    true => uv[w][y] &= !mask,
                }
            }
        }
    }

    // Use a greedy mesher to merge the deltas into the voxel map.  Do this
    // before storing to disk!
    fn encode_voxels(&mut self) {
        let mut voxel_kinds: BTreeSet<&VoxelKind> = BTreeSet::new();

        // Make note of all the unique kinds of voxels in this chunk.
        // We will use this when completing greedy meshing.
        for (kind, s) in voxels.values() {
            voxel_kinds.insert(kind);
        }
        for (l, kind, s) in delta.iter() {
            voxel_kinds.insert(kind);
        }

        // Threaded by kind of voxel being meshed
        let mut new_voxel_rle: BTreeMap<u16, (&VoxelKind, PackedXYZ32)> = BTreeMap::new();
        for kind in voxel_kinds.par_iter() {
            let mut xy = [[0u32; 32]; 32];
            let mut yz = [[0u32; 32]; 32];
            let mut xz = [[0u32; 32]; 32];
            for (uv, plane) in ((xy, "xy"), (yz, "yz"), (xz, "xz")) {
                for (l, (this_kind, r)) in self.voxels.iter() {
                    if this_kind != kind {
                        continue;
                    }
                    build_bitmap(uv, l, this_kind, r, plane, false);
                }
                // Merge deltas
                for (l, this_kind, r) in self.deltas.iter() {
                    if this_kind.is_some() && this_kind != Some(kind) {
                        continue;
                    }
                    build_bitmap(uv, l, this_kind, r, plane, !this_kind.is_some);
                }
            }
            // TODO:  Greedy mesh and store in new_voxel_rle
        }

        self.voxels = new_voxel_rle;
    }

    // Performs binary meshing to create a mesh for the chunk.
    // Will eventually need some way to address cracks between chunks.
    fn create_mesh(&mut self) {
        // Use the prepared voxel data for binary meshing
        // These arrays are 4096 bytes and using one thread per array gives
        // better cache performance than breaking it into further threads
        let handles = Vec::new();
        for plane in ("xy", "yz", "xz") {
            let voxels = self.voxels.iter();
            let deltas = self.deltas.iter();
            let mut uv = [[0u32; 32]; 32];
            let uv_handle = thread::spawn(move || {
                for (l, (kind, r)) in voxels {
                    build_bitmap(uv, l, Some(kind), r, plane, false);
                }
                // Merge deltas
                for (l, this_kind, r) in deltas {
                    build_bitmap(uv, l, this_kind, r, plane, !this_kind.is_some);
                }
                uv
            });
            handles.push(uv_handle);
        }

        let xz = handles.pop().join().unwrap();
        let yz = handles.pop().join().unwrap();
        let xy = handles.pop().join().unwrap();

        // TODO: Make mesh
    }
}

