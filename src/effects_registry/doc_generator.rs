//! # doc_generator.rs
//!
//! # 文档生成器
//!
//! Documentation generator for effects and builtins.
//! 效果和内置功能的文档生成器。
//!
//! 生成的文档会在开头标注 "此文档由代码自动生成"。

use std::fmt::Write;

use super::types::{BuiltinDef, EffectDef, FieldType, SupportLevel};

/// 为单个效果生成 Markdown 文档 / Generate Markdown doc for a single effect
pub fn generate_effect_doc(effect: &EffectDef, lang: &str) -> String {
    let mut doc = String::new();

    // 标题 / Title
    let title = if lang == "zh-hans" {
        effect.display_name_zh
    } else {
        effect.display_name_en
    };
    writeln!(doc, "# {}", title).unwrap();

    // 自动生成标注 / Auto-generation notice
    if lang == "zh-hans" {
        writeln!(doc, "\n> ⚠️ **此文档由代码自动生成，请勿手动编辑。**").unwrap();
        writeln!(
            doc,
            "> 源定义位置：`src/effects_registry/effects/{}.rs`\n",
            effect.short_name
        )
        .unwrap();
    } else {
        writeln!(
            doc,
            "\n> ⚠️ **This documentation is auto-generated. Do not edit manually.**"
        )
        .unwrap();
        writeln!(
            doc,
            "> Source definition: `src/effects_registry/effects/{}.rs`\n",
            effect.short_name
        )
        .unwrap();
    }

    // 描述 / Description
    let desc = if lang == "zh-hans" {
        effect.description_zh
    } else {
        effect.description_en
    };
    writeln!(doc, "{}\n", desc).unwrap();

    // 整体支持状态 / Overall support status
    let status_text = if lang == "zh-hans" {
        format!(
            "{} {}",
            effect.support_level.icon(),
            effect.support_level.description_zh()
        )
    } else {
        format!(
            "{} {}",
            effect.support_level.icon(),
            effect.support_level.description_en()
        )
    };
    if lang == "zh-hans" {
        writeln!(doc, "**支持状态**: {}\n", status_text).unwrap();
    } else {
        writeln!(doc, "**Support Status**: {}\n", status_text).unwrap();
    }

    // 属性表格 / Properties table
    if lang == "zh-hans" {
        writeln!(doc, "## 属性\n").unwrap();
        writeln!(doc, "| 属性 | 类型 | 支持状态 | 默认值 | 说明 |").unwrap();
        writeln!(doc, "|------|------|---------|--------|------|").unwrap();
    } else {
        writeln!(doc, "## Properties\n").unwrap();
        writeln!(
            doc,
            "| Property | Type | Status | Default | Description |"
        )
        .unwrap();
        writeln!(doc, "|----------|------|--------|---------|-------------|").unwrap();
    }

    for field in effect.fields {
        write_field_row(&mut doc, field, lang);
    }

    // XML 示例 / XML example
    if lang == "zh-hans" {
        writeln!(doc, "\n## XML 示例\n").unwrap();
    } else {
        writeln!(doc, "\n## XML Example\n").unwrap();
    }
    writeln!(doc, "```xml\n{}\n```", effect.xml_example).unwrap();

    // 关联测试文件 / Related test files
    if !effect.test_files.is_empty() {
        if lang == "zh-hans" {
            writeln!(doc, "\n## 关联测试文件\n").unwrap();
        } else {
            writeln!(doc, "\n## Related Test Files\n").unwrap();
        }
        for file in effect.test_files {
            writeln!(doc, "- `{}`", file).unwrap();
        }
    }

    doc
}

