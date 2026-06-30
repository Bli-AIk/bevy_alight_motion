//! # doc_generator.rs
//!
//! Documentation generator for effects and builtins.

mod pages;
mod render;

use std::collections::HashMap;

use super::impl_scanner::{EffectImpl, EffectTestFiles};
use super::test_results::TestResults;
use super::types::{BuiltinDef, EffectDef, SupportLevel};

pub use pages::{
    generate_all_docs, generate_builtin_doc, generate_builtins_index, generate_effect_doc,
    generate_effects_index,
};

/// 文档生成配置 / Documentation generation configuration
pub struct DocGeneratorConfig<'a> {
    /// 测试结果（可选）/ Test results (optional)
    pub test_results: Option<&'a TestResults>,
    /// 实现状态扫描结果（可选）/ Implementation scan results (optional)
    pub impl_status: Option<&'a HashMap<String, EffectImpl>>,
    /// 效果测试文件关联（可选）/ Effect test files mapping (optional)
    pub effect_test_files: Option<&'a EffectTestFiles>,
    /// 过期天数阈值 / Stale threshold in days
    pub stale_days: i64,
}

impl Default for DocGeneratorConfig<'_> {
    fn default() -> Self {
        Self {
            test_results: None,
            impl_status: None,
            effect_test_files: None,
            stale_days: 1,
        }
    }
}

// VitePress 相关功能已移至 vitepress.rs / VitePress functions moved to vitepress.rs
pub use super::vitepress::{
    SupportStats, generate_vitepress_effects_sidebar, generate_vitepress_sidebar_snippet,
    get_support_stats,
};

/// 获取效果的支持级别 / Get effect support level
pub(crate) fn get_effect_support_level(
    effect: &EffectDef,
    config: &DocGeneratorConfig,
) -> SupportLevel {
    let has_unsupported = effect
        .fields
        .iter()
        .any(|field| matches!(field.support_level, SupportLevel::Unsupported));
    let has_partial = effect
        .fields
        .iter()
        .any(|field| matches!(field.support_level, SupportLevel::Partial));

    let max_from_fields = if has_unsupported || has_partial {
        SupportLevel::Partial
    } else {
        SupportLevel::Full
    };

    let test_files: Vec<&str> = if let Some(effect_test_files) = config.effect_test_files {
        if let Some(files) = effect_test_files.effect_test_map.get(effect.id) {
            files.iter().map(|s| s.as_str()).collect()
        } else {
            effect.test_files.to_vec()
        }
    } else {
        effect.test_files.to_vec()
    };

    if let Some(test_level) = config
        .test_results
        .and_then(|results| results.compute_support_level(&test_files))
    {
        return match (test_level, max_from_fields) {
            (SupportLevel::Full, SupportLevel::Full) => SupportLevel::Full,
            (SupportLevel::Full, SupportLevel::Partial) => SupportLevel::Partial,
            (SupportLevel::Full, SupportLevel::Unsupported) => SupportLevel::Unsupported,
            (SupportLevel::Partial, _) => SupportLevel::Partial,
            (SupportLevel::Unsupported, _) => SupportLevel::Unsupported,
        };
    }

    effect.support_level
}

/// 获取内置功能的支持级别 / Get builtin support level
pub(crate) fn get_builtin_support_level(
    builtin: &BuiltinDef,
    config: &DocGeneratorConfig,
) -> SupportLevel {
    if let Some(level) = config
        .test_results
        .and_then(|results| results.compute_support_level(builtin.test_files))
    {
        return level;
    }
    builtin.support_level
}
