//! Generates per-effect and index Markdown pages from the built-in effect registry.
//! 根据内置效果注册表生成单效果页面和索引页 Markdown。
//!
//! The registry stores effect metadata in Rust definitions, but contributors and reviewers still
//! need browsable documentation. This file turns `EffectDef` and `BuiltinDef` entries into concrete
//! Markdown pages, including support level, field tables, related tests, and bilingual descriptions.
//! 效果注册表把元数据保存在 Rust 定义里，但贡献者和审查者仍然需要可浏览的文档。
//! 这个文件会把 `EffectDef` 与 `BuiltinDef` 条目转换成真正的 Markdown 页面，包含支持等级、
//! 字段表、关联测试和双语说明。

use std::fmt::Write;
use std::path::Path;

use crate::effects_registry::doc_generator::render::{
    to_kebab_case, truncate_desc, write_auto_generated_notice, write_details_block,
    write_field_line, write_field_line_with_impl, write_index_header, write_index_table_header,
    write_page_title, write_related_test_files, write_support_status, write_test_timestamp,
};
use crate::effects_registry::doc_generator::{
    DocGeneratorConfig, get_builtin_support_level, get_effect_support_level,
};

use super::super::types::{BuiltinDef, EffectDef};

/// 为单个效果生成 Markdown 文档 / Generate Markdown doc for a single effect
pub fn generate_effect_doc(effect: &EffectDef, lang: &str, config: &DocGeneratorConfig) -> String {
    let mut doc = String::new();
    let support_level = get_effect_support_level(effect, config);

    write_page_title(
        &mut doc,
        lang,
        effect.display_name_zh,
        effect.display_name_en,
    );
    write_auto_generated_notice(&mut doc, lang);
    write_test_timestamp(&mut doc, lang, config);

    let desc = if lang == "zh-hans" {
        effect.description_zh
    } else {
        effect.description_en
    };
    writeln!(doc, "\n{}\n", desc).unwrap();
    write_support_status(&mut doc, lang, support_level);

    for field in effect.fields {
        write_field_line_with_impl(&mut doc, field, lang, effect.id, config);
    }

    let test_files: Vec<&str> = if let Some(effect_test_files) = config.effect_test_files {
        if let Some(files) = effect_test_files.effect_test_map.get(effect.id) {
            files.iter().map(|s| s.as_str()).collect()
        } else {
            effect.test_files.to_vec()
        }
    } else {
        effect.test_files.to_vec()
    };

    write_related_test_files(&mut doc, lang, &test_files, config);
    write_details_block(&mut doc, lang, effect.xml_example);
    doc
}

/// 为单个内置功能生成 Markdown 文档 / Generate Markdown doc for a single builtin
pub fn generate_builtin_doc(
    builtin: &BuiltinDef,
    lang: &str,
    config: &DocGeneratorConfig,
) -> String {
    let mut doc = String::new();
    let support_level = get_builtin_support_level(builtin, config);

    write_page_title(
        &mut doc,
        lang,
        builtin.display_name_zh,
        builtin.display_name_en,
    );
    write_auto_generated_notice(&mut doc, lang);
    write_test_timestamp(&mut doc, lang, config);

    let desc = if lang == "zh-hans" {
        builtin.description_zh
    } else {
        builtin.description_en
    };
    writeln!(doc, "\n{}\n", desc).unwrap();
    write_support_status(&mut doc, lang, support_level);

    for field in builtin.fields {
        write_field_line(&mut doc, field, lang);
    }

    write_related_test_files(&mut doc, lang, builtin.test_files, config);
    write_details_block(&mut doc, lang, builtin.xml_example);
    doc
}

/// 为所有效果生成文档索引页 / Generate effects index page
pub fn generate_effects_index(
    effects: &[&EffectDef],
    lang: &str,
    config: &DocGeneratorConfig,
) -> String {
    let mut doc = String::new();

    write_index_header(&mut doc, lang, "效果列表", "Effects List");
    write_test_timestamp(&mut doc, lang, config);
    write_index_table_header(&mut doc, lang, false);

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

        writeln!(
            doc,
            "| [{}](./{}.md) | {} | {} |",
            name,
            to_kebab_case(effect.short_name),
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

    write_index_header(&mut doc, lang, "基础功能列表", "Builtins List");
    write_test_timestamp(&mut doc, lang, config);
    write_index_table_header(&mut doc, lang, true);

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

        writeln!(
            doc,
            "| [{}](./{}.md) | {} | {} |",
            name,
            to_kebab_case(builtin.short_name),
            support_level.icon(),
            truncate_desc(desc, 60)
        )
        .unwrap();
    }

    doc
}

/// 生成所有文档文件 / Generate all documentation files
///
/// 调用方式 / Usage: `cargo run --example generate_docs`
pub fn generate_all_docs(output_dir: &Path, config: &DocGeneratorConfig) -> std::io::Result<()> {
    use std::fs;

    let effects = super::super::effects::all();
    let builtins = super::super::builtin::all();

    write_lang_docs(output_dir, "zh-hans", effects, builtins, config)?;
    write_lang_docs(output_dir, "en", effects, builtins, config)?;

    let sidebar_config =
        super::super::vitepress::generate_vitepress_sidebar_snippet(effects, builtins, config);
    fs::write(
        output_dir.join(".vitepress/sidebar-effects.mts"),
        sidebar_config,
    )?;

    Ok(())
}

fn write_lang_docs(
    output_dir: &Path,
    lang: &str,
    effects: &[&EffectDef],
    builtins: &[&BuiltinDef],
    config: &DocGeneratorConfig,
) -> std::io::Result<()> {
    use std::fs;

    let effects_dir = output_dir.join(lang).join("effects");
    fs::create_dir_all(&effects_dir)?;
    for effect in effects {
        let content = generate_effect_doc(effect, lang, config);
        let path = effects_dir.join(format!("{}.md", to_kebab_case(effect.short_name)));
        fs::write(path, content)?;
    }
    fs::write(
        effects_dir.join("index.md"),
        generate_effects_index(effects, lang, config),
    )?;

    let builtins_dir = output_dir.join(lang).join("builtins");
    fs::create_dir_all(&builtins_dir)?;
    for builtin in builtins {
        let content = generate_builtin_doc(builtin, lang, config);
        let path = builtins_dir.join(format!("{}.md", to_kebab_case(builtin.short_name)));
        fs::write(path, content)?;
    }
    fs::write(
        builtins_dir.join("index.md"),
        generate_builtins_index(builtins, lang, config),
    )?;

    Ok(())
}
