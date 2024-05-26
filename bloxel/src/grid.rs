#![forbid(unsafe_code)]

// Grids are finite field discrete spaces aligned to integers.
// A grid can be on a grid; otherwise, the grid is aligned such that
// (0,0,0) occurs at (0,0,0) in the world, i=(1,0,0), j=(0,1,0), and
// k=(0,0,1).

use bevy::prelude::Query;
use bevy::math::IVec3;
use bevy::ecs::entity::Entity;

#[derive(Component)]
struct Grid;

// To find an entity in space, look up the associated grid and add its
// position.  Grid 0 is the world grid; for any other grid, get the
// GridPosition component of that grid and add its position parameter.  If that
// GridPosition component references a grid other than 0, repeat this, until
// finding a grid whose GridPosition is on grid 0.
#[derive(Component)]
struct GridPosition {
    position: IVec3,
    grid: Option<Entity>,
}

// world_position() gives the absolute position in 3D space.  If a grid unit
// is ever not equal to a world unit, world_position() will return the adjusted
// position().
// TODO: Need a way to change what grid the entity is on
trait GridEntity {
    fn position(
        &self,
        query: &Query<(&Grid, &GridPosition)>) -> IVec3;
    fn local_position(&self) -> IVec3;
    fn relative_position(&self,
        query: &Query<(&Grid, &GridPosition)>,
        grid: &Entity) -> IVec3;
    fn world_position(
        &self,
        query: &Query<(&Grid, &GridPosition)>) -> Vec3;
    fn set_position(&mut self, position: IVec3);
}

impl GridEntity for GridPosition {
    fn position(
        &self,
        query: &Query<(&Grid, &GridPosition)>) -> IVec3 {
        match self.grid {
            Some(grid_entity) => {
                if let Ok((_, parent_position)) = query.get(grid_entity) {
                    self.position + parent_position.position(query)
                } else {
                    panic!("Attempted to locate Entity on non-existent grid.");
                }
            }
            None => self.position,
        }
    }
    fn local_position(&self) -> IVec3 {
        self.position
    }
    fn relative_position(
        &self,
        query: &Query<(&Grid, &GridPosition)>,
        grid: &Entity) -> IVec3 {
        if let OK((_, grid_position)) = query.get(grid) {
            self.position() - grid_position.position()
        } else {
            panic!("Attempted to locate Entity relative to non-existent grid.");
        }
        
    }
    fn world_position(
        &self,
        query: Query<(&Grid, &GridPosition)>) -> Vec3 {
        self.position(query).as_vec3()
    }
    fn set_position(&mut self, position: IVec3) {
        self.position = position;
    }
}