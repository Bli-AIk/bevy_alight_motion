//! # generate_docs.rs
//!
//! # 文档生成示例
//!
//! Documentation generation example.
//! 文档生成示例。
//!
//! Usage / 使用方法:
//! ```bash
//! # 先运行测试生成 test_results.json
//! ./test_comparison.sh --all
//!
//! # 然后生成文档
//! cargo run --example generate_docs
//! ```

use bevy_alight_motion::effects_registry::{DEFAULT_TEST_RESULTS_PATH, TestResults, doc_generator};
use std::path::Path;

fn main() {
    let output_dir = Path::new("doc");
    let test_results_path = Path::new(DEFAULT_TEST_RESULTS_PATH);

    println!("正在生成文档...");
    println!("Generating documentation...\n");

    // 尝试加载测试结果
    let test_results = match TestResults::load_from_file(test_results_path) {
        Ok(results) => {
            println!(
                "✅ 已加载测试结果 / Test results loaded: {}",
                test_results_path.display()
            );
            println!(
                "   测试时间 / Test time: {}",
                results.format_timestamp_local()
            );
            println!(
                "   汇总 / Summary: {} passed, {} failed, {} skipped",
                results.summary.passed, results.summary.failed, results.summary.skipped
            );
            if results.is_stale(1) {
                println!("   ⚠️ 警告：测试数据已过期！/ Warning: Test data is stale!");
            }
            println!();
            Some(results)
        }
        Err(e) => {
            println!(
                "⚠️ 未找到测试结果文件 / Test results file not found: {}",
                test_results_path.display()
            );
            println!("   错误 / Error: {}", e);
            println!("   将使用静态支持级别 / Using static support levels instead.\n");
            None
        }
    };

    // 配置文档生成器
    let config = doc_generator::DocGeneratorConfig {
        test_results: test_results.as_ref(),
        stale_days: 1,
    };

    match doc_generator::generate_all_docs(output_dir, &config) {
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
            let stats = doc_generator::get_support_stats(effects, &config);

            println!("\n统计 / Statistics:");
            println!("  效果数量 / Effects: {}", effects.len());
            println!("    - 完全支持 / Full support: {}", stats.full_count);
            println!("    - 部分支持 / Partial support: {}", stats.partial_count);
            println!("    - 不支持 / Unsupported: {}", stats.unsupported_count);
            println!("  内置功能数量 / Builtins: {}", builtins.len());

            // 输出 VitePress 侧边栏配置
            println!("\n{}", "=".repeat(60));
            println!(
                "{}",
                doc_generator::generate_vitepress_sidebar_snippet(effects, builtins)
            );
        }
        Err(e) => {
            eprintln!("❌ 文档生成失败 / Documentation generation failed: {}", e);
            std::process::exit(1);
        }
    }
}
