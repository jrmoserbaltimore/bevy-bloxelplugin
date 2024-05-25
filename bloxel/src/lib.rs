#![forbid(unsafe_code)]

//! Voxel block world for Bevy

use bevy_app::prelude::*;
use bevy_asset::{Asset, AssetApp};
use bevy_ecs::prelude::*;

#[derive(Default)]
pub struct BloxelPlugin {
    dimension: f32,
}

impl Plugin for BloxelPlugin {
    fn build(&self, app: &mut App) {
        // Default dimension is 1m cubes
        self.dimension = 1;
    }
}

/// Returns this [`BloxelPlugin`] with the dimension set
impl BloxelPlugin {
    pub fn with_dimension(mut self, dimension: f32) {
        self.dimension = dimension;
        self
    }
}