//! Propagates render-layer assignments for embed-scene rendering.
//! It keeps direct and composite embed content on the correct `RenderLayers`,
//! and cascades those choices to descendants so cameras and child visuals render
//! into the intended pass.
//!
//! 负责传播嵌套场景渲染所需的 render layer 分配。它会让 direct 和 composite
//! 两种路径下的 embed 内容都处在正确的 `RenderLayers` 上，并把这个选择级联到子节点，
//! 确保相机和视觉对象进入预期的渲染通道。

use std::collections::{HashMap, HashSet};

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use super::{EmbedSceneRtt, RenderStrategy, propagate_to_descendants};

fn dynamic_render_layer(layer: usize) -> RenderLayers {
    RenderLayers::from_layers(&[layer])
}

pub fn propagate_render_layers_system(
    mut commands: Commands,
    composite_embed_query: Query<(Entity, &EmbedSceneRtt)>,
    direct_embed_query: Query<(Entity, &RenderStrategy), Without<EmbedSceneRtt>>,
    content_query: Query<(
        Entity,
        &crate::scene::AmEmbedContentMarker,
        Option<&RenderLayers>,
        Option<&Visibility>,
    )>,
) {
    let trace_renderlayers = std::env::var_os("AM_RENDERLAYER_TRACE").is_some();

    let composite_layers: HashMap<Entity, usize> = composite_embed_query
        .iter()
        .map(|(entity, rtt)| (entity, rtt.render_layer))
        .collect();

    let direct_embeds: HashSet<Entity> = direct_embed_query
        .iter()
        .filter(|(_, strategy)| **strategy == RenderStrategy::Direct)
        .map(|(entity, _)| entity)
        .collect();

    let mut updates = 0;

    for (content_entity, marker, current_layers, current_visibility) in content_query.iter() {
        let target_layer = if let Some(&rtt_layer) = composite_layers.get(&marker.embed_entity) {
            dynamic_render_layer(rtt_layer)
        } else if direct_embeds.contains(&marker.embed_entity) {
            RenderLayers::layer(0)
        } else {
            continue;
        };

        if trace_renderlayers {
            bevy::log::warn!(
                "[RenderLayers:Content] content={:?} embed={:?} target={:?} current={:?}",
                content_entity,
                marker.embed_entity,
                target_layer,
                current_layers,
            );
        }

        let needs_update = match current_layers {
            Some(current) => *current != target_layer,
            None => true,
        };

        if needs_update {
            let target_visibility = match current_visibility {
                Some(Visibility::Hidden) => Visibility::Hidden,
                _ => Visibility::Inherited,
            };

            commands
                .entity(content_entity)
                .insert((target_layer.clone(), target_visibility));
            updates += 1;
        }
    }

    if updates > 0 {
        bevy::log::trace!(
            "[RenderLayers] Made {} direct content updates this frame",
            updates
        );
    }
}

pub fn sync_new_sdf_child_render_layers_system(
    mut commands: Commands,
    new_sdf_query: Query<
        (
            Entity,
            &ChildOf,
            Option<&RenderLayers>,
            Option<&Visibility>,
            Option<&crate::scene::AmEmbedContentMarker>,
        ),
        Added<MeshMaterial2d<crate::sdf_material::SdfMaterial>>,
    >,
    parent_query: Query<(
        Option<&RenderLayers>,
        Option<&Visibility>,
        Option<&crate::scene::AmEmbedContentMarker>,
    )>,
    force_hidden_query: Query<(), With<crate::scene::AmForceHidden>>,
) {
    let trace_renderlayers = std::env::var_os("AM_RENDERLAYER_TRACE").is_some();

    for (entity, child_of, current_layers, current_visibility, current_marker) in
        new_sdf_query.iter()
    {
        let parent = child_of.parent();
        let Ok((parent_layers, parent_visibility, parent_marker)) = parent_query.get(parent) else {
            continue;
        };

        let mut entity_commands = commands.entity(entity);
        let mut updated = false;

        if let Some(parent_layers) = parent_layers
            && current_layers != Some(parent_layers)
        {
            entity_commands.insert(parent_layers.clone());
            updated = true;
        }

        let should_force_hidden = parent_visibility
            .is_some_and(|visibility| *visibility == Visibility::Hidden)
            || force_hidden_query.get(entity).is_ok();
        if should_force_hidden {
            if current_visibility != Some(&Visibility::Hidden) {
                entity_commands.insert(Visibility::Hidden);
                updated = true;
            }
        } else if current_visibility == Some(&Visibility::Hidden) {
            entity_commands.insert(Visibility::Inherited);
            updated = true;
        }

        if let Some(parent_marker) = parent_marker
            && current_marker.is_none()
        {
            entity_commands.insert(parent_marker.clone());
            updated = true;
        }

        if trace_renderlayers && updated {
            bevy::log::warn!(
                "[RenderLayers:SdfChildSync] child={:?} parent={:?} layers={:?} marker_embed={:?}",
                entity,
                parent,
                parent_layers,
                parent_marker.map(|marker| marker.embed_entity),
            );
        }
    }
}

pub fn propagate_render_layers_to_children_system(
    mut commands: Commands,
    composite_embed_query: Query<(Entity, &EmbedSceneRtt)>,
    direct_embed_query: Query<(Entity, &RenderStrategy), Without<EmbedSceneRtt>>,
    children_query: Query<&Children>,
    render_layers_query: Query<&RenderLayers>,
    visibility_query: Query<&Visibility>,
    force_hidden_query: Query<(), With<crate::scene::AmForceHidden>>,
    non_embed_query: Query<Entity, (Without<EmbedSceneRtt>, Without<RenderStrategy>)>,
) {
    let trace_renderlayers = std::env::var_os("AM_RENDERLAYER_TRACE").is_some();
    let mut total_updates = 0;

    for (embed_entity, rtt) in composite_embed_query.iter() {
        let Ok(children) = children_query.get(embed_entity) else {
            continue;
        };

        let target_layer = dynamic_render_layer(rtt.render_layer);
        total_updates += propagate_to_descendants(
            &mut commands,
            embed_entity,
            children,
            &target_layer,
            &children_query,
            &render_layers_query,
            &visibility_query,
            &force_hidden_query,
            &non_embed_query,
        );
    }

    let layer_0 = RenderLayers::layer(0);
    let mut direct_with_children = 0;
    let mut direct_total_children = 0;
    for (embed_entity, strategy) in direct_embed_query.iter() {
        if *strategy != RenderStrategy::Direct {
            continue;
        }

        let Ok(children) = children_query.get(embed_entity) else {
            continue;
        };

        direct_with_children += 1;
        direct_total_children += children.len();

        if trace_renderlayers {
            bevy::log::warn!(
                "[RenderLayers:DirectEmbed] embed={:?} children={} target={:?}",
                embed_entity,
                children.len(),
                layer_0,
            );
        }

        total_updates += propagate_to_descendants(
            &mut commands,
            embed_entity,
            children,
            &layer_0,
            &children_query,
            &render_layers_query,
            &visibility_query,
            &force_hidden_query,
            &non_embed_query,
        );
    }

    if total_updates > 0 {
        bevy::log::trace!(
            "[PropagateChildren] {} updates, Direct embeds with children: {} (total {} children)",
            total_updates,
            direct_with_children,
            direct_total_children
        );
    }
}
