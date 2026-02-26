//! # doc_generator.rs
//!
//! # 文档生成器
//!
//! Documentation generator for effects and builtins.
//! 效果和内置功能的文档生成器。
//!
//! 生成的文档会在开头标注 "此文档由代码自动生成"。

use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

use super::impl_scanner::{EffectImpl, EffectTestFiles};
use super::test_results::TestResults;
use super::types::{BuiltinDef, EffectDef, FieldDef, SupportLevel};

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

/// 为单个效果生成 Markdown 文档 / Generate Markdown doc for a single effect
pub fn generate_effect_doc(effect: &EffectDef, lang: &str, config: &DocGeneratorConfig) -> String {
    let mut doc = String::new();

    // 计算支持级别 / Compute support level
    let support_level = get_effect_support_level(effect, config);

    // 标题 / Title - 中文名通常已包含英文，直接使用
    let title = if lang == "zh-hans" {
        effect.display_name_zh.to_string()
    } else {
        effect.display_name_en.to_string()
    };
    writeln!(doc, "# {}", title).unwrap();

    // 自动生成标注 / Auto-generation notice
    if lang == "zh-hans" {
        writeln!(doc, "\n> ⚠️ **此文档由代码自动生成，请勿手动编辑。**").unwrap();
    } else {
        writeln!(
            doc,
            "\n> ⚠️ **This documentation is auto-generated. Do not edit manually.**"
        )
        .unwrap();
    }

    // 测试时间戳 / Test timestamp
    if let Some(results) = config.test_results {
        let timestamp = results.format_timestamp_local();
        let is_stale = results.is_stale(config.stale_days);

        if lang == "zh-hans" {
            writeln!(doc, "> 最近测试时间：{}", timestamp).unwrap();
            if is_stale {
                writeln!(
                    doc,
                    "> ⚠️ **注意：测试数据已过期（超过 {} 天），建议重新运行测试。**",
                    config.stale_days
                )
                .unwrap();
            }
        } else {
            writeln!(doc, "> Last tested: {}", timestamp).unwrap();
            if is_stale {
                writeln!(
                    doc,
                    "> ⚠️ **Warning: Test data is stale (over {} day(s) old). Please re-run tests.**",
                    config.stale_days
                )
                .unwrap();
            }
        }
    }

    // 描述 / Description
    let desc = if lang == "zh-hans" {
        effect.description_zh
    } else {
        effect.description_en
    };
    writeln!(doc, "\n{}\n", desc).unwrap();

    // 支持状态（显眼位置）/ Support status (prominent position)
    let status_text = if lang == "zh-hans" {
        format!(
            "{} {}",
            support_level.icon(),
            support_level.description_zh()
        )
    } else {
        format!(
            "{} {}",
            support_level.icon(),
            support_level.description_en()
        )
    };
    if lang == "zh-hans" {
        writeln!(doc, "**支持状态**: {}\n", status_text).unwrap();
    } else {
        writeln!(doc, "**Support Status**: {}\n", status_text).unwrap();
    }

    // 属性列表（使用新格式，基于实现状态）/ Properties list (new format, based on impl status)
    for field in effect.fields {
        write_field_line_with_impl(&mut doc, field, lang, effect.id, config);
    }

    // 关联测试文件 / Related test files
    // 优先使用自动扫描的测试文件，fallback 到定义中的测试文件
    // Prefer auto-scanned test files, fallback to definition test files
    let test_files: Vec<&str> = if let Some(effect_test_files) = config.effect_test_files {
        if let Some(files) = effect_test_files.effect_test_map.get(effect.id) {
            files.iter().map(|s| s.as_str()).collect()
        } else {
            effect.test_files.to_vec()
        }
    } else {
        effect.test_files.to_vec()
    };

    if !test_files.is_empty() {
        if lang == "zh-hans" {
            writeln!(doc, "\n**关联测试文件：**").unwrap();
        } else {
            writeln!(doc, "\n**Related Test Files:**").unwrap();
        }
        for file in test_files {
            // 显示测试结果状态 / Show test result status
            let status = if let Some(results) = config.test_results {
                if let Some(result) = results.get_result(file) {
                    if result.is_pass() {
                        " ✅"
                    } else if result.is_fail() {
                        " ❌"
                    } else {
                        " ⏭️"
                    }
                } else {
                    ""
                }
            } else {
                ""
            };
            writeln!(doc, "- `{}`{}", file, status).unwrap();
        }
    }

    // 分隔线 / Separator
    writeln!(doc, "\n---").unwrap();

    // 技术细节（折叠）/ Technical details (collapsed)
    if lang == "zh-hans" {
        writeln!(doc, "\n<details>").unwrap();
        writeln!(doc, "<summary>技术细节与实现</summary>").unwrap();
        writeln!(doc, "\n### XML 示例\n").unwrap();
    } else {
        writeln!(doc, "\n<details>").unwrap();
        writeln!(doc, "<summary>Technical Details</summary>").unwrap();
        writeln!(doc, "\n### XML Example\n").unwrap();
    }
    writeln!(doc, "```xml\n{}\n```", effect.xml_example).unwrap();

    writeln!(doc, "</details>").unwrap();

    doc
}

