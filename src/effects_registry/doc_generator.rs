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
use super::types::{BuiltinCategory, BuiltinDef, EffectDef, FieldDef, SupportLevel};

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

    // 根据实现扫描结果确定状态 / Determine status based on implementation scan
    let is_implemented = is_field_implemented(effect_id, field.name, config.impl_status);

    let (icon, status_text) = if is_implemented {
        if lang == "zh-hans" {
            (SupportLevel::Full.icon(), "已实现")
        } else {
            (SupportLevel::Full.icon(), "Implemented")
        }
    } else if lang == "zh-hans" {
        (SupportLevel::Unsupported.icon(), "未实现")
    } else {
        (SupportLevel::Unsupported.icon(), "Not implemented")
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

/// 获取支持级别的统计信息 / Get support level statistics
pub fn get_support_stats(effects: &[&EffectDef], config: &DocGeneratorConfig) -> SupportStats {
    let mut stats = SupportStats::default();

    for effect in effects {
        let support_level = get_effect_support_level(effect, config);

        match support_level {
            SupportLevel::Full => stats.full_count += 1,
            SupportLevel::Partial => stats.partial_count += 1,
            SupportLevel::Unsupported => stats.unsupported_count += 1,
        }
    }

    stats
}

/// 支持级别统计 / Support level statistics
#[derive(Debug, Default)]
pub struct SupportStats {
    pub full_count: usize,
    pub partial_count: usize,
    pub unsupported_count: usize,
}

/// 生成 VitePress 效果侧边栏配置 / Generate VitePress effects sidebar config
pub fn generate_vitepress_effects_sidebar(effects: &[&EffectDef], lang: &str) -> String {
    let mut items = String::new();

    for (i, effect) in effects.iter().enumerate() {
        let name = if lang == "zh-hans" {
            effect.display_name_zh
        } else {
            effect.display_name_en
        };
        let link = format!("/{}/effects/{}", lang, effect.short_name);

        if i > 0 {
            items.push_str(",\n");
        }
        items.push_str(&format!(
            "                {{ text: '{}', link: '{}' }}",
            name, link
        ));
    }

    items
}

/// 生成 VitePress 侧边栏 TypeScript 模块（包含效果和内置功能）
/// Generate VitePress sidebar TypeScript module (including effects and builtins)
pub fn generate_vitepress_sidebar_snippet(
    effects: &[&EffectDef],
    builtins: &[&BuiltinDef],
    config: &DocGeneratorConfig,
) -> String {
    let mut output = String::new();

    writeln!(
        output,
        "// ⚠️ 此文件由 `cargo run --example generate_docs` 自动生成，请勿手动编辑。"
    )
    .unwrap();
    writeln!(output, "// ⚠️ This file is auto-generated by `cargo run --example generate_docs`. Do not edit manually.").unwrap();
    writeln!(output).unwrap();

    // 辅助函数：根据支持级别获取颜色标记
    // Helper function: get color indicator based on support level
    fn get_color_indicator(level: SupportLevel) -> &'static str {
        match level {
            // 使用非常淡的颜色 / Using very light colors
            // 绿色圆点表示完全支持 / Green dot for full support
            SupportLevel::Full => "<span style=\"color:#22c55e\">●</span> ",
            // 黄色圆点表示部分支持 / Yellow dot for partial support
            SupportLevel::Partial => "<span style=\"color:#eab308\">●</span> ",
            // 红色圆点表示不支持 / Red dot for unsupported
            SupportLevel::Unsupported => "<span style=\"color:#ef4444\">●</span> ",
        }
    }

    // 中文效果
    writeln!(output, "export const zhHansEffects = {{").unwrap();
    writeln!(output, "  text: '高级效果',").unwrap();
    writeln!(output, "  items: [").unwrap();
    for (i, effect) in effects.iter().enumerate() {
        let comma = if i < effects.len() - 1 { "," } else { "" };
        // 计算支持级别 / Calculate support level
        let support_level = get_effect_support_level(effect, config);
        let indicator = get_color_indicator(support_level);
        writeln!(
            output,
            "    {{ text: '{}{}', link: '/zh-hans/effects/{}' }}{}",
            indicator, effect.display_name_zh, effect.short_name, comma
        )
        .unwrap();
    }
    writeln!(output, "  ]").unwrap();
    writeln!(output, "}};").unwrap();
    writeln!(output).unwrap();

    // 英文效果
    writeln!(output, "export const enEffects = {{").unwrap();
    writeln!(output, "  text: 'Advanced Effects',").unwrap();
    writeln!(output, "  items: [").unwrap();
    for (i, effect) in effects.iter().enumerate() {
        let comma = if i < effects.len() - 1 { "," } else { "" };
        let support_level = get_effect_support_level(effect, config);
        let indicator = get_color_indicator(support_level);
        writeln!(
            output,
            "    {{ text: '{}{}', link: '/en/effects/{}' }}{}",
            indicator, effect.display_name_en, effect.short_name, comma
        )
        .unwrap();
    }
    writeln!(output, "  ]").unwrap();
    writeln!(output, "}};").unwrap();
    writeln!(output).unwrap();

    // 按分类分组内置功能 / Group builtins by category
    let categories = [
        BuiltinCategory::Properties,
        BuiltinCategory::Shapes,
        BuiltinCategory::Fill,
    ];

    // 中文内置功能 - 按分类分组
    writeln!(output, "export const zhHansBuiltins = [").unwrap();
    for (cat_idx, category) in categories.iter().enumerate() {
        let cat_builtins: Vec<_> = builtins
            .iter()
            .filter(|b| b.category == *category)
            .collect();
        if cat_builtins.is_empty() {
            continue;
        }

        let cat_comma = if cat_idx < categories.len() - 1 {
            ","
        } else {
            ""
        };
        writeln!(output, "  {{").unwrap();
        writeln!(output, "    text: '{}',", category.display_name_zh()).unwrap();
        writeln!(output, "    items: [").unwrap();
        for (i, builtin) in cat_builtins.iter().enumerate() {
            let comma = if i < cat_builtins.len() - 1 { "," } else { "" };
            let support_level = get_builtin_support_level(builtin, config);
            let indicator = get_color_indicator(support_level);
            writeln!(
                output,
                "      {{ text: '{}{}', link: '/zh-hans/builtins/{}' }}{}",
                indicator, builtin.display_name_zh, builtin.short_name, comma
            )
            .unwrap();
        }
        writeln!(output, "    ]").unwrap();
        writeln!(output, "  }}{}", cat_comma).unwrap();
    }
    writeln!(output, "];").unwrap();
    writeln!(output).unwrap();

    // 英文内置功能 - 按分类分组
    writeln!(output, "export const enBuiltins = [").unwrap();
    for (cat_idx, category) in categories.iter().enumerate() {
        let cat_builtins: Vec<_> = builtins
            .iter()
            .filter(|b| b.category == *category)
            .collect();
        if cat_builtins.is_empty() {
            continue;
        }

        let cat_comma = if cat_idx < categories.len() - 1 {
            ","
        } else {
            ""
        };
        writeln!(output, "  {{").unwrap();
        writeln!(output, "    text: '{}',", category.display_name_en()).unwrap();
        writeln!(output, "    items: [").unwrap();
        for (i, builtin) in cat_builtins.iter().enumerate() {
            let comma = if i < cat_builtins.len() - 1 { "," } else { "" };
            let support_level = get_builtin_support_level(builtin, config);
            let indicator = get_color_indicator(support_level);
            writeln!(
                output,
                "      {{ text: '{}{}', link: '/en/builtins/{}' }}{}",
                indicator, builtin.display_name_en, builtin.short_name, comma
            )
            .unwrap();
        }
        writeln!(output, "    ]").unwrap();
        writeln!(output, "  }}{}", cat_comma).unwrap();
    }
    writeln!(output, "];").unwrap();

    output
}

