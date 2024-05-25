
use bevy::math::UVec3;

pub trait Prop {

}

// Do we want to return values from Collide, or just take action?
pub trait Interactable {
    // 
    fn Collide(&self, direction: Vec3) -> ();
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
    pub on_grid: bool,
    pub rendered: bool,
    pub kind: u16,
}

use bevy::math::UVec3;

pub trait Prop {

}

// Do we want to return values from Collide, or just take action?
pub trait Interactable {
    // 
    fn Collide(&self, direction: Vec3) -> ();
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
    pub on_grid: bool,
    pub rendered: bool,
    pub kind: u16,
}