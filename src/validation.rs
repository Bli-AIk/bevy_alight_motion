//! # validation.rs
//!
//! # AM 场景验证模块
//!
//! Validation module for AM scene files.
//! AM 场景文件的验证模块。
//!
//! This module provides functionality to validate AM project files and report:
//! - Supported effects that are in use
//! - Partially supported effects (works but with some limitations)
//! - Unsupported effects that will be skipped
//! - Unsupported layer types (Audio, Video, Camera)
//!
//! 效果定义现在来自 effects_registry 模块（单一数据源）。
//! Effect definitions now come from effects_registry module (single source of truth).

use crate::effects_registry::{self, types::SupportLevel};
use crate::schema::{AmEffect, AmLayer, AmScene};
use serde::Serialize;
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use owo_colors::OwoColorize;

/// Effect support level (re-exported for backward compatibility)
/// 效果支持级别（为向后兼容而重新导出）
pub use crate::effects_registry::types::SupportLevel as EffectSupportLevel;

/// Validation report containing information about supported and unsupported features
/// 验证报告，包含支持和不支持的特性信息
#[derive(Debug, Default, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
pub struct EffectUsage {
    /// Effect ID (e.g., "com.alightcreative.effects.transform2")
    pub effect_id: String,
    /// Effect display name
    pub display_name: String,
    /// Number of layers using this effect
    pub usage_count: u32,
    /// Support level
    pub level: SupportLevel,
}

/// Information about an unsupported effect
/// 不支持的效果信息
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Default, Clone, Serialize)]
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

/// Print a single supported effect entry with colored output
#[cfg(not(target_arch = "wasm32"))]
fn log_effect_usage_entry(effect: &EffectUsage) {
    match effect.level {
        SupportLevel::Full => {
            println!(
                "  {} {} - {} usage(s)",
                "✓".green(),
                effect.display_name.green(),
                effect.usage_count
            );
        }
        SupportLevel::Partial => {
            println!(
                "  {} {} - {} usage(s) {}",
                "⚠".yellow(),
                effect.display_name.yellow(),
                effect.usage_count,
                "(partial support)".dimmed()
            );
        }
        SupportLevel::Unsupported => {
            // This shouldn't happen in supported_effects_used
        }
    }
}

/// Print a single unsupported effect entry with colored output
#[cfg(not(target_arch = "wasm32"))]
fn log_unsupported_effect_entry(effect_id: &str, usages: &[&UnsupportedEffect]) {
    let first = usages.first().unwrap();
    if usages.len() == 1 {
        println!(
            "  {} '{}' ({}) on layer '{}' (id={})",
            "✗".red(),
            first.effect_label.red(),
            effect_id.dimmed(),
            first.layer_label,
            first.layer_id
        );
    } else {
        println!(
            "  {} '{}' ({}) - {} usage(s)",
            "✗".red(),
            first.effect_label.red(),
            effect_id.dimmed(),
            usages.len()
        );
    }
}

impl ValidationReport {
    /// Validate an AM scene and generate a report
    /// 验证 AM 场景并生成报告
    pub fn validate(scene: &AmScene) -> Self {
        let mut report = Self::default();
        // Create a map of effect ID -> EffectDef for quick lookup using effects_registry
        // 使用 effects_registry 创建效果 ID -> EffectDef 的映射
        let all_effects = effects_registry::all_effects();
        let effect_defs: HashMap<&str, &effects_registry::EffectDef> =
            all_effects.iter().map(|e| (e.id, *e)).collect();
        let mut effect_usage_map: HashMap<String, u32> = HashMap::new();

        report.validate_layers_recursive(&scene.layers, &effect_defs, &mut effect_usage_map);

        // Build supported effects list with usage counts
        for effect_def in all_effects {
            if let Some(&count) = effect_usage_map.get(effect_def.id) {
                report.supported_effects_used.push(EffectUsage {
                    effect_id: effect_def.id.to_string(),
                    display_name: effect_def.display_name_zh.to_string(),
                    usage_count: count,
                    level: effect_def.support_level,
                });
            }
        }

        report
    }

