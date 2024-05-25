BloxelPlugin for Bevy
-

This plugin provides a voxel world for Bevy, similar to games like Minecraft,
Dragon Quest Builders, Cube World, Super Voxel World, and others.

# General goal

BloxelPlugin needs to do the following.

## World Grid

Create a world grid addressable a `IVec3`.  The world grid is for placed
objects such as blocks, not moving ones such as characters or projectiles.

`IVec3` is Bevy's `i32` 3-dimensional vector.  For a 1 meter cube, as in most
games, the grid should aligns to integer coordinates: (-32,15,7) is the `Vec3`
coordinate (-32,15,7).

The world grid does not scale.  For a 1/2 meter cube, these same coordinates
would be in the world at `Vec3` coordinate (-16,7.5,3.5); however, the player
interacts with the world grid by placing, destroying, and interacting with
objects on the grid, and placing extremely small objects aligned to an
extremely small grid would be difficult.  For games like Teardown this might
be interesting, but it's not a goal here.

Objects can be taken off the grid and moved in the physical world, then placed
back onto the grid when they settle.

## Manage blocks

Managing blocks on the world grid has performance implications.  A face of
5×5 blocks provides 25 cubic hitboxes.  These don't need collision checks with
eachother, but do need collision checks with everything else.  They do need
individual rendering, however.

Alternatively, cubes could be combined into single, larger composite objects.
This reduces the number of objects to render.  Although AABB is efficient, AABB
on a single cuboid rather than 25 cuboids is even more efficient.  On
collision, the location oft he collision must be ascertained, and the game must
behave as if the collision occurred with the single object in that coordinate
as if they weren't a single composite object.

Further, blocks can be removed from rendering if they are behind other blocks.
A 5×5 cube encases a 3x3 cube, and all blocks making up this 3x3 cube can be
deleted from the world as objects; their entities continue to exist, and the
ECM system still processes them, but they don't have a corresponding 3D shape
in the rendering pipeline.  Breaking a block occluding another block must both
reshape the composited object and instantiate a rendered object, either as
part of a new composite object or as its own separate object.

BloxelPlugin must handle all of this while the programmer only places objects
on the grid.

## Manage non-block objects

Besides blocks, objects like chairs, tables, fences, doors, and so forth are
placed on the grid.  These also need to be managed.  They may be interactable.

## Extensible objects

Some objects change shape when placed side by side.  For example, fence posts
may build out into a fence, and windows may become broad glass sheets.

BloxelPlugin must handle combining these objects and changing the models to
match.  This can be done by triggering actions controlled by the programmer; a
9-slice style generic approach should be provided as well.


