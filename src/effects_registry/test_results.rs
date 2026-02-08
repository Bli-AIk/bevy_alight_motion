//! # test_results.rs
//!
//! # 测试结果解析模块
//!
//! Module for parsing test results from test_results.json.
//! 用于解析 test_results.json 中的测试结果。

use super::types::SupportLevel;
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 单个测试结果 / Single test result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestResult {
    /// 测试状态 / Test status: "pass", "fail", "skip", "cancelled"
    pub status: String,
    /// 平均相似度 / Average similarity (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_similarity: Option<f64>,
    /// 跳过原因 / Skip reason (if skipped)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl TestResult {
    /// 是否通过 / Whether the test passed
    pub fn is_pass(&self) -> bool {
        self.status == "pass"
    }

    /// 是否失败 / Whether the test failed
    pub fn is_fail(&self) -> bool {
        self.status == "fail"
    }

    /// 是否跳过 / Whether the test was skipped
    pub fn is_skip(&self) -> bool {
        self.status == "skip" || self.status == "cancelled"
    }
}

/// 测试汇总 / Test summary
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestSummary {
    pub passed: u32,
    pub skipped: u32,
    pub failed: u32,
}

/// 测试结果集合 / Test results collection
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestResults {
    /// 测试时间戳 / Test timestamp (ISO 8601 format)
    pub timestamp: String,
    /// 汇总信息 / Summary
    pub summary: TestSummary,
    /// 各测试结果 / Individual test results
    pub results: HashMap<String, TestResult>,
}

impl TestResults {
    /// 从 JSON 文件加载 / Load from JSON file
    pub fn load_from_file(path: &Path) -> Result<Self, TestResultsError> {
        let content = std::fs::read_to_string(path).map_err(TestResultsError::Io)?;
        serde_json::from_str(&content).map_err(TestResultsError::Parse)
    }

    /// 获取测试时间 / Get test timestamp as DateTime
    pub fn get_timestamp(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.timestamp)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }

    /// 检查测试数据是否过期（超过指定天数）/ Check if test data is stale
    pub fn is_stale(&self, max_days: i64) -> bool {
        if let Some(timestamp) = self.get_timestamp() {
            let now = Utc::now();
            let diff = now.signed_duration_since(timestamp);
            diff.num_days() > max_days
        } else {
            // 无法解析时间戳，视为过期 / Cannot parse timestamp, consider stale
            true
        }
    }

    /// 格式化时间戳为本地时间 / Format timestamp as local time
    pub fn format_timestamp_local(&self) -> String {
        if let Some(timestamp) = self.get_timestamp() {
            let local: DateTime<Local> = timestamp.into();
            local.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            self.timestamp.clone()
        }
    }

    /// 获取某个测试的结果 / Get result for a specific test
    pub fn get_result(&self, test_name: &str) -> Option<&TestResult> {
        // 尝试带 .amproj 后缀和不带后缀的查找
        // Try with and without .amproj suffix
        let name_without_ext = test_name.trim_end_matches(".amproj");
        self.results
            .get(name_without_ext)
            .or_else(|| self.results.get(test_name))
    }

    /// 根据关联的测试文件计算支持级别 / Compute support level based on associated test files
    pub fn compute_support_level(&self, test_files: &[&str]) -> SupportLevel {
        if test_files.is_empty() {
            return SupportLevel::Unsupported;
        }

        let mut pass_count = 0;
        let mut fail_count = 0;
        let mut _skip_count = 0;

        for test_file in test_files {
            if let Some(result) = self.get_result(test_file) {
                if result.is_pass() {
                    pass_count += 1;
                } else if result.is_fail() {
                    fail_count += 1;
                } else {
                    _skip_count += 1;
                }
            } else {
                // 未找到测试结果，视为未测试 / No result found, treat as not tested
                _skip_count += 1;
            }
        }

        let total = test_files.len();

        if pass_count == total {
            // 所有测试通过 / All tests passed
            SupportLevel::Full
        } else if pass_count > 0 {
            // 部分测试通过 / Some tests passed
            SupportLevel::Partial
        } else if fail_count > 0 {
            // 有失败的测试 / Has failed tests
            SupportLevel::Unsupported
        } else {
            // 全部跳过或未测试 / All skipped or not tested
            SupportLevel::Unsupported
        }
    }
}

/// 测试结果错误 / Test results error
#[derive(Debug)]
pub enum TestResultsError {
    Io(std::io::Error),
    Parse(serde_json::Error),
}

impl std::fmt::Display for TestResultsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestResultsError::Io(e) => write!(f, "IO error: {}", e),
            TestResultsError::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for TestResultsError {}

/// 默认测试结果文件路径 / Default test results file path
pub const DEFAULT_TEST_RESULTS_PATH: &str = "test_results.json";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_test_results() {
        let json = r#"{
            "timestamp": "2024-02-08T12:00:00+08:00",
            "summary": {
                "passed": 2,
                "skipped": 1,
                "failed": 1
            },
            "results": {
                "basic_shape": { "status": "pass", "avg_similarity": 0.99 },
                "fx_1_stretch_segment": { "status": "pass", "avg_similarity": 0.98 },
                "fx_2_gaussian_blur": { "status": "skip" },
                "fx_3_grid": { "status": "fail", "avg_similarity": 0.85 }
            }
        }"#;

        let results: TestResults = serde_json::from_str(json).unwrap();
        assert_eq!(results.summary.passed, 2);
        assert_eq!(results.summary.failed, 1);
        assert!(results.get_result("basic_shape").unwrap().is_pass());
        assert!(results.get_result("fx_2_gaussian_blur").unwrap().is_skip());
    }

    #[test]
    fn test_compute_support_level() {
        let json = r#"{
            "timestamp": "2024-02-08T12:00:00+08:00",
            "summary": { "passed": 2, "skipped": 1, "failed": 1 },
            "results": {
                "test_a": { "status": "pass" },
                "test_b": { "status": "pass" },
                "test_c": { "status": "skip" },
                "test_d": { "status": "fail" }
            }
        }"#;

        let results: TestResults = serde_json::from_str(json).unwrap();

        // All pass -> Full
        assert_eq!(
            results.compute_support_level(&["test_a", "test_b"]),
            SupportLevel::Full
        );

        // Mixed -> Partial
        assert_eq!(
            results.compute_support_level(&["test_a", "test_c"]),
            SupportLevel::Partial
        );

        // Has fail -> still Partial if some pass
        assert_eq!(
            results.compute_support_level(&["test_a", "test_d"]),
            SupportLevel::Partial
        );

        // All skip -> Unsupported
        assert_eq!(
            results.compute_support_level(&["test_c"]),
            SupportLevel::Unsupported
        );

        // All fail -> Unsupported
        assert_eq!(
            results.compute_support_level(&["test_d"]),
            SupportLevel::Unsupported
        );

        // Empty -> Unsupported
        assert_eq!(
            results.compute_support_level(&[]),
            SupportLevel::Unsupported
        );
    }
}