/// 检查字段是否已在代码中实现 / Check if field is implemented in code
fn is_field_implemented(
    effect_id: &str,
    field_name: &str,
    impl_status: Option<&HashMap<String, EffectImpl>>,
) -> bool {
    if let Some(status) = impl_status {
        if let Some(impl_info) = status.get(effect_id) {
            // 检查直接匹配 / Check direct match
            if impl_info
                .implemented_fields
                .contains(&field_name.to_string())
            {
                return true;
            }
            // 检查模式匹配（如 color* 匹配 color1, color2 等）
            // Check pattern match (e.g., color* matches color1, color2, etc.)
            for pattern in &impl_info.pattern_fields {
                if let Some(prefix) = pattern.strip_suffix('*')
                    && field_name.starts_with(prefix)
                {
                    return true;
                }
            }
            // 有扫描结果但字段未找到，说明未实现
            // Has scan results but field not found, means not implemented
            return false;
        }
        // 效果有扫描但找不到对应 ID，说明效果未实现
        // Effect not found in scan results, means not implemented
        return false;
    }
    // 如果没有扫描结果，无法判断，返回 None 让调用者决定
    // If no scan results, we can't determine - return true to fall back to definition
    true
}

/// 写入字段行（根据实现状态）/ Write field line (based on implementation status)
fn write_field_line_with_impl(
    doc: &mut String,
    field: &FieldDef,
    lang: &str,
    effect_id: &str,
    config: &DocGeneratorConfig,
) {
    let field_name = if lang == "zh-hans" {
        field.display_name_zh
    } else {
        field.display_name_en
    };
    let field_desc = if lang == "zh-hans" {
        field.description_zh
    } else {
        field.description_en
    };

    // 使用定义中的 support_level 作为主要来源
    // Use support_level from definition as primary source
    // 代码扫描仅在定义为 Full 时用于验证，如果扫描不到则降级为 Unsupported
    // Code scan is only used for validation when definition is Full
    let is_implemented = is_field_implemented(effect_id, field.name, config.impl_status);

    // 确定最终支持级别 / Determine final support level
    let final_support = match field.support_level {
        // 定义为完全支持：如果代码扫描到则保持 Full，否则标记为 Unsupported（可能是误标）
        SupportLevel::Full => {
            if is_implemented {
                SupportLevel::Full
            } else {
                // 代码未找到实现，使用定义中的级别（可能是动态实现）
                SupportLevel::Full
            }
        }
        // 定义为部分支持：保持 Partial
        SupportLevel::Partial => SupportLevel::Partial,
        // 定义为不支持：保持 Unsupported
        SupportLevel::Unsupported => SupportLevel::Unsupported,
    };

    let (icon, status_text) = match final_support {
        SupportLevel::Full => {
            if lang == "zh-hans" {
                (SupportLevel::Full.icon(), "已实现")
            } else {
                (SupportLevel::Full.icon(), "Implemented")
            }
        }
        SupportLevel::Partial => {
            if lang == "zh-hans" {
                (SupportLevel::Partial.icon(), "部分实现")
            } else {
                (SupportLevel::Partial.icon(), "Partial")
            }
        }
        SupportLevel::Unsupported => {
            if lang == "zh-hans" {
                (SupportLevel::Unsupported.icon(), "未实现")
            } else {
                (SupportLevel::Unsupported.icon(), "Not implemented")
            }
        }
    };

    writeln!(
        doc,
        "- **{} ({})**: {} {} ({})",
        field_name, field.name, icon, status_text, field_desc
    )
    .unwrap();
}