/// 获取效果的支持级别 / Get effect support level
fn get_effect_support_level(effect: &EffectDef, config: &DocGeneratorConfig) -> SupportLevel {
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

    // 1. 优先根据测试结果计算 / First try to compute from test results
    if let Some(level) = config
        .test_results
        .and_then(|r| r.compute_support_level(&test_files))
    {
        return level;
    }

    // 2. 没有测试结果时，根据代码实现状态判断 / Fall back to implementation status
    if let Some(impl_info) = config.impl_status.and_then(|s| s.get(effect.id)) {
        // 有实现的字段或模式 / Has implemented fields or patterns
        if !impl_info.implemented_fields.is_empty() || !impl_info.pattern_fields.is_empty() {
            // 检查是否实现了所有定义的字段 / Check if all defined fields are implemented
            let total_fields = effect.fields.len();
            let implemented_count = effect
                .fields
                .iter()
                .filter(|f| {
                    impl_info.implemented_fields.iter().any(|s| s == f.name)
                        || impl_info
                            .pattern_fields
                            .iter()
                            .any(|p: &String| f.name.starts_with(p.trim_end_matches('*')))
                })
                .count();

            return if implemented_count == total_fields && total_fields > 0 {
                SupportLevel::Full
            } else if implemented_count > 0 {
                SupportLevel::Partial
            } else {
                SupportLevel::Unsupported
            };
        }
    }

    // 3. 无法判断时使用定义中的默认值 / Use default from definition
    effect.support_level
}

/// 获取内置功能的支持级别 / Get builtin support level
fn get_builtin_support_level(builtin: &BuiltinDef, config: &DocGeneratorConfig) -> SupportLevel {
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
