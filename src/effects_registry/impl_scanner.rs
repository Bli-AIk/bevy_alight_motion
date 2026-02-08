//! # impl_scanner.rs
//!
//! # 效果实现扫描器
//!
//! Effect implementation scanner - scans source code to extract implemented fields.
//! 效果实现扫描器 - 扫描源代码提取已实现的字段。
//!
//! This tool scans `src/scene/effects.rs` to automatically detect which effect
//! properties are actually implemented in code, without manual maintenance.
//!
//! Usage / 使用方法:
//! ```bash
//! cargo run --example scan_impl
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 效果实现信息 / Effect implementation info
#[derive(Debug, Clone, Default)]
pub struct EffectImpl {
    /// 效果 ID / Effect ID (e.g., "com.alightcreative.effects.transform2")
    pub effect_id: String,
    /// 已实现的字段列表 / List of implemented fields
    pub implemented_fields: Vec<String>,
    /// 模式匹配的字段（如 color1-8）/ Pattern matched fields (e.g., color1-8)
    pub pattern_fields: Vec<String>,
    /// 从代码中提取的行号 / Line numbers from source
    pub source_lines: Vec<usize>,
}

/// 扫描源文件提取效果实现信息 / Scan source file to extract effect implementations
pub fn scan_effects_rs(source_path: &Path) -> Result<HashMap<String, EffectImpl>, String> {
    let content = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read {}: {}", source_path.display(), e))?;

    let mut effects: HashMap<String, EffectImpl> = HashMap::new();
    let mut current_effect_id: Option<String> = None;
    let mut in_match_block = false;
    let mut brace_depth = 0;

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1; // 1-based line numbers
        let trimmed = line.trim();

        // 检测效果 ID / Detect effect ID
        // 格式: if effect.id == "com.alightcreative.effects.xxx"
        if let Some(start) = trimmed.find("effect.id == \"") {
            let rest = &trimmed[start + 14..];
            if let Some(end) = rest.find('"') {
                let effect_id = rest[..end].to_string();
                current_effect_id = Some(effect_id.clone());

                if !effects.contains_key(&effect_id) {
                    effects.insert(
                        effect_id.clone(),
                        EffectImpl {
                            effect_id,
                            implemented_fields: Vec::new(),
                            pattern_fields: Vec::new(),
                            source_lines: Vec::new(),
                        },
                    );
                }
            }
        }

        // 检测 match prop.name.as_str() 块 / Detect match block
        if trimmed.contains("match prop.name.as_str()") {
            in_match_block = true;
            brace_depth = 0;
        }

        // 跟踪大括号深度 / Track brace depth
        if in_match_block {
            brace_depth += trimmed.matches('{').count() as i32;
            brace_depth -= trimmed.matches('}').count() as i32;

            // 检测模式匹配（如 name if name.starts_with("color")）
            // Detect pattern matches (e.g., name if name.starts_with("color"))
            if trimmed.contains("starts_with(\"") {
                if let Some(start) = trimmed.find("starts_with(\"") {
                    let rest = &trimmed[start + 13..];
                    if let Some(end) = rest.find('"') {
                        let pattern = rest[..end].to_string();
                        if let Some(ref effect_id) = current_effect_id {
                            if let Some(effect) = effects.get_mut(effect_id) {
                                let pattern_desc = format!("{}*", pattern);
                                if !effect.pattern_fields.contains(&pattern_desc) {
                                    effect.pattern_fields.push(pattern_desc);
                                }
                            }
                        }
                    }
                }
            }

            // 检测 match 分支中的字段名 / Detect field names in match arms
            // 格式: "fieldname" => { ... }
            // 但跳过非字段名的字符串（如 "true", 颜色值等）
            if trimmed.contains("=>") && !trimmed.contains("if ") {
                if let Some(start) = trimmed.find('"') {
                    if let Some(end) = trimmed[start + 1..].find('"') {
                        let field_name = trimmed[start + 1..start + 1 + end].to_string();

                        // 验证是否为有效的字段名 / Validate if it's a valid field name
                        // 有效字段名：小写字母开头，只包含字母、数字和下划线
                        if is_valid_field_name(&field_name) {
                            if let Some(ref effect_id) = current_effect_id {
                                if let Some(effect) = effects.get_mut(effect_id) {
                                    if !effect.implemented_fields.contains(&field_name) {
                                        effect.implemented_fields.push(field_name);
                                        effect.source_lines.push(line_num);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 退出 match 块 / Exit match block
            if brace_depth <= 0 {
                in_match_block = false;
            }
        }

        // 退出当前效果 / Exit current effect (when encountering next effect or function end)
        if current_effect_id.is_some()
            && (trimmed.starts_with("pub fn ")
                || trimmed.starts_with("pub(crate) fn ")
                || (trimmed.starts_with("fn ") && !trimmed.contains("(")))
        {
            current_effect_id = None;
        }
    }

    Ok(effects)
}

/// 检查是否为有效的字段名 / Check if it's a valid field name
fn is_valid_field_name(name: &str) -> bool {
    if name.is_empty() || name == "_" {
        return false;
    }

    // 排除常见的非字段名字符串 / Exclude common non-field strings
    let excluded = ["true", "false", "none", "some", "ok", "err"];
    if excluded.contains(&name.to_lowercase().as_str()) {
        return false;
    }

    // 排除包含特殊字符的字符串 / Exclude strings with special characters
    if name.contains(',') || name.contains('{') || name.contains('}') || name.contains('#') {
        return false;
    }

    // 有效字段名应该以小写字母开头 / Valid field names should start with lowercase letter
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_lowercase() {
        return false;
    }

    // 只包含字母、数字和下划线 / Only contains letters, digits, and underscores
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// 打印扫描结果 / Print scan results
pub fn print_scan_results(effects: &HashMap<String, EffectImpl>) {
    println!("=== 效果实现扫描结果 / Effect Implementation Scan Results ===\n");

    let mut sorted: Vec<_> = effects.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    for (effect_id, impl_info) in sorted {
        println!("📦 {}", effect_id);
        println!("   已实现字段 / Implemented fields:");
        for (i, field) in impl_info.implemented_fields.iter().enumerate() {
            let line = impl_info.source_lines.get(i).unwrap_or(&0);
            println!("     ✅ {} (line {})", field, line);
        }
        if !impl_info.pattern_fields.is_empty() {
            println!("   模式匹配 / Pattern matched:");
            for pattern in &impl_info.pattern_fields {
                println!("     🔤 {}", pattern);
            }
        }
        println!();
    }
}

/// 生成实现状态 JSON / Generate implementation status JSON
pub fn generate_impl_json(effects: &HashMap<String, EffectImpl>) -> String {
    let mut json = String::from("{\n");

    let mut sorted: Vec<_> = effects.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    for (i, (effect_id, impl_info)) in sorted.iter().enumerate() {
        json.push_str(&format!("  \"{}\": {{\n", effect_id));
        json.push_str("    \"implemented_fields\": [");

        let fields: Vec<String> = impl_info
            .implemented_fields
            .iter()
            .map(|f| format!("\"{}\"", f))
            .collect();
        json.push_str(&fields.join(", "));

        json.push_str("],\n");

        // 添加模式匹配字段 / Add pattern matched fields
        json.push_str("    \"pattern_fields\": [");
        let patterns: Vec<String> = impl_info
            .pattern_fields
            .iter()
            .map(|f| format!("\"{}\"", f))
            .collect();
        json.push_str(&patterns.join(", "));
        json.push_str("]\n");

        json.push_str("  }");
        if i < sorted.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("}\n");
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_field_name() {
        assert!(is_valid_field_name("posx"));
        assert!(is_valid_field_name("color1"));
        assert!(is_valid_field_name("lock_luminance"));
        assert!(!is_valid_field_name("true"));
        assert!(!is_valid_field_name("_"));
        assert!(!is_valid_field_name("{},{},{}"));
        assert!(!is_valid_field_name("#ff0000"));
    }

    #[test]
    fn test_scan_effects() {
        let path = Path::new("src/scene/effects.rs");
        if path.exists() {
            let results = scan_effects_rs(path).unwrap();
            assert!(!results.is_empty());

            // 验证 transform2 效果 / Verify transform2 effect
            if let Some(transform2) = results.get("com.alightcreative.effects.transform2") {
                assert!(transform2.implemented_fields.contains(&"posx".to_string()));
                assert!(transform2.implemented_fields.contains(&"posy".to_string()));
            }
        }
    }
}