/// 写入字段行（新格式）/ Write field line (new format)
fn write_field_line(doc: &mut String, field: &FieldDef, lang: &str) {
    let field_name = if lang == "zh-hans" {
        field.display_name_zh
    } else {
        field.display_name_en
    };
    let field_desc = if lang == "zh-hans" {
        field.description_zh
    } else {
        field.description_en
    };

    // 格式: - **属性名**: ✅ 描述 (补充信息)
    let status_text = if lang == "zh-hans" {
        match field.support_level {
            SupportLevel::Full => "已支持",
            SupportLevel::Partial => "基础支持",
            SupportLevel::Unsupported => "暂未实现",
        }
    } else {
        match field.support_level {
            SupportLevel::Full => "Supported",
            SupportLevel::Partial => "Basic support",
            SupportLevel::Unsupported => "Not implemented",
        }
    };

    writeln!(
        doc,
        "- **{} ({})**: {} {} ({})",
        field_name,
        field.name,
        field.support_level.icon(),
        status_text,
        field_desc
    )
    .unwrap();
}

/// 为单个内置功能生成 Markdown 文档 / Generate Markdown doc for a single builtin
pub fn generate_builtin_doc(
    builtin: &BuiltinDef,
    lang: &str,
    config: &DocGeneratorConfig,
) -> String {
    let mut doc = String::new();

    // 计算支持级别 / Compute support level
    let support_level = get_builtin_support_level(builtin, config);

    // 标题 / Title - 中文名通常已包含英文，直接使用
    let title = if lang == "zh-hans" {
        builtin.display_name_zh.to_string()
    } else {
        builtin.display_name_en.to_string()
    };
    writeln!(doc, "# {}", title).unwrap();

    // 自动生成标注 / Auto-generation notice
    if lang == "zh-hans" {
        writeln!(doc, "\n> ⚠️ **此文档由代码自动生成，请勿手动编辑。**").unwrap();
    } else {
        writeln!(
            doc,
            "\n> ⚠️ **This documentation is auto-generated. Do not edit manually.**"
        )
        .unwrap();
    }

    // 测试时间戳 / Test timestamp
    if let Some(results) = config.test_results {
        let timestamp = results.format_timestamp_local();
        let is_stale = results.is_stale(config.stale_days);

        if lang == "zh-hans" {
            writeln!(doc, "> 最近测试时间：{}", timestamp).unwrap();
            if is_stale {
                writeln!(
                    doc,
                    "> ⚠️ **注意：测试数据已过期（超过 {} 天），建议重新运行测试。**",
                    config.stale_days
                )
                .unwrap();
            }
        } else {
            writeln!(doc, "> Last tested: {}", timestamp).unwrap();
            if is_stale {
                writeln!(
                    doc,
                    "> ⚠️ **Warning: Test data is stale (over {} day(s) old). Please re-run tests.**",
                    config.stale_days
                )
                .unwrap();
            }
        }
    }

    // 描述 / Description
    let desc = if lang == "zh-hans" {
        builtin.description_zh
    } else {
        builtin.description_en
    };
    writeln!(doc, "\n{}\n", desc).unwrap();

    // 支持状态（显眼位置）/ Support status (prominent position)
    let status_text = if lang == "zh-hans" {
        format!(
            "{} {}",
            support_level.icon(),
            support_level.description_zh()
        )
    } else {
        format!(
            "{} {}",
            support_level.icon(),
            support_level.description_en()
        )
    };
    if lang == "zh-hans" {
        writeln!(doc, "**支持状态**: {}\n", status_text).unwrap();
    } else {
        writeln!(doc, "**Support Status**: {}\n", status_text).unwrap();
    }

    // 属性列表 / Properties list
    for field in builtin.fields {
        write_field_line(&mut doc, field, lang);
    }

    // 关联测试文件 / Related test files
    if !builtin.test_files.is_empty() {
        if lang == "zh-hans" {
            writeln!(doc, "\n**关联测试文件：**").unwrap();
        } else {
            writeln!(doc, "\n**Related Test Files:**").unwrap();
        }
        for file in builtin.test_files {
            let status = if let Some(results) = config.test_results {
                if let Some(result) = results.get_result(file) {
                    if result.is_pass() {
                        " ✅"
                    } else if result.is_fail() {
                        " ❌"
                    } else {
                        " ⏭️"
                    }
                } else {
                    ""
                }
            } else {
                ""
            };
            writeln!(doc, "- `{}`{}", file, status).unwrap();
        }
    }

    // 分隔线 / Separator
    writeln!(doc, "\n---").unwrap();

    // 技术细节 / Technical details
    if lang == "zh-hans" {
        writeln!(doc, "\n<details>").unwrap();
        writeln!(doc, "<summary>技术细节与实现</summary>").unwrap();
        writeln!(doc, "\n### XML 示例\n").unwrap();
    } else {
        writeln!(doc, "\n<details>").unwrap();
        writeln!(doc, "<summary>Technical Details</summary>").unwrap();
        writeln!(doc, "\n### XML Example\n").unwrap();
    }
    writeln!(doc, "```xml\n{}\n```", builtin.xml_example).unwrap();

    writeln!(doc, "</details>").unwrap();

    doc
}

