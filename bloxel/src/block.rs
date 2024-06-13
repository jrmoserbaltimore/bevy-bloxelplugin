// Manages voxel blocks on the grid.  These can be meshed by the greedy mesher.

use bevy::math::IVec3;

// GridVoxel is a block on the grid.  They are subject to the greedy mesher.
#[derive(Component)]
pub struct GridVoxel;

// Filter Without<GridVoxelUnmeshable> in the greedy mesher.
// This can include voxels that are altered by cutting them.
#[derive(Component)]
pub struct GridVoxelUnmeshable;

#[derive(Default)]
pub struct VoxelKind {
    // Need a way to indicate what texture to use
    // including textures that change with size
    // pub texture: ?,
}

#[derive(Default)]
pub struct GridObjectKind {
    pub hardness: u8,
    // pub on_destruct: callback,
    // pub model: ?,
    // pub skin: ?, // Texture
    // pub name:  str,
}

// Objects with health are set up as such:
//   - The object type defines its maximum HP
//   - The object has a HitPoints component when it has less than full HP
// For voxels, this saves 1 byte per voxel versus assigning a 1-byte HP value
// to every voxel object.  A voxel only has HP when it's damaged, and in games
// where voxels are only temporarily damaged the memory to store HP data is
// released after several seconds.
#[derive(Component)]
pub struct HitPoints(u16);