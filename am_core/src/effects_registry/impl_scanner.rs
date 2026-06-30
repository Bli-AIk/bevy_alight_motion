//! # impl_scanner.rs
//!
//! # 效果实现扫描器
//!
//! Effect implementation scanner - scans source code to extract implemented fields.
//! 效果实现扫描器 - 扫描源代码提取已实现的字段。
//!
//! This tool scans `src/scene/effects/**/*.rs` to automatically detect which effect
//! properties are actually implemented in code, without manual maintenance.
//!
//! Usage / 使用方法:
//! ```bash
//! cargo run --example scan_impl
//! ```

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
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

/// 效果与测试文件关联 / Effect to test file mapping
#[derive(Debug, Clone, Default)]
pub struct EffectTestFiles {
    /// 效果 ID 到测试文件列表的映射 / Effect ID to test files mapping
    pub effect_test_map: HashMap<String, Vec<String>>,
    /// 所有扫描的测试文件数量 / Total scanned test files
    pub total_files_scanned: usize,
}

/// 扫描 amproj 文件目录，提取每个效果关联的测试文件
/// Scan amproj directory to extract test files associated with each effect
///
/// 只扫描被 git 追踪的文件，遵守 .gitignore 规则
/// Only scans git-tracked files, respects .gitignore rules
pub fn scan_amproj_files(assets_dir: &Path) -> Result<EffectTestFiles, String> {
    let projects_dir = assets_dir.join("projects");
    if !projects_dir.exists() {
        return Err(format!("Directory not found: {}", projects_dir.display()));
    }

    let mut effect_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut total_files = 0;

    // 使用 git ls-files 获取被追踪的文件列表
    // Use git ls-files to get tracked files list
    let tracked_files = get_git_tracked_amproj_files(&projects_dir)?;

    for filename in tracked_files {
        let path = projects_dir.join(&filename);
        if !path.exists() {
            continue;
        }
        total_files += 1;

        // Extract effect IDs from amproj file
        let Ok(effects) = extract_effects_from_amproj(&path) else {
            continue;
        };
        for effect_id in effects {
            effect_map
                .entry(effect_id)
                .or_default()
                .insert(filename.clone());
        }
    }

    // 转换 HashSet 为 Vec 并排序 / Convert HashSet to Vec and sort
    let effect_test_map = effect_map
        .into_iter()
        .map(|(k, v)| {
            let mut files: Vec<String> = v.into_iter().collect();
            files.sort();
            (k, files)
        })
        .collect();

    Ok(EffectTestFiles {
        effect_test_map,
        total_files_scanned: total_files,
    })
}

