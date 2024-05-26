#![forbid(unsafe_code)]

//! Voxel block world for Bevy

use bevy::app::{Plugin,App};

#[derive(Default)]
pub struct BloxelPlugin {
}

impl Plugin for BloxelPlugin {
    fn build(&self, app: &mut App) {
    }
}

/// Returns this [`BloxelPlugin`] with the dimension set
impl BloxelPlugin {
}