/// 为单个内置功能生成 Markdown 文档 / Generate Markdown doc for a single builtin
pub fn generate_builtin_doc(builtin: &BuiltinDef, lang: &str) -> String {
    let mut doc = String::new();

    // 标题 / Title
    let title = if lang == "zh-hans" {
        builtin.display_name_zh
    } else {
        builtin.display_name_en
    };
    writeln!(doc, "# {}", title).unwrap();

    // 自动生成标注 / Auto-generation notice
    if lang == "zh-hans" {
        writeln!(doc, "\n> ⚠️ **此文档由代码自动生成，请勿手动编辑。**").unwrap();
        writeln!(
            doc,
            "> 源定义位置：`src/effects_registry/builtin/`\n"
        )
        .unwrap();
    } else {
        writeln!(
            doc,
            "\n> ⚠️ **This documentation is auto-generated. Do not edit manually.**"
        )
        .unwrap();
        writeln!(
            doc,
            "> Source definition: `src/effects_registry/builtin/`\n"
        )
        .unwrap();
    }

    // 描述 / Description
    let desc = if lang == "zh-hans" {
        builtin.description_zh
    } else {
        builtin.description_en
    };
    writeln!(doc, "{}\n", desc).unwrap();

    // 整体支持状态 / Overall support status
    let status_text = if lang == "zh-hans" {
        format!(
            "{} {}",
            builtin.support_level.icon(),
            builtin.support_level.description_zh()
        )
    } else {
        format!(
            "{} {}",
            builtin.support_level.icon(),
            builtin.support_level.description_en()
        )
    };
    if lang == "zh-hans" {
        writeln!(doc, "**支持状态**: {}\n", status_text).unwrap();
    } else {
        writeln!(doc, "**Support Status**: {}\n", status_text).unwrap();
    }

    // 属性表格 / Properties table
    if !builtin.fields.is_empty() {
        if lang == "zh-hans" {
            writeln!(doc, "## 属性\n").unwrap();
            writeln!(doc, "| 属性 | 类型 | 支持状态 | 默认值 | 说明 |").unwrap();
            writeln!(doc, "|------|------|---------|--------|------|").unwrap();
        } else {
            writeln!(doc, "## Properties\n").unwrap();
            writeln!(
                doc,
                "| Property | Type | Status | Default | Description |"
            )
            .unwrap();
            writeln!(doc, "|----------|------|--------|---------|-------------|").unwrap();
        }

        for field in builtin.fields {
            write_field_row(&mut doc, field, lang);
        }
    }

    // XML 示例 / XML example
    if lang == "zh-hans" {
        writeln!(doc, "\n## XML 示例\n").unwrap();
    } else {
        writeln!(doc, "\n## XML Example\n").unwrap();
    }
    writeln!(doc, "```xml\n{}\n```", builtin.xml_example).unwrap();

    // 关联测试文件 / Related test files
    if !builtin.test_files.is_empty() {
        if lang == "zh-hans" {
            writeln!(doc, "\n## 关联测试文件\n").unwrap();
        } else {
            writeln!(doc, "\n## Related Test Files\n").unwrap();
        }
        for file in builtin.test_files {
            writeln!(doc, "- `{}`", file).unwrap();
        }
    }

    doc
}

/// 写入字段行 / Write field row
fn write_field_row(doc: &mut String, field: &super::types::FieldDef, lang: &str) {
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
    let default = field.default_value.unwrap_or("-");
    let type_str = format_field_type(&field.field_type);

    writeln!(
        doc,
        "| {} (`{}`) | {} | {} | `{}` | {} |",
        field_name,
        field.name,
        type_str,
        field.support_level.icon(),
        default,
        field_desc
    )
    .unwrap();
}

/// 格式化字段类型 / Format field type
fn format_field_type(field_type: &FieldType) -> String {
    match field_type {
        FieldType::Enum(values) => format!("enum: {}", values.join(", ")),
        _ => field_type.name().to_string(),
    }
}

/// 为所有效果生成文档索引页 / Generate effects index page
pub fn generate_effects_index(effects: &[&EffectDef], lang: &str) -> String {
    let mut doc = String::new();

    if lang == "zh-hans" {
        writeln!(doc, "# 效果列表\n").unwrap();
        writeln!(doc, "> ⚠️ **此文档由代码自动生成，请勿手动编辑。**\n").unwrap();
        writeln!(doc, "| 效果 | 支持状态 | 说明 |").unwrap();
        writeln!(doc, "|------|---------|------|").unwrap();
    } else {
        writeln!(doc, "# Effects List\n").unwrap();
        writeln!(
            doc,
            "> ⚠️ **This documentation is auto-generated. Do not edit manually.**\n"
        )
        .unwrap();
        writeln!(doc, "| Effect | Status | Description |").unwrap();
        writeln!(doc, "|--------|--------|-------------|").unwrap();
    }

    for effect in effects {
        let name = if lang == "zh-hans" {
            effect.display_name_zh
        } else {
            effect.display_name_en
        };
        let desc = if lang == "zh-hans" {
            effect.description_zh
        } else {
            effect.description_en
        };
        let link = format!("./{}.md", effect.short_name);

        writeln!(
            doc,
            "| [{}]({}) | {} | {} |",
            name,
            link,
            effect.support_level.icon(),
            truncate_desc(desc, 60)
        )
        .unwrap();
    }

    doc
}