/// 获取被 git 追踪的 amproj 文件列表
/// Get list of git-tracked amproj files
fn get_git_tracked_amproj_files(projects_dir: &Path) -> Result<Vec<String>, String> {
    use std::process::Command;

    // 将相对路径转换为绝对路径 / Convert relative path to absolute
    let projects_dir_abs = if projects_dir.is_absolute() {
        projects_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Failed to get current dir: {}", e))?
            .join(projects_dir)
    };

    // 获取仓库根目录 / Get repository root
    let repo_root = projects_dir_abs
        .ancestors()
        .find(|p| p.join(".git").exists())
        .ok_or_else(|| format!("Not in a git repository: {}", projects_dir_abs.display()))?;

    // 计算相对路径 / Calculate relative path
    let rel_path = projects_dir_abs
        .strip_prefix(repo_root)
        .map_err(|_| "Failed to calculate relative path")?;

    // 运行 git ls-files，使用 **/*.amproj 递归匹配 / Run git ls-files with **/*.amproj for recursive match
    let output = Command::new("/usr/bin/git")
        .args(["ls-files", &format!("{}/**/*.amproj", rel_path.display())])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("Failed to run git ls-files: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // 返回相对于 projects_dir 的路径（如 basic/shape/shape.amproj）
    // Return paths relative to projects_dir (e.g., basic/shape/shape.amproj)
    let prefix = format!("{}/", rel_path.display());
    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            // 移除前缀，保留相对路径 / Remove prefix, keep relative path
            if line.starts_with(&prefix) {
                Some(line[prefix.len()..].to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(files)
}

/// 从单个 amproj 文件中提取效果 ID 列表
/// Extract effect IDs from a single amproj file
fn extract_effects_from_amproj(amproj_path: &Path) -> Result<Vec<String>, String> {
    let file = fs::File::open(amproj_path)
        .map_err(|e| format!("Failed to open {}: {}", amproj_path.display(), e))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip {}: {}", amproj_path.display(), e))?;

    let mut effects = HashSet::new();

    // Find and read XML files
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {}", e))?;

        if !file.name().ends_with(".xml") {
            continue;
        }

        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| format!("Failed to read XML content: {}", e))?;

        // Extract effect IDs
        // Format: <effect id="com.alightcreative.effects.xxx"
        for line in content.lines() {
            let Some(start) = line.find("<effect id=\"") else {
                continue;
            };
            let rest = &line[start + 12..];
            let Some(end) = rest.find('"') else {
                continue;
            };
            effects.insert(rest[..end].to_string());
        }
    }

    Ok(effects.into_iter().collect())
}

/// 打印效果测试文件关联结果 / Print effect test files mapping results
pub fn print_effect_test_files(mapping: &EffectTestFiles) {
    println!("=== 效果测试文件关联 / Effect Test Files Mapping ===\n");
    println!(
        "共扫描 {} 个 amproj 文件 / Scanned {} amproj files\n",
        mapping.total_files_scanned, mapping.total_files_scanned
    );

    let mut sorted: Vec<_> = mapping.effect_test_map.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    for (effect_id, files) in sorted {
        println!(
            "📦 {} ({} 个文件 / {} files)",
            effect_id,
            files.len(),
            files.len()
        );
        for file in files {
            println!("   - {}", file);
        }
        println!();
    }
}

/// 扫描源文件提取效果实现信息 / Scan source file to extract effect implementations
pub fn scan_effects_rs(source_path: &Path) -> Result<HashMap<String, EffectImpl>, String> {
    let content = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read {}: {}", source_path.display(), e))?;

    let mut effects: HashMap<String, EffectImpl> = HashMap::new();
    let mut current_effect_id: Option<String> = None;
    let mut in_match_block = false;
    let mut brace_depth = 0;

    // First pass: collect const declarations for effect IDs
    // e.g. `const FADE_ID: &str = "com.alightcreative.effects.fade";`
    let mut const_map: HashMap<String, String> = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(id) = extract_const_effect_id(trimmed) {
            const_map.insert(id.0, id.1);
        }
    }

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1; // 1-based line numbers
        let trimmed = line.trim();

        // Detect effect ID
        // Patterns: `effect.id == "..."`, `effect.id != "..."`,
        //           `effect.id == CONST`, `effect.id != CONST`,
        //           `e.id == "..."` (in closures)
        if let Some(effect_id) = extract_effect_id_extended(trimmed, &const_map) {
            current_effect_id = Some(effect_id.clone());
            effects
                .entry(effect_id.clone())
                .or_insert_with(|| EffectImpl {
                    effect_id,
                    implemented_fields: Vec::new(),
                    pattern_fields: Vec::new(),
                    source_lines: Vec::new(),
                });
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

            // Detect pattern matches (e.g., name if name.starts_with("color"))
            if let Some(pattern_desc) = extract_starts_with_pattern(trimmed)
                && let Some(ref effect_id) = current_effect_id
                && let Some(effect) = effects.get_mut(effect_id)
                && !effect.pattern_fields.contains(&pattern_desc)
            {
                effect.pattern_fields.push(pattern_desc);
            }

            // Detect field names in match arms
            // Format: "fieldname" => { ... }
            // Skip non-field strings (e.g., "true", color values)
            if let Some(field_name) = extract_match_arm_field(trimmed)
                && let Some(ref effect_id) = current_effect_id
                && let Some(effect) = effects.get_mut(effect_id)
                && !effect.implemented_fields.contains(&field_name)
            {
                effect.implemented_fields.push(field_name);
                effect.source_lines.push(line_num);
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

/// 扫描效果实现目录 / Scan an effect implementation directory.
pub fn scan_effects_dir(source_dir: &Path) -> Result<HashMap<String, EffectImpl>, String> {
    if source_dir.is_file() {
        return scan_effects_rs(source_dir);
    }
    if !source_dir.is_dir() {
        return Err(format!("Directory not found: {}", source_dir.display()));
    }

    let mut files = Vec::new();
    collect_rs_files(source_dir, &mut files)?;
    files.sort();

    let mut effects = HashMap::new();
    for file in files {
        let file_effects = scan_effects_rs(&file)?;
        merge_effect_impls(&mut effects, file_effects);
    }

    Ok(effects)
}

fn collect_rs_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|e| format!("Failed to read {}: {}", dir.display(), e))?
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn merge_effect_impls(
    target: &mut HashMap<String, EffectImpl>,
    source: HashMap<String, EffectImpl>,
) {
    for (id, mut effect) in source {
        target
            .entry(id)
            .and_modify(|existing| {
                extend_unique_fields(&mut existing.implemented_fields, effect.implemented_fields.drain(..));
                extend_unique_fields(&mut existing.pattern_fields, effect.pattern_fields.drain(..));
                existing.source_lines.extend(effect.source_lines.iter().copied());
            })
            .or_insert(effect);
    }
}

fn extend_unique_fields(target: &mut Vec<String>, source: impl Iterator<Item = String>) {
    for field in source {
        if target.contains(&field) {
            continue;
        }
        target.push(field);
    }
}

/// Extract const effect ID declaration like `const FADE_ID: &str = "com.alightcreative...";`
/// Returns (constant_name, effect_id)
fn extract_const_effect_id(trimmed: &str) -> Option<(String, String)> {
    if !trimmed.starts_with("const ") || !trimmed.contains("com.alightcreative") {
        return None;
    }
    let name_end = trimmed.find(':')?;
    let const_name = trimmed[6..name_end].trim().to_string();
    let quote_start = trimmed.find('"')?;
    let rest = &trimmed[quote_start + 1..];
    let quote_end = rest.find('"')?;
    Some((const_name, rest[..quote_end].to_string()))
}

/// Extended effect ID extractor that handles multiple patterns:
/// - `effect.id == "..."` / `effect.id != "..."`
/// - `effect.id == CONST` / `effect.id != CONST`
/// - `e.id == "..."` (in closures like `.filter(|e| e.id == "...")`)
fn extract_effect_id_extended(
    trimmed: &str,
    const_map: &HashMap<String, String>,
) -> Option<String> {
    // Try direct string literal patterns (== and !=)
    for pattern in &[".id == \"", ".id != \""] {
        let id =
            extract_quoted_after(trimmed, pattern).filter(|s| s.starts_with("com.alightcreative"));
        if id.is_some() {
            return id;
        }
    }
    // Try constant reference patterns (== CONST and != CONST)
    for op in &[".id == ", ".id != "] {
        if let Some(start) = trimmed.find(op) {
            let rest = trimmed[start + op.len()..].trim();
            let ident: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if let Some(resolved) = const_map.get(&ident) {
                return Some(resolved.clone());
            }
        }
    }
    None
}

/// Extract a quoted string after a pattern marker
fn extract_quoted_after(text: &str, pattern: &str) -> Option<String> {
    let start = text.find(pattern)?;
    let rest = &text[start + pattern.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract pattern from a `starts_with("...")` call, returning formatted `pattern*`
fn extract_starts_with_pattern(trimmed: &str) -> Option<String> {
    let start = trimmed.find("starts_with(\"")?;
    let rest = &trimmed[start + 13..];
    let end = rest.find('"')?;
    Some(format!("{}*", &rest[..end]))
}

/// Extract a valid field name from a match arm line like `"fieldname" => { ... }`
fn extract_match_arm_field(trimmed: &str) -> Option<String> {
    if !trimmed.contains("=>") || trimmed.contains("if ") {
        return None;
    }
    let start = trimmed.find('"')?;
    let end = trimmed[start + 1..].find('"')?;
    let field_name = trimmed[start + 1..start + 1 + end].to_string();
    is_valid_field_name(&field_name).then_some(field_name)
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
        let effects_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("src/scene/effects");
        let all_results = scan_effects_dir(&effects_dir).unwrap();
        assert!(!all_results.is_empty());

        // 验证 transform2 效果 / Verify transform2 effect
        if let Some(transform2) = all_results.get("com.alightcreative.effects.transform2") {
            assert!(transform2.implemented_fields.contains(&"posx".to_string()));
            assert!(transform2.implemented_fields.contains(&"posy".to_string()));
        }
    }
}
