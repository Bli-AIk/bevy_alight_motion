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
    let effects_rs = Path::new("src/scene/effects.rs");

    println!("扫描源文件... / Scanning source file...");
    println!("路径 / Path: {}\n", effects_rs.display());

    match impl_scanner::scan_effects_rs(effects_rs) {
        Ok(results) => {
            impl_scanner::print_scan_results(&results);

            // 生成 JSON 输出 / Generate JSON output
            println!("=== JSON 输出 / JSON Output ===\n");
            let json = impl_scanner::generate_impl_json(&results);
            println!("{}", json);

            // 保存到文件 / Save to file
            let output_path = Path::new("impl_status.json");
            if let Err(e) = std::fs::write(output_path, &json) {
                eprintln!("❌ 保存失败 / Failed to save: {}", e);
            } else {
                println!("✅ 已保存到 / Saved to: {}", output_path.display());
            }
        }
        Err(e) => {
            eprintln!("❌ 扫描失败 / Scan failed: {}", e);
            std::process::exit(1);
        }
    }
}