/// 为所有内置功能生成文档索引页 / Generate builtins index page
pub fn generate_builtins_index(builtins: &[&BuiltinDef], lang: &str) -> String {
    let mut doc = String::new();

    if lang == "zh-hans" {
        writeln!(doc, "# 基础功能列表\n").unwrap();
        writeln!(doc, "> ⚠️ **此文档由代码自动生成，请勿手动编辑。**\n").unwrap();
        writeln!(doc, "| 功能 | 支持状态 | 说明 |").unwrap();
        writeln!(doc, "|------|---------|------|").unwrap();
    } else {
        writeln!(doc, "# Builtins List\n").unwrap();
        writeln!(
            doc,
            "> ⚠️ **This documentation is auto-generated. Do not edit manually.**\n"
        )
        .unwrap();
        writeln!(doc, "| Feature | Status | Description |").unwrap();
        writeln!(doc, "|---------|--------|-------------|").unwrap();
    }

    for builtin in builtins {
        let name = if lang == "zh-hans" {
            builtin.display_name_zh
        } else {
            builtin.display_name_en
        };
        let desc = if lang == "zh-hans" {
            builtin.description_zh
        } else {
            builtin.description_en
        };
        let link = format!("./{}.md", builtin.short_name);

        writeln!(
            doc,
            "| [{}]({}) | {} | {} |",
            name,
            link,
            builtin.support_level.icon(),
            truncate_desc(desc, 60)
        )
        .unwrap();
    }

    doc
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
pub fn generate_all_docs(output_dir: &std::path::Path) -> std::io::Result<()> {
    use std::fs;

    let effects = super::effects::all();
    let builtins = super::builtin::all();

    // 生成中文文档 / Generate Chinese docs
    let zh_effects_dir = output_dir.join("zh-hans/effects");
    fs::create_dir_all(&zh_effects_dir)?;

    for effect in effects {
        let content = generate_effect_doc(effect, "zh-hans");
        let path = zh_effects_dir.join(format!("{}.md", effect.short_name));
        fs::write(path, content)?;
    }
    let index = generate_effects_index(effects, "zh-hans");
    fs::write(zh_effects_dir.join("_index.md"), index)?;

    let zh_builtins_dir = output_dir.join("zh-hans/builtins");
    fs::create_dir_all(&zh_builtins_dir)?;

    for builtin in builtins {
        let content = generate_builtin_doc(builtin, "zh-hans");
        let path = zh_builtins_dir.join(format!("{}.md", builtin.short_name));
        fs::write(path, content)?;
    }
    let index = generate_builtins_index(builtins, "zh-hans");
    fs::write(zh_builtins_dir.join("_index.md"), index)?;

    // 生成英文文档 / Generate English docs
    let en_effects_dir = output_dir.join("en/effects");
    fs::create_dir_all(&en_effects_dir)?;

    for effect in effects {
        let content = generate_effect_doc(effect, "en");
        let path = en_effects_dir.join(format!("{}.md", effect.short_name));
        fs::write(path, content)?;
    }
    let index = generate_effects_index(effects, "en");
    fs::write(en_effects_dir.join("_index.md"), index)?;

    let en_builtins_dir = output_dir.join("en/builtins");
    fs::create_dir_all(&en_builtins_dir)?;

    for builtin in builtins {
        let content = generate_builtin_doc(builtin, "en");
        let path = en_builtins_dir.join(format!("{}.md", builtin.short_name));
        fs::write(path, content)?;
    }
    let index = generate_builtins_index(builtins, "en");
    fs::write(en_builtins_dir.join("_index.md"), index)?;

    Ok(())
}

/// 获取支持级别的统计信息 / Get support level statistics
pub fn get_support_stats(effects: &[&EffectDef]) -> SupportStats {
    let mut stats = SupportStats::default();

    for effect in effects {
        match effect.support_level {
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
