//! # validation.rs
//!
//! # AM 场景验证模块
//!
//! Validation module for AM scene files.
//! AM 场景文件的验证模块。
//!
//! This module provides functionality to validate AM project files and report:
//! - Supported effects that are in use
//! - Unsupported effects that will be skipped
//! - Unsupported layer types (Audio, Video, Camera)
//! - Unknown XML elements that couldn't be parsed

use crate::schema::{AmEffect, AmLayer, AmScene};
use std::collections::{HashMap, HashSet};

/// Supported effect IDs and their display names
/// 支持的效果 ID 及其显示名称
pub const SUPPORTED_EFFECTS: &[(&str, &str)] = &[
    ("com.alightcreative.effects.transform2", "Transform2 (位置偏移)"),
    ("com.alightcreative.effects.wipe2", "Wipe2 (擦除效果)"),
    (
        "com.alightcreative.effects.stretchsegment",
        "StretchSegment (拉伸分割)",
    ),
    (
        "com.alightcreative.effects.gaussianblur",
        "GaussianBlur (高斯模糊)",
    ),
    (
        "com.alightcreative.effects.palettemap",
        "PaletteMap (调色板映射)",
    ),
    ("com.alightcreative.replacecolor", "ReplaceColor (颜色替换)"),
    (
        "com.alightcreative.effects.scaleassist",
        "ScaleAssist (缩放辅助)",
    ),
];

/// Validation report containing information about supported and unsupported features
/// 验证报告，包含支持和不支持的特性信息
#[derive(Debug, Default, Clone)]
pub struct ValidationReport {
    /// Effects that are used and supported
    /// 使用中且被支持的效果
    pub supported_effects_used: Vec<EffectUsage>,

    /// Effects that are used but not supported
    /// 使用中但不被支持的效果
    pub unsupported_effects: Vec<UnsupportedEffect>,

    /// Layer types that are not supported
    /// 不支持的图层类型
    pub unsupported_layers: Vec<UnsupportedLayer>,

    /// Warnings about potential issues
    /// 潜在问题的警告
    pub warnings: Vec<String>,

    /// Statistics about the scene
    /// 场景统计信息
    pub stats: SceneStats,
}

/// Information about an effect being used
/// 使用中的效果信息
#[derive(Debug, Clone)]
pub struct EffectUsage {
    /// Effect ID (e.g., "com.alightcreative.effects.transform2")
    pub effect_id: String,
    /// Effect display name
    pub display_name: String,
    /// Number of layers using this effect
    pub usage_count: u32,
}

/// Information about an unsupported effect
/// 不支持的效果信息
#[derive(Debug, Clone)]
pub struct UnsupportedEffect {
    /// Effect ID
    pub effect_id: String,
    /// Effect label from the project
    pub effect_label: String,
    /// Layer label where this effect is used
    pub layer_label: String,
    /// Layer ID
    pub layer_id: u64,
}

/// Information about an unsupported layer type
/// 不支持的图层类型信息
#[derive(Debug, Clone)]
pub struct UnsupportedLayer {
    /// Layer label
    pub label: String,
    /// Layer type name
    pub layer_type: String,
    /// Layer ID
    pub id: u64,
}

/// Scene statistics
/// 场景统计信息
#[derive(Debug, Default, Clone)]
pub struct SceneStats {
    /// Total number of layers (including nested)
    pub total_layers: u32,
    /// Number of shape layers
    pub shape_count: u32,
    /// Number of text layers
    pub text_count: u32,
    /// Number of image layers
    pub image_count: u32,
    /// Number of null object layers
    pub null_count: u32,
    /// Number of embedded scene layers
    pub embed_count: u32,
    /// Number of audio layers (unsupported)
    pub audio_count: u32,
    /// Number of video layers (unsupported)
    pub video_count: u32,
    /// Number of camera layers (unsupported)
    pub camera_count: u32,
    /// Number of bookmark layers
    pub bookmark_count: u32,
}

impl ValidationReport {
    /// Validate an AM scene and generate a report
    /// 验证 AM 场景并生成报告
    pub fn validate(scene: &AmScene) -> Self {
        let mut report = Self::default();
        let supported_ids: HashSet<&str> = SUPPORTED_EFFECTS.iter().map(|(id, _)| *id).collect();
        let mut effect_usage_map: HashMap<String, u32> = HashMap::new();

        report.validate_layers_recursive(&scene.layers, &supported_ids, &mut effect_usage_map);

        // Build supported effects list with usage counts
        for (id, name) in SUPPORTED_EFFECTS {
            if let Some(&count) = effect_usage_map.get(*id) {
                report.supported_effects_used.push(EffectUsage {
                    effect_id: id.to_string(),
                    display_name: name.to_string(),
                    usage_count: count,
                });
            }
        }

        report
    }

