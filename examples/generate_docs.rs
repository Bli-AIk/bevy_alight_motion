//! # generate_docs.rs
//!
//! # 文档生成示例
//!
//! Documentation generation example.
//! 文档生成示例。
//!
//! Usage / 使用方法:
//! ```bash
//! cargo run --example generate_docs
//! ```

use bevy_alight_motion::effects_registry::doc_generator;
use std::path::Path;

fn main() {
    let output_dir = Path::new("doc");

    println!("正在生成文档...");
    println!("Generating documentation...\n");

    match doc_generator::generate_all_docs(output_dir) {
        Ok(()) => {
            println!("✅ 文档生成成功！");
            println!("✅ Documentation generated successfully!\n");

            // 显示生成的文件列表
            println!("生成的文件 / Generated files:");
            println!("  doc/zh-hans/effects/_index.md");
            println!("  doc/zh-hans/effects/*.md");
            println!("  doc/zh-hans/builtins/_index.md");
            println!("  doc/zh-hans/builtins/*.md");
            println!("  doc/en/effects/_index.md");
            println!("  doc/en/effects/*.md");
            println!("  doc/en/builtins/_index.md");
            println!("  doc/en/builtins/*.md");

            // 显示统计信息
            let effects = bevy_alight_motion::effects_registry::all_effects();
            let builtins = bevy_alight_motion::effects_registry::all_builtins();
            let stats = doc_generator::get_support_stats(effects);

            println!("\n统计 / Statistics:");
            println!("  效果数量 / Effects: {}", effects.len());
            println!("    - 完全支持 / Full support: {}", stats.full_count);
            println!("    - 部分支持 / Partial support: {}", stats.partial_count);
            println!(
                "    - 不支持 / Unsupported: {}",
                stats.unsupported_count
            );
            println!("  内置功能数量 / Builtins: {}", builtins.len());
        }
        Err(e) => {
            eprintln!("❌ 文档生成失败 / Documentation generation failed: {}", e);
            std::process::exit(1);
        }
    }
}
