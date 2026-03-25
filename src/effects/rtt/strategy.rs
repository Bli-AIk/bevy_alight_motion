//! Decides how each embed scene should be rendered.
//! It inspects scale animation, fill requirements, masking, and other constraints
//! to choose between direct, stencil, and composite rendering, then seeds the
//! follow-up components that the chosen path needs.
//!
//! 负责决定每个嵌套场景应当采用哪种渲染策略。它会检查缩放动画、fill 需求、
//! 遮罩以及其他约束，在 direct、stencil 和 composite 之间做选择，然后补上该路径
//! 后续系统所需要的组件。

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use super::{
    AmEmbedMask, AmGroupFill, EmbedSceneBounds, GroupFillType, NeedsEmbedSceneRtt,
    NeedsStrategyEvaluation, RenderHierarchyInfo, RenderStrategy,
};

pub fn evaluate_render_strategy_system(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &NeedsStrategyEvaluation,
            Option<&AmGroupFill>,
            Option<&AmEmbedMask>,
            Option<&crate::scene::AmForceHidden>,
        ),
        Without<RenderStrategy>,
    >,
) {
    for (entity, needs_eval, group_fill, embed_mask, force_hidden) in query.iter() {
        let needs_fill = group_fill.is_some();
        let is_mask = embed_mask.is_some();

        let strategy = if needs_eval.requires_composite || needs_fill || is_mask {
            RenderStrategy::Composite
        } else if needs_eval.has_scale_animation {
            RenderStrategy::Stencil
        } else {
            RenderStrategy::Direct
        };

        bevy::log::warn!(
            "[Strategy-DBG] Embed {:?} → {:?} (fill={}, mask={}, force_composite={})",
            entity,
            strategy,
            needs_fill,
            is_mask,
            needs_eval.requires_composite,
        );

        commands
            .entity(entity)
            .remove::<NeedsStrategyEvaluation>()
            .insert((
                strategy,
                RenderHierarchyInfo::default(),
                RenderLayers::layer(0),
                if force_hidden.is_some() {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                },
                EmbedSceneBounds {
                    width: needs_eval.scene_width,
                    height: needs_eval.scene_height,
                },
            ));

        if strategy == RenderStrategy::Composite {
            commands.entity(entity).insert(NeedsEmbedSceneRtt {
                scene_width: needs_eval.scene_width,
                scene_height: needs_eval.scene_height,
                dynamic_resolution: needs_eval.dynamic_resolution,
            });
        }

        if let Some(fill) = group_fill
            && fill.fill_type == GroupFillType::None
        {
            commands.entity(entity).insert(Visibility::Hidden);
        }
    }
}