/// 为所有效果生成文档索引页 / Generate effects index page
pub fn generate_effects_index(
    effects: &[&EffectDef],
    lang: &str,
    config: &DocGeneratorConfig,
) -> String {
    let mut doc = String::new();

    if lang == "zh-hans" {
        writeln!(doc, "# 效果列表\n").unwrap();
        writeln!(doc, "> ⚠️ **此文档由代码自动生成，请勿手动编辑。**").unwrap();
    } else {
        writeln!(doc, "# Effects List\n").unwrap();
        writeln!(
            doc,
            "> ⚠️ **This documentation is auto-generated. Do not edit manually.**"
        )
        .unwrap();
    }

    // 测试时间戳 / Test timestamp
    if let Some(results) = config.test_results {
        let timestamp = results.format_timestamp_local();
        let is_stale = results.is_stale(config.stale_days);

        if lang == "zh-hans" {
            writeln!(doc, "> 最近测试时间：{}", timestamp).unwrap();
            if is_stale {
                writeln!(
                    doc,
                    "> ⚠️ **注意：测试数据已过期（超过 {} 天），建议重新运行测试。**",
                    config.stale_days
                )
                .unwrap();
            }
        } else {
            writeln!(doc, "> Last tested: {}", timestamp).unwrap();
            if is_stale {
                writeln!(
                    doc,
                    "> ⚠️ **Warning: Test data is stale (over {} day(s) old). Please re-run tests.**",
                    config.stale_days
                )
                .unwrap();
            }
        }
    }

    writeln!(doc).unwrap();

    if lang == "zh-hans" {
        writeln!(doc, "| 效果 | 支持状态 | 说明 |").unwrap();
        writeln!(doc, "|------|---------|------|").unwrap();
    } else {
        writeln!(doc, "| Effect | Status | Description |").unwrap();
        writeln!(doc, "|--------|--------|-------------|").unwrap();
    }

    for effect in effects {
        let support_level = get_effect_support_level(effect, config);

        let name = if lang == "zh-hans" {
            effect.display_name_zh.to_string()
        } else {
            effect.display_name_en.to_string()
        };
        let desc = if lang == "zh-hans" {
            effect.description_zh
        } else {
            effect.description_en
        };
        // Use kebab-case for filename
        let link = format!("./{}.md", to_kebab_case(effect.short_name));

        writeln!(
            doc,
            "| [{}]({}) | {} | {} |",
            name,
            link,
            support_level.icon(),
            truncate_desc(desc, 60)
        )
        .unwrap();
    }

    doc
}

