use std::collections::{HashMap, HashSet};

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use super::{EmbedSceneRtt, RenderStrategy, propagate_to_descendants};

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
    children_query: Query<&Children>,
    render_layers_query: Query<&RenderLayers>,
) {
    let trace_renderlayers = std::env::var_os("AM_RENDERLAYER_TRACE").is_some();

    let composite_layers: HashMap<Entity, u8> = composite_embed_query
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
            RenderLayers::layer(rtt_layer as usize)
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

        let mut to_visit = Vec::new();
        if let Ok(children) = children_query.get(content_entity) {
            to_visit.extend(children.iter());
        }
        while let Some(child) = to_visit.pop() {
            let child_needs_update = match render_layers_query.get(child) {
                Ok(current) => *current != target_layer,
                Err(_) => true,
            };
            if child_needs_update {
                commands.entity(child).insert(target_layer.clone());
                updates += 1;
            }
            if let Ok(grandchildren) = children_query.get(child) {
                to_visit.extend(grandchildren.iter());
            }
        }
    }

    if updates > 0 {
        bevy::log::trace!(
            "[RenderLayers] Made {} updates this frame (content + descendants)",
            updates
        );
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

        let target_layer = RenderLayers::layer(rtt.render_layer as usize);
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
