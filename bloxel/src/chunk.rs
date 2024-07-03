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
    voxels: BTreeMap<PackedXYZ32, (&VoxelKind, PackedXYZ32)>,
    //rotations: BTreeMap?
    props: BTreeMap<PackedXYZ32, ChunkObjects>,
    delta: Vec<(PackedXYZ32, Option<&VoxelKind>, PackedXYZ32)>,
}

// xy plane:
//       x->
//  ^ u32
//  | u32
//  y ...
//
// Find the first left face and right face across x, create a mask (m), then
// along y check if xy[y][z] * m = xy[y][z].
// Once the y dimension is known, check along z to see how far xy[y][z] fits.
// The result gives a start and length for a box.

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
    fn build_bitmap(&mut uv: [[u32; 32]; 32], location: PackedXYZ32, &k: Option<&VoxelKind>, rle: PackedXYZ32, &axis: str, clear: bool) {
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
        for v in location_v..=location_v+rle_v {
            for w in location_w()..=location_w+rle_w {
                match clear {
                    false => uv[v][w] |= mask,
                    true => uv[v][w] &= !mask,
                }
            }
        }
    }

    // Use a greedy mesher to merge the deltas into the voxel map.  Do this
    // before storing to disk!
    // This should have phenomenal cache performance.
    fn encode_voxels(&mut self) {
        let mut voxel_kinds: BTreeSet<&VoxelKind> = BTreeSet::new();

        // Make note of all the unique kinds of voxels in this chunk.
        // We will use this when completing greedy meshing.
        for (kind, _) in voxels.values() {
            voxel_kinds.insert(Some(kind));
        }
        for (_, kind, _) in delta.iter() {
            voxel_kinds.insert(kind);
        }

        // Threaded by kind of voxel being meshed.  Each thread's entire
        // working set fits easily in L1 cache, 4-way set associative caches
        // should never invalidate any of the working set.
        let mut new_voxel_rle: BTreeMap<PackedXYZ32, (&VoxelKind, PackedXYZ32)> = BTreeMap::new();
        for kind in voxel_kinds.par_iter() {
            let mut xy = [[0u32; 32]; 32];
            for (l, (this_kind, r)) in self.voxels.iter() {
                if Some(this_kind) != kind {
                    continue;
                }
                build_bitmap(&mut xy, l, this_kind, r, "xy", false);
            }
            // Merge deltas; this_kind=None means delete that volume
            for (l, this_kind, r) in self.deltas.iter() {
                if this_kind.is_some() && this_kind != kind {
                    continue;
                }
                build_bitmap(&mut xy, l, this_kind, r, "xy", !this_kind.is_some());
            }
            // Binary mesh and store in new_voxel_rle
            
            let mut xy_left = [[0u32; 32]; 32];
            let mut xy_right = [[0u32; 32]; 32];

            // Build the face tables
            for y in 0..32 {
                for z in 0..32 {
                    xy_left[y][z] = xy[y][z] & !(xy[y][z] >> 1);
                    xy_right[y][z] = xy[y][z] & !(xy[y][z] << 1);
                }
            }

            // lzcnt and clz instructions on modern processors probably let us
            // use leading_zeroes() and skip cleaning up xy[][], but using
            // leading_zeroes() requires multiple instruction for every check
            // versus just bitwise AND followed by jz.  The code is simpler
            // using masking, but xy[][] is 6.25% of D$.  This whole nested
            // loop easily fits in cache.
            // We iterate along each xy plane in z
            for z in 0..32 {
                for y in 0..32 {
                    // Work across the current row until it's empty
                    while xy[y][z] != 0 {
                        // Find the first span
                        let x: u8 = xy_left[y][z].leading_zeroes();
                        let x_end: u8 = xy_right[y][z].leading_zeroes();
                        // Set bits spanning that and left-align to left face
                        let mask = (1 << (x_end - x)) << (31 - x);
                        let x_pos_mask = 1 << (31-x);
                        let x_end_pos_mask = 1 << (31-x_end);
                        // Check each step along the y axis to make a triangle
                        let mut y_length: u8 = 0;
                        let mut z_length: u8 = 0;
                        while y+y_length < 31 {
                            // If there's an area spanning these columns on
                            // the next row up, expand the rectangle and do
                            // accounting
                            if xy[y+y_length+1][z] & mask == mask {
                                y_length += 1;
                            } else {
                                break;
                            }
                        }
                        // Next we need to check the *entire* rectangle
                        // along each z plane
                        while z+z_length < 31 {
                            let z_extend: bool = true;
                            for yn in y..=y+y_length+1 {
                                if xy[yn][z+z_length+1] & mask != mask {
                                    z_extend = false;
                                    break;
                                }
                            }
                            if z_extend {
                                z_length += 1;
                            } else {
                                break;
                            }
                        }
                        // Finally, clear out bits and move all faces
                        for zc in z..=z+z_length+1 {
                            for yc in y..=y+y_length+1 {
                                // clear those bits out
                                xy[yc][zc] &= !mask;
                                // Move the left face if the right side isn't a face
                                if (xy_rght[yc][zc] & x_end_pos_mask) == 0 {
                                    xy_left[yc][zc] |= x_end_pos_mask;
                                }
                                xy_left[yc][zc] &= !x_pos_mask;
                            }
                        }
                        // add the RLE box to new_voxel_rle
                        let mut origin = PackedXYZ32(0);
                        let mut rle = PackedXYZ32(0);
                        origin.set_location(x, y as u8, z as u8);
                        rle.set_location(x + x_length, y + y_length as u8, z + z_length as u8);
                        new_voxel_rle.insert(origin, (kind.unwrap(), rle));
                    }
                }
            }
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
                    build_bitmap(&mut uv, l, Some(kind), r, plane, false);
                }
                // Merge deltas
                for (l, this_kind, r) in deltas {
                    build_bitmap(&mut uv, l, this_kind, r, plane, !this_kind.is_some);
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