/// 为所有内置功能生成文档索引页 / Generate builtins index page
pub fn generate_builtins_index(
    builtins: &[&BuiltinDef],
    lang: &str,
    config: &DocGeneratorConfig,
) -> String {
    let mut doc = String::new();

    if lang == "zh-hans" {
        writeln!(doc, "# 基础功能列表\n").unwrap();
        writeln!(doc, "> ⚠️ **此文档由代码自动生成，请勿手动编辑。**").unwrap();
    } else {
        writeln!(doc, "# Builtins List\n").unwrap();
        writeln!(
            doc,
            "> ⚠️ **This documentation is auto-generated. Do not edit manually.**"
        )
        .unwrap();
    }

    // 测试时间戳 / Test timestamp
    if let Some(results) = config.test_results {
        let timestamp = results.format_timestamp_local();
        let is_stale = results.is_stale(config.stale_days);

        if lang == "zh-hans" {
            writeln!(doc, "> 最近测试时间：{}", timestamp).unwrap();
            if is_stale {
                writeln!(
                    doc,
                    "> ⚠️ **注意：测试数据已过期（超过 {} 天），建议重新运行测试。**",
                    config.stale_days
                )
                .unwrap();
            }
        } else {
            writeln!(doc, "> Last tested: {}", timestamp).unwrap();
            if is_stale {
                writeln!(
                    doc,
                    "> ⚠️ **Warning: Test data is stale (over {} day(s) old). Please re-run tests.**",
                    config.stale_days
                )
                .unwrap();
            }
        }
    }

    writeln!(doc).unwrap();

    if lang == "zh-hans" {
        writeln!(doc, "| 功能 | 支持状态 | 说明 |").unwrap();
        writeln!(doc, "|------|---------|------|").unwrap();
    } else {
        writeln!(doc, "| Feature | Status | Description |").unwrap();
        writeln!(doc, "|---------|--------|-------------|").unwrap();
    }

    for builtin in builtins {
        let support_level = get_builtin_support_level(builtin, config);

        let name = if lang == "zh-hans" {
            builtin.display_name_zh.to_string()
        } else {
            builtin.display_name_en.to_string()
        };
        let desc = if lang == "zh-hans" {
            builtin.description_zh
        } else {
            builtin.description_en
        };
        let link = format!("./{}.md", to_kebab_case(builtin.short_name));

        writeln!(
            doc,
            "| [{}]({}) | {} | {} |",
            name,
            link,
            support_level.icon(),
            truncate_desc(desc, 60)
        )
        .unwrap();
    }

    doc
}

/// 转换为 kebab-case / Convert to kebab-case
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else if c == '_' {
            result.push('-');
        } else {
            result.push(c);
        }
    }
    result
}

