// Manages non-voxel objects on the grid.  These aren't meshed.

pub trait Prop {

}

// Do we want to return values from Collide, or just take action?
pub trait Interactable {
    // 
    fn Collide(&self, direction: Vec3) -> ();
}

#[derive(Component)]
pub struct GridProp {
    //pub model: ?,
    // pub size: IVec3,
    // pub orientation: ?,
    // pub flip:
}