    /// Recursively validate layers
    fn validate_layers_recursive(
        &mut self,
        layers: &[AmLayer],
        effect_defs: &HashMap<&str, &effects_registry::EffectDef>,
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
                        effect_defs,
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
                        effect_defs,
                        effect_usage_map,
                    );
                }
                AmLayer::Image(image) => {
                    self.stats.image_count += 1;
                    self.validate_effects(
                        &image.effects,
                        &image.label,
                        image.id,
                        effect_defs,
                        effect_usage_map,
                    );
                }
                AmLayer::Nullobj(null) => {
                    self.stats.null_count += 1;
                    self.validate_effects(
                        &null.effects,
                        &null.label,
                        null.id,
                        effect_defs,
                        effect_usage_map,
                    );
                }
                AmLayer::EmbedScene(embed) => {
                    self.stats.embed_count += 1;
                    // EmbedScene doesn't have effects field, but has nested scene
                    // Recursively validate nested scene
                    self.validate_layers_recursive(
                        &embed.scene.layers,
                        effect_defs,
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
                        effect_defs,
                        effect_usage_map,
                    );
                    self.unsupported_layers.push(UnsupportedLayer {
                        label: video.label.clone(),
                        layer_type: "Video (视频)".to_string(),
                        id: video.id,
                    });
                }
                AmLayer::Camera(_camera) => {
                    self.stats.camera_count += 1;
                    // Camera layers are supported via the collect pipeline
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
        effect_defs: &HashMap<&str, &effects_registry::EffectDef>,
        effect_usage_map: &mut HashMap<String, u32>,
    ) {
        for effect in effects {
            if effect_defs.contains_key(effect.id.as_str()) {
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

    /// Log the validation report with colored output (native only)
    /// 使用彩色输出日志验证报告 (仅限原生环境)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn log_report(&self, project_title: &str) {
        println!();
        println!("{}", "========================================".cyan());
        println!(
            "{} {}",
            "[AM Validation]".cyan().bold(),
            format!("Project: {}", project_title).white()
        );
        println!("{}", "========================================".cyan());

        // Scene statistics
        println!(
            "{} {} layers total",
            "[AM Validation]".cyan(),
            self.stats.total_layers.to_string().white()
        );
        println!(
            "  {} Shape: {}, Text: {}, Image: {}, Null: {}, Embed: {}",
            "·".dimmed(),
            self.stats.shape_count,
            self.stats.text_count,
            self.stats.image_count,
            self.stats.null_count,
            self.stats.embed_count
        );

        // Supported effects in use
        if self.supported_effects_used.is_empty() {
            println!(
                "{} Effects: {}",
                "[AM Validation]".cyan(),
                "None used".dimmed()
            );
        } else {
            let full_count = self
                .supported_effects_used
                .iter()
                .filter(|e| e.level == SupportLevel::Full)
                .count();
            let partial_count = self
                .supported_effects_used
                .iter()
                .filter(|e| e.level == SupportLevel::Partial)
                .count();

            println!(
                "{} Effects in use: {} full, {} partial",
                "[AM Validation]".cyan(),
                full_count.to_string().green(),
                partial_count.to_string().yellow()
            );

            for effect in &self.supported_effects_used {
                log_effect_usage_entry(effect);
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

            println!(
                "{} {} ({} unique types):",
                "[AM Validation]".cyan(),
                "Unsupported effects".red().bold(),
                effect_counts.len()
            );
            for (effect_id, usages) in &effect_counts {
                log_unsupported_effect_entry(effect_id, usages);
            }
        }

        // Unsupported layers
        if !self.unsupported_layers.is_empty() {
            println!(
                "{} {} ({}):",
                "[AM Validation]".cyan(),
                "Unsupported layer types".red().bold(),
                self.unsupported_layers.len()
            );
            for layer in &self.unsupported_layers {
                println!(
                    "  {} {} '{}' (id={}) - will be skipped",
                    "✗".red(),
                    layer.layer_type.red(),
                    layer.label,
                    layer.id
                );
            }
        }

        // Summary
        println!("{}", "----------------------------------------".dimmed());
        if self.unsupported_effects.is_empty() && self.unsupported_layers.is_empty() {
            let partial_count = self
                .supported_effects_used
                .iter()
                .filter(|e| e.level == SupportLevel::Partial)
                .count();
            if partial_count > 0 {
                println!(
                    "{} {} (with {} partial effects)",
                    "[AM Validation]".cyan(),
                    "All features supported".green().bold(),
                    partial_count.to_string().yellow()
                );
            } else {
                println!(
                    "{} {}",
                    "[AM Validation]".cyan(),
                    "✓ All features in this project are fully supported"
                        .green()
                        .bold()
                );
            }
        } else {
            let total_issues = self.unsupported_effects.len() + self.unsupported_layers.len();
            println!(
                "{} {} unsupported feature(s) will be skipped",
                "[AM Validation]".cyan(),
                format!("⚠ {}", total_issues).red().bold()
            );
        }
        println!("{}", "========================================".cyan());
        println!();
    }

    /// Log the validation report for WASM (outputs JSON for JavaScript parsing)
    /// WASM 版本的验证报告输出 (输出 JSON 供 JavaScript 解析)
    #[cfg(target_arch = "wasm32")]
    pub fn log_report_wasm(&self, project_title: &str) {
        use web_sys::console;

        // Output JSON for structured parsing by JavaScript
        #[derive(Serialize)]
        struct WasmReport<'a> {
            project_title: &'a str,
            stats: &'a SceneStats,
            supported_effects: &'a Vec<EffectUsage>,
            unsupported_effects: &'a Vec<UnsupportedEffect>,
            unsupported_layers: &'a Vec<UnsupportedLayer>,
        }

        let report = WasmReport {
            project_title,
            stats: &self.stats,
            supported_effects: &self.supported_effects_used,
            unsupported_effects: &self.unsupported_effects,
            unsupported_layers: &self.unsupported_layers,
        };

        if let Ok(json) = serde_json::to_string(&report) {
            console::log_1(&format!("[AM_VALIDATION_JSON]{}", json).into());
        }

        // Also output human-readable version
        console::log_1(&"========================================".into());
        console::log_1(&format!("[AM Validation] Project: {}", project_title).into());
        console::log_1(&"========================================".into());
        console::log_1(&format!("[AM Validation] {} layers total", self.stats.total_layers).into());
        console::log_1(
            &format!(
                "  · Shape: {}, Text: {}, Image: {}, Null: {}, Embed: {}",
                self.stats.shape_count,
                self.stats.text_count,
                self.stats.image_count,
                self.stats.null_count,
                self.stats.embed_count
            )
            .into(),
        );

        let full_count = self
            .supported_effects_used
            .iter()
            .filter(|e| e.level == SupportLevel::Full)
            .count();
        let partial_count = self
            .supported_effects_used
            .iter()
            .filter(|e| e.level == SupportLevel::Partial)
            .count();

        if self.supported_effects_used.is_empty() {
            console::log_1(&"[AM Validation] Effects: None used".into());
        } else {
            console::log_1(
                &format!(
                    "[AM Validation] Effects in use: {} full, {} partial",
                    full_count, partial_count
                )
                .into(),
            );
            for effect in &self.supported_effects_used {
                let icon = if effect.level == SupportLevel::Full {
                    "✓"
                } else {
                    "⚠"
                };
                let suffix = if effect.level == SupportLevel::Partial {
                    " (partial support)"
                } else {
                    ""
                };
                console::log_1(
                    &format!(
                        "  {} {} - {} usage(s){}",
                        icon, effect.display_name, effect.usage_count, suffix
                    )
                    .into(),
                );
            }
        }

        if !self.unsupported_effects.is_empty() {
            // First, collect deduplicated effect counts
            let mut effect_counts: HashMap<&str, (&UnsupportedEffect, usize)> = HashMap::new();
            for effect in &self.unsupported_effects {
                effect_counts
                    .entry(&effect.effect_id)
                    .and_modify(|(_, count)| *count += 1)
                    .or_insert((effect, 1));
            }

            console::warn_1(
                &format!(
                    "[AM Validation] Unsupported effects ({} unique types):",
                    effect_counts.len()
                )
                .into(),
            );

            // Now output each unique effect
            for (effect_id, (effect, count)) in &effect_counts {
                if *count == 1 {
                    console::warn_1(
                        &format!(
                            "  ✗ '{}' ({}) on layer '{}' (id={})",
                            effect.effect_label, effect_id, effect.layer_label, effect.layer_id
                        )
                        .into(),
                    );
                } else {
                    console::warn_1(
                        &format!(
                            "  ✗ '{}' ({}) - {} usage(s)",
                            effect.effect_label, effect_id, count
                        )
                        .into(),
                    );
                }
            }
        }

        if !self.unsupported_layers.is_empty() {
            console::warn_1(
                &format!(
                    "[AM Validation] Unsupported layer types ({}):",
                    self.unsupported_layers.len()
                )
                .into(),
            );
            for layer in &self.unsupported_layers {
                console::warn_1(
                    &format!(
                        "  ✗ {} '{}' (id={}) - will be skipped",
                        layer.layer_type, layer.label, layer.id
                    )
                    .into(),
                );
            }
        }

        console::log_1(&"----------------------------------------".into());
        if self.unsupported_effects.is_empty() && self.unsupported_layers.is_empty() {
            console::log_1(
                &"[AM Validation] ✓ All features in this project are fully supported".into(),
            );
        } else {
            let total_issues = self.unsupported_effects.len() + self.unsupported_layers.len();
            console::warn_1(
                &format!(
                    "[AM Validation] ⚠ {} unsupported feature(s) will be skipped",
                    total_issues
                )
                .into(),
            );
        }
        console::log_1(&"========================================".into());
    }
}
