//! Bridges loaded `AmProject` assets into live scene entities.
//! Once the asset loader has parsed a project, the systems here apply resolution
//! scaling, build the layer containers needed for lazy spawning, and attach the
//! runtime bookkeeping that playback uses afterwards.
//!
//! 负责把已经加载好的 `AmProject` 资产接到真实场景实体上。资产加载器
//! 解析完项目后，这里的系统会应用分辨率缩放、创建懒生成所需的图层容器，并挂上
//! 后续播放阶段要使用的运行时状态。

use bevy::prelude::*;

use crate::loader::AmProject;
use crate::plugin::resources::AmProjectResolution;
use crate::scene::{AmProjectBundle, AmProjectRoot, AmSceneConfig};

fn trace_replace_pending_layers(layers: &[crate::scene::PendingLayer]) {
    fn visit(layers: &[crate::scene::PendingLayer]) {
        for layer in layers {
            if layer.animated.replace_old_color != Vec4::ZERO
                || layer.animated.replace_new_color.value.is_some()
                || !layer.animated.replace_new_color.keyframes.is_empty()
            {
                bevy::log::warn!(
                    "[PendingReplaceTrace] id={} label='{}' old={:?} new_static={:?}",
                    layer.id,
                    layer.label,
                    layer.animated.replace_old_color,
                    layer.animated.replace_new_color.value,
                );
            }
            visit(&layer.children);
        }
    }

    visit(layers);
}

fn trace_pending_subtree(layers: &[crate::scene::PendingLayer], root_id: u64) {
    fn print_layer(layer: &crate::scene::PendingLayer, depth: usize) {
        let indent = "  ".repeat(depth);
        bevy::log::warn!(
            "[PendingSubtree] {}id={} label='{}' parent={} range={}..{} children={}",
            indent,
            layer.id,
            layer.label,
            layer.parent,
            layer.start_time,
            layer.end_time,
            layer.children.len(),
        );
        for child in &layer.children {
            print_layer(child, depth + 1);
        }
    }

    fn visit(layers: &[crate::scene::PendingLayer], root_id: u64) -> bool {
        for layer in layers {
            if layer.id == root_id {
                print_layer(layer, 0);
                return true;
            }
            if visit(&layer.children, root_id) {
                return true;
            }
        }
        false
    }

    let _ = visit(layers, root_id);
}

pub(super) fn spawn_loaded_projects_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AmProjectRoot, &mut Transform)>,
    projects: Res<Assets<AmProject>>,
    mut playback: ResMut<crate::animation::AmPlayback>,
    resolution_config: Res<AmProjectResolution>,
    window_query: Query<&Window>,
) {
    for (entity, mut root, mut transform) in query.iter_mut() {
        if root.spawned {
            continue;
        }

        let Some(project) = projects.get(&root.handle) else {
            continue;
        };
        bevy::log::info!(
            "Loading AM project: {} ({}x{}, {}ms)",
            project.scene.title,
            project.scene.width,
            project.scene.height,
            project.scene.total_time
        );
        bevy::log::debug!("  Media count: {}", project.scene.media.len());
        bevy::log::debug!("  Images loaded: {}", project.images.len());
        for uri in project.images.keys() {
            bevy::log::trace!("    - {}", uri);
        }
        bevy::log::debug!("  Layers count: {}", project.scene.layers.len());

        playback.total_time_ms = project.scene.total_time as f32;

        let fit_scale = apply_resolution_scale(
            &mut transform,
            &project.scene,
            *resolution_config,
            window_query.iter().next(),
        );

        let config = AmSceneConfig {
            canvas_width: project.scene.width as f32,
            canvas_height: project.scene.height as f32,
            scene_fps: project.scene.fps as f32,
            scene_total_time: project.scene.total_time as f32,
            render_fps: project.scene.fps as f32,
            comparison_frame_center_bias_ms: 500.0 / project.scene.fps.max(1) as f32,
            ..Default::default()
        };

        let pending_layers = crate::scene::collect_pending_layers(
            &project.scene,
            &project.fonts,
            &project.font_metrics,
            &config,
        );

        if std::env::var_os("AM_TRACE_PENDING_REPLACE").is_some() {
            trace_replace_pending_layers(&pending_layers);
        }
        if let Some(root_id) = std::env::var("AM_TRACE_PENDING_SUBTREE_ID")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            trace_pending_subtree(&pending_layers, root_id);
        }

        bevy::log::info!(
            "Prepared {} pending layers for lazy spawning",
            pending_layers.len()
        );

        let layers_container = spawn_named_container(
            &mut commands,
            "AmLayersContainer",
            crate::scene::AmLayersContainer,
        );
        commands.entity(entity).add_child(layers_container);

        let embed_contents_container = spawn_named_container(
            &mut commands,
            "AmEmbedContentsContainer",
            crate::scene::AmEmbedContentsContainer,
        );
        let rtt_cameras_container = spawn_named_container(
            &mut commands,
            "AmRttCamerasContainer",
            crate::scene::AmRttCamerasContainer,
        );

        commands
            .entity(entity)
            .insert(crate::scene::AmPendingLayers {
                layers: pending_layers,
                spawned_entities: std::collections::HashMap::new(),
                hibernated_entities: std::collections::HashMap::new(),
                inv_fit_scale: 1.0 / fit_scale,
                layers_container: Some(layers_container),
                embed_contents_container: Some(embed_contents_container),
                rtt_cameras_container: Some(rtt_cameras_container),
            });

        root.spawned = true;
        bevy::log::info!("Project ready for playback");
    }
}

