//! Renders shared Markdown fragments for registry documentation generation.
//! 为注册表文档生成渲染共享的 Markdown 片段。
//!
//! Page generation relies on many repeated fragments such as headers, support badges, field rows,
//! and auto-generated notices. This file centralizes those rendering helpers so the page generator
//! can describe document structure without duplicating formatting rules.
//! 文档生成会重复使用许多片段，例如标题、支持等级标记、字段行和自动生成提示。
//! 这个文件把这些渲染辅助统一起来，让页面生成逻辑可以专注于文档结构，而不用到处重复格式规则。

use std::collections::HashMap;
use std::fmt::Write;

use crate::effects_registry::doc_generator::DocGeneratorConfig;

use super::super::impl_scanner::EffectImpl;
use super::super::test_results::TestResults;
use super::super::types::{FieldDef, SupportLevel};

pub(super) fn write_page_title(doc: &mut String, lang: &str, zh_title: &str, en_title: &str) {
    let title = if lang == "zh-hans" {
        zh_title
    } else {
        en_title
    };
    writeln!(doc, "# {}", title).unwrap();
}

pub(super) fn write_auto_generated_notice(doc: &mut String, lang: &str) {
    if lang == "zh-hans" {
        writeln!(doc, "\n> ⚠️ **此文档由代码自动生成，请勿手动编辑。**").unwrap();
    } else {
        writeln!(
            doc,
            "\n> ⚠️ **This documentation is auto-generated. Do not edit manually.**"
        )
        .unwrap();
    }
}

pub(super) fn write_test_timestamp(doc: &mut String, lang: &str, config: &DocGeneratorConfig) {
    let Some(results) = config.test_results else {
        return;
    };

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

pub(super) fn write_support_status(doc: &mut String, lang: &str, support_level: SupportLevel) {
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
}

pub(super) fn write_related_test_files(
    doc: &mut String,
    lang: &str,
    test_files: &[&str],
    config: &DocGeneratorConfig,
) {
    if test_files.is_empty() {
        return;
    }

    if lang == "zh-hans" {
        writeln!(doc, "\n**关联测试文件：**").unwrap();
    } else {
        writeln!(doc, "\n**Related Test Files:**").unwrap();
    }

    for file in test_files {
        let status = test_result_status(config.test_results, file);
        writeln!(doc, "- `{}`{}", file, status).unwrap();
    }
}

pub(super) fn write_details_block(doc: &mut String, lang: &str, xml_example: &str) {
    writeln!(doc, "\n---").unwrap();

    if lang == "zh-hans" {
        writeln!(doc, "\n<details>").unwrap();
        writeln!(doc, "<summary>技术细节与实现</summary>").unwrap();
        writeln!(doc, "\n### XML 示例\n").unwrap();
    } else {
        writeln!(doc, "\n<details>").unwrap();
        writeln!(doc, "<summary>Technical Details</summary>").unwrap();
        writeln!(doc, "\n### XML Example\n").unwrap();
    }

    writeln!(doc, "```xml\n{}\n```", xml_example).unwrap();
    writeln!(doc, "</details>").unwrap();
}

pub(super) fn write_index_header(doc: &mut String, lang: &str, zh_title: &str, en_title: &str) {
    let title = if lang == "zh-hans" {
        zh_title
    } else {
        en_title
    };
    writeln!(doc, "# {}\n", title).unwrap();
    if lang == "zh-hans" {
        writeln!(doc, "> ⚠️ **此文档由代码自动生成，请勿手动编辑。**").unwrap();
    } else {
        writeln!(
            doc,
            "> ⚠️ **This documentation is auto-generated. Do not edit manually.**"
        )
        .unwrap();
    }
}

pub(super) fn write_index_table_header(doc: &mut String, lang: &str, builtin: bool) {
    writeln!(doc).unwrap();
    match (lang, builtin) {
        ("zh-hans", false) => {
            writeln!(doc, "| 效果 | 支持状态 | 说明 |").unwrap();
            writeln!(doc, "|------|---------|------|").unwrap();
        }
        ("zh-hans", true) => {
            writeln!(doc, "| 功能 | 支持状态 | 说明 |").unwrap();
            writeln!(doc, "|------|---------|------|").unwrap();
        }
        (_, false) => {
            writeln!(doc, "| Effect | Status | Description |").unwrap();
            writeln!(doc, "|--------|--------|-------------|").unwrap();
        }
        (_, true) => {
            writeln!(doc, "| Feature | Status | Description |").unwrap();
            writeln!(doc, "|---------|--------|-------------|").unwrap();
        }
    }
}

pub(super) fn write_field_line_with_impl(
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

    let is_implemented = is_field_implemented(effect_id, field.name, config.impl_status);
    let final_support = match field.support_level {
        SupportLevel::Full => {
            if is_implemented {
                SupportLevel::Full
            } else {
                SupportLevel::Unsupported
            }
        }
        SupportLevel::Partial => SupportLevel::Partial,
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

pub(super) fn write_field_line(doc: &mut String, field: &FieldDef, lang: &str) {
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

pub(super) fn to_kebab_case(s: &str) -> String {
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

pub(super) fn truncate_desc(desc: &str, max_chars: usize) -> String {
    if desc.chars().count() <= max_chars {
        desc.to_string()
    } else {
        let truncated: String = desc.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

fn test_result_status<'a>(test_results: Option<&TestResults>, file: &str) -> &'a str {
    let Some(results) = test_results else {
        return "";
    };
    let Some(result) = results.get_result(file) else {
        return "";
    };
    if result.is_pass() {
        " ✅"
    } else if result.is_fail() {
        " ❌"
    } else {
        " ⏭️"
    }
}

fn is_field_implemented(
    effect_id: &str,
    field_name: &str,
    impl_status: Option<&HashMap<String, EffectImpl>>,
) -> bool {
    let Some(status) = impl_status else {
        return true;
    };
    let Some(impl_info) = status.get(effect_id) else {
        return false;
    };

    if impl_info
        .implemented_fields
        .contains(&field_name.to_string())
    {
        return true;
    }

    impl_info.pattern_fields.iter().any(|pattern| {
        pattern
            .strip_suffix('*')
            .is_some_and(|prefix| field_name.starts_with(prefix))
    })
}
