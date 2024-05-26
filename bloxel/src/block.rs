// Manages voxel blocks on the grid.  These can be meshed by the greedy mesher.

use bevy::math::IVec3;

// GridVoxel is a block on the grid.  They are subject to the greedy mesher.
#[derive(Component)]
pub struct GridVoxel;

#[derive(Default)]
pub struct GridVoxelKind {
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

// location is the (x,y,z) grid location
// type is the type of object
#[derive(Component)]
pub struct GridObject {
    pub location: IVec3,
    pub kind: u16,
}