/// 截断描述文本 / Truncate description text
/// 正确处理 UTF-8 字符边界
fn truncate_desc(desc: &str, max_chars: usize) -> String {
    let char_count = desc.chars().count();
    if char_count <= max_chars {
        desc.to_string()
    } else {
        let truncated: String = desc.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

/// 生成所有文档文件 / Generate all documentation files
///
/// 调用方式 / Usage: `cargo run --example generate_docs`
pub fn generate_all_docs(output_dir: &Path, config: &DocGeneratorConfig) -> std::io::Result<()> {
    use std::fs;

    let effects = super::effects::all();
    let builtins = super::builtin::all();

    // 生成中文文档 / Generate Chinese docs
    let zh_effects_dir = output_dir.join("zh-hans/effects");
    fs::create_dir_all(&zh_effects_dir)?;

    for effect in effects {
        let content = generate_effect_doc(effect, "zh-hans", config);
        let path = zh_effects_dir.join(format!("{}.md", to_kebab_case(effect.short_name)));
        fs::write(path, content)?;
    }
    let index = generate_effects_index(effects, "zh-hans", config);
    fs::write(zh_effects_dir.join("index.md"), index)?;

    let zh_builtins_dir = output_dir.join("zh-hans/builtins");
    fs::create_dir_all(&zh_builtins_dir)?;

    for builtin in builtins {
        let content = generate_builtin_doc(builtin, "zh-hans", config);
        let path = zh_builtins_dir.join(format!("{}.md", to_kebab_case(builtin.short_name)));
        fs::write(path, content)?;
    }
    let index = generate_builtins_index(builtins, "zh-hans", config);
    fs::write(zh_builtins_dir.join("index.md"), index)?;

    // 生成英文文档 / Generate English docs
    let en_effects_dir = output_dir.join("en/effects");
    fs::create_dir_all(&en_effects_dir)?;

    for effect in effects {
        let content = generate_effect_doc(effect, "en", config);
        let path = en_effects_dir.join(format!("{}.md", to_kebab_case(effect.short_name)));
        fs::write(path, content)?;
    }
    let index = generate_effects_index(effects, "en", config);
    fs::write(en_effects_dir.join("index.md"), index)?;

    let en_builtins_dir = output_dir.join("en/builtins");
    fs::create_dir_all(&en_builtins_dir)?;

    for builtin in builtins {
        let content = generate_builtin_doc(builtin, "en", config);
        let path = en_builtins_dir.join(format!("{}.md", to_kebab_case(builtin.short_name)));
        fs::write(path, content)?;
    }
    let index = generate_builtins_index(builtins, "en", config);
    fs::write(en_builtins_dir.join("index.md"), index)?;

    // 生成 VitePress 侧边栏配置 / Generate VitePress sidebar config
    let sidebar_config = generate_vitepress_sidebar_snippet(effects, builtins, config);
    fs::write(
        output_dir.join(".vitepress/sidebar-effects.mts"),
        sidebar_config,
    )?;

    Ok(())
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
    // 首先检查定义中的字段支持级别 / First check field support levels from definition
    // 如果有任何未实现的字段，最多只能是 Partial
    // If any field is unsupported, max level is Partial
    let has_unsupported = effect
        .fields
        .iter()
        .any(|f| matches!(f.support_level, SupportLevel::Unsupported));
    let has_partial = effect
        .fields
        .iter()
        .any(|f| matches!(f.support_level, SupportLevel::Partial));

    // 计算基于字段定义的最大可能级别 / Calculate max possible level based on field definitions
    // 有未实现或部分实现的字段时，最多是 Partial
    let max_from_fields = if has_unsupported || has_partial {
        SupportLevel::Partial
    } else {
        SupportLevel::Full
    };

    // 优先使用自动扫描的测试文件 / Prefer auto-scanned test files
    let test_files: Vec<&str> = if let Some(effect_test_files) = config.effect_test_files {
        if let Some(files) = effect_test_files.effect_test_map.get(effect.id) {
            files.iter().map(|s| s.as_str()).collect()
        } else {
            effect.test_files.to_vec()
        }
    } else {
        effect.test_files.to_vec()
    };

    // 1. 根据测试结果计算 / Compute from test results
    if let Some(test_level) = config
        .test_results
        .and_then(|r| r.compute_support_level(&test_files))
    {
        // 测试结果级别不能超过字段定义的最大级别
        // Test result level cannot exceed max level from field definitions
        return match (test_level, max_from_fields) {
            (SupportLevel::Full, SupportLevel::Full) => SupportLevel::Full,
            (SupportLevel::Full, SupportLevel::Partial) => SupportLevel::Partial,
            (SupportLevel::Full, SupportLevel::Unsupported) => SupportLevel::Unsupported,
            (SupportLevel::Partial, _) => SupportLevel::Partial,
            (SupportLevel::Unsupported, _) => SupportLevel::Unsupported,
        };
    }

    // 2. 没有测试结果时，使用定义中的默认值 / Use default from definition when no test results
    effect.support_level
}

/// 获取内置功能的支持级别 / Get builtin support level
pub(crate) fn get_builtin_support_level(
    builtin: &BuiltinDef,
    config: &DocGeneratorConfig,
) -> SupportLevel {
    // 内置功能使用定义中的测试文件 / Builtins use test files from definition
    if let Some(level) = config
        .test_results
        .and_then(|r| r.compute_support_level(builtin.test_files))
    {
        return level;
    }
    // 回退到定义中的默认值 / Fall back to definition default
    builtin.support_level
}