pub fn load_am_project(
    commands: &mut Commands,
    asset_server: &AssetServer,
    path: impl Into<String>,
) -> Entity {
    let path_string: String = path.into();
    let handle: Handle<AmProject> = asset_server.load(path_string.clone());

    let project_name = path_string
        .rsplit('/')
        .next()
        .unwrap_or(&path_string)
        .trim_end_matches(".amproj")
        .trim_end_matches(".xml");

    commands
        .spawn((
            Name::new(format!("AmProject: {}", project_name)),
            AmProjectBundle {
                transform: Transform::default(),
                global_transform: GlobalTransform::default(),
                visibility: Visibility::default(),
                inherited_visibility: InheritedVisibility::default(),
                view_visibility: ViewVisibility::default(),
                marker: AmProjectRoot {
                    handle,
                    spawned: false,
                },
            },
        ))
        .id()
}

fn apply_resolution_scale(
    transform: &mut Transform,
    scene: &crate::schema::AmScene,
    resolution_config: AmProjectResolution,
    window: Option<&Window>,
) -> f32 {
    let mut fit_scale = 1.0f32;

    match resolution_config {
        AmProjectResolution::None => {}
        AmProjectResolution::FitWindow => {
            if let Some(window) = window {
                let s_x = window.width() / (scene.width as f32);
                let s_y = window.height() / (scene.height as f32);
                fit_scale = s_x.min(s_y);
                transform.scale = Vec3::splat(fit_scale);
                bevy::log::info!(
                    "Scaled project to fit window: scale={:.4} (win={}x{})",
                    fit_scale,
                    window.width(),
                    window.height()
                );
            }
        }
        AmProjectResolution::CoverWindow => {
            if let Some(window) = window {
                let s_x = window.width() / (scene.width as f32);
                let s_y = window.height() / (scene.height as f32);
                fit_scale = s_x.max(s_y);
                transform.scale = Vec3::splat(fit_scale);
                bevy::log::info!(
                    "Scaled project to cover window: scale={:.4} (win={}x{})",
                    fit_scale,
                    window.width(),
                    window.height()
                );
            }
        }
        AmProjectResolution::FixedWidth(target_width) => {
            fit_scale = target_width / (scene.width as f32);
            transform.scale = Vec3::splat(fit_scale);
            bevy::log::info!(
                "Scaled project to fixed width {}: scale={:.4}",
                target_width,
                fit_scale
            );
        }
        AmProjectResolution::FixedHeight(target_height) => {
            fit_scale = target_height / (scene.height as f32);
            transform.scale = Vec3::splat(fit_scale);
            bevy::log::info!(
                "Scaled project to fixed height {}: scale={:.4}",
                target_height,
                fit_scale
            );
        }
        AmProjectResolution::FixedSize(target_width, target_height) => {
            let s_x = target_width / (scene.width as f32);
            let s_y = target_height / (scene.height as f32);
            fit_scale = s_x.min(s_y);
            transform.scale = Vec3::splat(fit_scale);
            bevy::log::info!(
                "Scaled project to fixed size {}x{}: scale={:.4}",
                target_width,
                target_height,
                fit_scale
            );
        }
    }

    fit_scale
}

fn spawn_named_container<C: Component>(commands: &mut Commands, name: &str, marker: C) -> Entity {
    commands
        .spawn((
            Name::new(name.to_string()),
            marker,
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id()
}
