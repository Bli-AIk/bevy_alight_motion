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
use std::path::Path;

fn main() {
    let effects_dir = Path::new("src/scene/effects");

    println!("扫描源文件... / Scanning source files...");
    println!("路径 / Path: {}", effects_dir.display());
    let all_results = match impl_scanner::scan_effects_dir(effects_dir) {
        Ok(results) => results,
        Err(e) => {
            eprintln!("❌ 扫描失败 / Scan failed: {}", e);
            return;
        }
    };

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
