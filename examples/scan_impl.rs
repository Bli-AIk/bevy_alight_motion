//! # scan_impl.rs
//!
//! # 扫描效果实现
//!
//! Scan effect implementations from source code.
//! 从源代码扫描效果实现。
//!
//! Usage / 使用方法:
//! ```bash
//! cargo run --example scan_impl
//! ```

use bevy_alight_motion::effects_registry::impl_scanner;
use std::collections::HashMap;
use std::path::Path;

fn main() {
    // Scan all effect submodules, not just the re-export file
    // 扫描所有效果子模块，而非仅重导出文件
    let effect_files = [
        "src/scene/effects/common.rs",
        "src/scene/effects/extended.rs",
        "src/scene/effects/other.rs",
        "src/scene/effects/repeat.rs",
    ];

    println!("扫描源文件... / Scanning source files...");

    let mut all_results = HashMap::new();
    for file in &effect_files {
        let path = Path::new(file);
        println!("路径 / Path: {}", path.display());
        match impl_scanner::scan_effects_rs(path) {
            Ok(results) => merge_results(&mut all_results, results),
            Err(e) => {
                eprintln!("❌ 扫描 {} 失败 / Scan failed: {}", file, e);
            }
        }
    }

    println!();
    impl_scanner::print_scan_results(&all_results);

    // 生成 JSON 输出 / Generate JSON output
    println!("=== JSON 输出 / JSON Output ===\n");
    let json = impl_scanner::generate_impl_json(&all_results);
    println!("{}", json);

    // 保存到文件 / Save to file
    let output_path = Path::new("impl_status.json");
    if let Err(e) = std::fs::write(output_path, &json) {
        eprintln!("❌ 保存失败 / Failed to save: {}", e);
    } else {
        println!("✅ 已保存到 / Saved to: {}", output_path.display());
    }
}

fn merge_results(
    target: &mut HashMap<String, impl_scanner::EffectImpl>,
    source: HashMap<String, impl_scanner::EffectImpl>,
) {
    for (id, effect) in source {
        target
            .entry(id)
            .and_modify(|existing| {
                existing
                    .implemented_fields
                    .extend(effect.implemented_fields.clone());
                existing
                    .pattern_fields
                    .extend(effect.pattern_fields.clone());
            })
            .or_insert(effect);
    }
}