    /// Recursively validate layers
    fn validate_layers_recursive(
        &mut self,
        layers: &[AmLayer],
        supported_ids: &HashSet<&str>,
        effect_usage_map: &mut HashMap<String, u32>,
    ) {
        for layer in layers {
            self.stats.total_layers += 1;

            match layer {
                AmLayer::Shape(shape) => {
                    self.stats.shape_count += 1;
                    self.validate_effects(
                        &shape.effects,
                        &shape.label,
                        shape.id,
                        supported_ids,
                        effect_usage_map,
                    );
                }
                AmLayer::Text(text) => {
                    self.stats.text_count += 1;
                    // Text layers can also have effects
                    self.validate_effects(
                        &text.effects,
                        &text.label,
                        text.id,
                        supported_ids,
                        effect_usage_map,
                    );
                }
                AmLayer::Image(image) => {
                    self.stats.image_count += 1;
                    self.validate_effects(
                        &image.effects,
                        &image.label,
                        image.id,
                        supported_ids,
                        effect_usage_map,
                    );
                }
                AmLayer::Nullobj(null) => {
                    self.stats.null_count += 1;
                    self.validate_effects(
                        &null.effects,
                        &null.label,
                        null.id,
                        supported_ids,
                        effect_usage_map,
                    );
                }
                AmLayer::EmbedScene(embed) => {
                    self.stats.embed_count += 1;
                    // EmbedScene doesn't have effects field, but has nested scene
                    // Recursively validate nested scene
                    self.validate_layers_recursive(
                        &embed.scene.layers,
                        supported_ids,
                        effect_usage_map,
                    );
                }
                AmLayer::Audio(audio) => {
                    self.stats.audio_count += 1;
                    self.unsupported_layers.push(UnsupportedLayer {
                        label: audio.label.clone(),
                        layer_type: "Audio (音频)".to_string(),
                        id: audio.id,
                    });
                }
                AmLayer::Video(video) => {
                    self.stats.video_count += 1;
                    // Video also has effects
                    self.validate_effects(
                        &video.effects,
                        &video.label,
                        video.id,
                        supported_ids,
                        effect_usage_map,
                    );
                    self.unsupported_layers.push(UnsupportedLayer {
                        label: video.label.clone(),
                        layer_type: "Video (视频)".to_string(),
                        id: video.id,
                    });
                }
                AmLayer::Camera(camera) => {
                    self.stats.camera_count += 1;
                    self.unsupported_layers.push(UnsupportedLayer {
                        label: camera.label.clone(),
                        layer_type: "Camera (相机)".to_string(),
                        id: camera.id,
                    });
                }
                AmLayer::Bookmark(_) => {
                    self.stats.bookmark_count += 1;
                    // Bookmarks are non-visual markers, silently skip
                }
            }
        }
    }

    /// Validate effects on a layer
    fn validate_effects(
        &mut self,
        effects: &[AmEffect],
        layer_label: &str,
        layer_id: u64,
        supported_ids: &HashSet<&str>,
        effect_usage_map: &mut HashMap<String, u32>,
    ) {
        for effect in effects {
            if supported_ids.contains(effect.id.as_str()) {
                *effect_usage_map.entry(effect.id.clone()).or_insert(0) += 1;
            } else {
                // Effect doesn't have a label field, use a short form of id as label
                let effect_label = effect
                    .id
                    .rsplit('.')
                    .next()
                    .unwrap_or(&effect.id)
                    .to_string();
                self.unsupported_effects.push(UnsupportedEffect {
                    effect_id: effect.id.clone(),
                    effect_label,
                    layer_label: layer_label.to_string(),
                    layer_id,
                });
            }
        }
    }

    /// Log the validation report
    /// 输出验证报告到日志
    pub fn log_report(&self, project_title: &str) {
        bevy::log::info!("========================================");
        bevy::log::info!("[AM Validation] Project: {}", project_title);
        bevy::log::info!("========================================");

        // Scene statistics
        bevy::log::info!(
            "[AM Validation] Scene stats: {} layers total",
            self.stats.total_layers
        );
        bevy::log::info!(
            "  - Shape: {}, Text: {}, Image: {}, Null: {}, Embed: {}",
            self.stats.shape_count,
            self.stats.text_count,
            self.stats.image_count,
            self.stats.null_count,
            self.stats.embed_count
        );

        // Supported effects in use
        if self.supported_effects_used.is_empty() {
            bevy::log::info!("[AM Validation] Effects: None used");
        } else {
            bevy::log::info!(
                "[AM Validation] Supported effects in use ({}):",
                self.supported_effects_used.len()
            );
            for effect in &self.supported_effects_used {
                bevy::log::info!(
                    "  ✓ {} - {} usage(s)",
                    effect.display_name,
                    effect.usage_count
                );
            }
        }

        // Unsupported effects
        if !self.unsupported_effects.is_empty() {
            // Deduplicate by effect_id for cleaner output
            let mut effect_counts: HashMap<&str, Vec<&UnsupportedEffect>> = HashMap::new();
            for effect in &self.unsupported_effects {
                effect_counts
                    .entry(&effect.effect_id)
                    .or_default()
                    .push(effect);
            }

            bevy::log::warn!(
                "[AM Validation] Unsupported effects ({} unique types):",
                effect_counts.len()
            );
            for (effect_id, usages) in &effect_counts {
                let first = usages.first().unwrap();
                if usages.len() == 1 {
                    bevy::log::warn!(
                        "  ✗ '{}' ({}) on layer '{}' (id={})",
                        first.effect_label,
                        effect_id,
                        first.layer_label,
                        first.layer_id
                    );
                } else {
                    bevy::log::warn!(
                        "  ✗ '{}' ({}) - {} usage(s)",
                        first.effect_label,
                        effect_id,
                        usages.len()
                    );
                }
            }
        }

        // Unsupported layers
        if !self.unsupported_layers.is_empty() {
            bevy::log::warn!(
                "[AM Validation] Unsupported layer types ({}):",
                self.unsupported_layers.len()
            );
            for layer in &self.unsupported_layers {
                bevy::log::warn!(
                    "  ✗ {} '{}' (id={}) - will be skipped",
                    layer.layer_type,
                    layer.label,
                    layer.id
                );
            }
        }

        // Summary
        if self.unsupported_effects.is_empty() && self.unsupported_layers.is_empty() {
            bevy::log::info!("[AM Validation] ✓ All features in this project are supported");
        } else {
            let total_issues = self.unsupported_effects.len() + self.unsupported_layers.len();
            bevy::log::warn!(
                "[AM Validation] ⚠ {} unsupported feature(s) will be skipped",
                total_issues
            );
        }

        bevy::log::info!("========================================");
    }
}
