use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use bevy::prelude::*;

use crate::scene::AmLayerMarker;

fn trace_layer_z_once(key: impl Into<String>, message: impl FnOnce() -> String) {
    if std::env::var_os("AM_LAYER_Z_TRACE").is_none() {
        return;
    }

    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let key = key.into();

    let should_log = {
        let mut guard = seen.lock().expect("layer z trace mutex poisoned");
        guard.insert(key)
    };

    if should_log {
        bevy::log::warn!("{}", message());
    }
}

pub fn debug_layer_global_z_system(
    query: Query<(
        Entity,
        &AmLayerMarker,
        &Transform,
        &GlobalTransform,
        Option<&ChildOf>,
    )>,
) {
    for (entity, marker, transform, global_transform, child_of) in query.iter() {
        let interesting = marker.label == "编组 2"
            || marker.label == "编组 2 Copy"
            || marker.label == "Rectangle 1 Copy"
            || marker.label == "Rectangle 1 Copy 3"
            || marker.label == "Rectangle 1 Copy 2"
            || marker.label == "spr_s_boneloop_0.png Copy";
        if !interesting {
            continue;
        }

        let parent = child_of.map(|c| c.parent());
        let global = global_transform.translation();
        trace_layer_z_once(format!("{}:{}", marker.id, marker.label), || {
            format!(
                "[LAYER-Z] entity={:?} layer_id={} label='{}' parent={:?} local_z={:.6} global_z={:.6} local_xy=({:.2},{:.2}) global_xy=({:.2},{:.2})",
                entity,
                marker.id,
                marker.label,
                parent,
                transform.translation.z,
                global.z,
                transform.translation.x,
                transform.translation.y,
                global.x,
                global.y,
            )
        });
    }
}
