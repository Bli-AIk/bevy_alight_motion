use bevy::prelude::*;

pub fn apply_mask_clipping_system(
    _playback: Res<crate::animation::AmPlayback>,
    _query: Query<(
        &GlobalTransform,
        &ChildOf,
        &crate::scene::AmMaskInfo,
        &mut Visibility,
        &crate::scene::AmLayerMarker,
    )>,
    _parent_query: Query<&GlobalTransform>,
) {
    // Disabled: using shader-based mask clipping instead
}
