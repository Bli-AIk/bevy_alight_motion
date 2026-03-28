//! Assembles the Bevy-side runtime for `bevy_alight_motion`.
//! It is the internal plugin wiring layer: material plugins, asset loaders,
//! core resources, startup hooks, lifecycle system ordering, animation updates,
//! and post-update cleanup are all registered here so the public plugin surface
//! can stay thin.
//!
//! 负责组装 `bevy_alight_motion` 的 Bevy 运行时。它是内部插件接线层：
//! 材质插件、资产加载器、核心资源、启动钩子、生命周期系统顺序、动画更新以及
//! 后处理清理都在这里注册，从而让公开插件入口保持精简。

use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

use crate::animation::{
    AmPlayback, advance_playback_system, animate_am_camera_system, animate_counter_system,
    animate_opacity_system, animate_path_repeat_system, animate_rtt_blur_system,
    animate_sdf_opacity_system, animate_sdf_repeat_system, animate_sdf_scale_system,
    animate_sdf_stretch_system, animate_size_system, animate_text_opacity_system,
    animate_text_progress_system, animate_text_spacing_system, animate_transform_system,
    animate_unified_effect_system, apply_mask_clipping_system, apply_parenthelper_system,
    compensate_sdf_ancestor_scale_for_children_system, compensate_sdf_parent_scale_system,
    debug_layer_global_z_system, fix_rtl_line_alignment_system, manage_layer_lifecycle_system,
    update_echo_runtime_system, update_sdf_mask_system, update_unified_mask_system,
};
use crate::effects::EffectRenderPlugin;
use crate::gaussian_blur::{GaussianBlurHMaterial, GaussianBlurPlugin, GaussianBlurVMaterial};
use crate::group_fill::GroupFillMaterial;
use crate::loader::{AlightMotionLoader, AmProject};
use crate::masked_sprite::UnifiedEffectMaterial;
use crate::plugin::AlightMotionSystemSet;
use crate::plugin::project_loading::spawn_loaded_projects_system;
use crate::plugin::resources::AmProjectResolution;
use crate::plugin::startup::{load_system_fonts_for_fallback, setup_white_pixel_system};
use crate::sdf::hot_reload_shader_system;
use crate::sdf_material::SdfMaterial;
use std::sync::atomic::{AtomicU32, Ordering};

pub(super) fn build_plugin(app: &mut App) {
    app.add_plugins(Material2dPlugin::<SdfMaterial>::default())
        .add_plugins(Material2dPlugin::<UnifiedEffectMaterial>::default())
        .add_plugins(Material2dPlugin::<GaussianBlurHMaterial>::default())
        .add_plugins(Material2dPlugin::<GaussianBlurVMaterial>::default())
        .add_plugins(Material2dPlugin::<GroupFillMaterial>::default())
        .add_plugins(EffectRenderPlugin)
        .add_plugins(GaussianBlurPlugin)
        .init_asset::<AmProject>()
        .init_asset_loader::<AlightMotionLoader>()
        .init_resource::<AmPlayback>()
        .init_resource::<AmProjectResolution>()
        .init_resource::<crate::effects::LiftCompositeState>()
        .add_systems(Startup, setup_white_pixel_system)
        .add_systems(Startup, load_system_fonts_for_fallback)
        .add_systems(Update, trace_asset_counts_system);

    configure_update_sets(app);
    register_lifecycle_systems(app);
    register_animation_systems(app);
    register_post_update_systems(app);
}

fn configure_update_sets(app: &mut App) {
    app.configure_sets(
        Update,
        (
            AlightMotionSystemSet::Lifecycle,
            AlightMotionSystemSet::Animation,
        )
            .chain(),
    );
}

fn register_lifecycle_systems(app: &mut App) {
    use bevy::ecs::schedule::ApplyDeferred;

    app.add_systems(
        Update,
        (
            spawn_loaded_projects_system,
            advance_playback_system,
            manage_layer_lifecycle_system,
            ApplyDeferred,
            crate::effects::evaluate_render_strategy_system,
            ApplyDeferred,
            crate::effects::setup_embed_scene_rtt_system,
            ApplyDeferred,
            crate::effects::sync_rtt_capture_root_system,
            ApplyDeferred,
            crate::effects::fix_nested_embed_render_layers_system,
            crate::effects::propagate_render_layers_system,
            crate::effects::sync_new_sdf_child_render_layers_system,
            crate::effects::propagate_render_layers_to_children_system,
            crate::effects::refresh_group_fill_material_texture_system,
            ApplyDeferred,
            crate::effects::setup_lift_composite_system,
            crate::effects::propagate_lift_render_layers_system,
            crate::effects::update_lift_comp_material_system,
        )
            .chain()
            .in_set(AlightMotionSystemSet::Lifecycle),
    );
}

fn register_animation_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            update_echo_runtime_system,
            animate_transform_system,
            compensate_sdf_parent_scale_system,
            animate_am_camera_system,
            animate_size_system,
            animate_sdf_stretch_system,
            animate_sdf_scale_system,
            animate_opacity_system,
            animate_sdf_opacity_system,
            animate_text_opacity_system,
            fix_rtl_line_alignment_system,
            animate_counter_system,
            animate_text_spacing_system,
            animate_text_progress_system,
            animate_unified_effect_system,
            animate_path_repeat_system,
            animate_rtt_blur_system,
            apply_mask_clipping_system,
            hot_reload_shader_system,
        )
            .chain()
            .in_set(AlightMotionSystemSet::Animation),
    )
    .add_systems(
        Update,
        apply_parenthelper_system
            .in_set(AlightMotionSystemSet::Animation)
            .after(animate_transform_system)
            .before(compensate_sdf_parent_scale_system),
    )
    .add_systems(
        Update,
        animate_sdf_repeat_system
            .in_set(AlightMotionSystemSet::Animation)
            .after(animate_sdf_stretch_system)
            .before(animate_sdf_scale_system),
    )
    .add_systems(
        Update,
        compensate_sdf_ancestor_scale_for_children_system
            .in_set(AlightMotionSystemSet::Animation)
            .after(compensate_sdf_parent_scale_system),
    );
}

fn register_post_update_systems(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (update_sdf_mask_system, update_unified_mask_system)
            .chain()
            .in_set(AlightMotionSystemSet::Mask)
            .after(bevy::transform::TransformSystems::Propagate),
    )
    .add_systems(
        PostUpdate,
        crate::effects::sync_rtt_camera_position_system
            .after(bevy::transform::TransformSystems::Propagate),
    )
    .add_systems(
        PostUpdate,
        debug_layer_global_z_system.after(bevy::transform::TransformSystems::Propagate),
    );
}

fn trace_asset_counts_system(
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    unified_materials: Res<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
) {
    if std::env::var_os("AM_ASSET_COUNT_TRACE").is_none() {
        return;
    }

    static FRAME_COUNTER: AtomicU32 = AtomicU32::new(0);
    let frame = FRAME_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    if !frame.is_multiple_of(30) {
        return;
    }

    bevy::log::warn!(
        "[ASSET-COUNT] frame={} meshes={} images={} unified_materials={}",
        frame,
        meshes.len(),
        images.len(),
        unified_materials.len()
    );
}
