//! Emits one-shot debug traces for suspicious runtime layer Z values.
//! 对可疑的运行时图层 Z 值输出一次性调试日志。
//!
//! This module is intentionally diagnostic-only. It helps inspect how collected layers, parenting,
//! and runtime transforms combine into global depth ordering, without mixing that instrumentation
//! into the main animation systems.
//! 这个模块是纯调试用途。它用来观察收集结果、父子继承和运行时变换是如何组合成最终全局深度排序的，
//! 同时避免把这些观测逻辑混进正式动画系统里。

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
