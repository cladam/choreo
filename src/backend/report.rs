use serde::{Serialize, Serializer};
use std::time::Duration;

// A custom serializer for converting Duration to fractional seconds for JUnit.
fn to_seconds_str<S>(d: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&format!("{:.3}", d.as_secs_f64()))
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Running,
}

#[derive(Debug, Serialize, Clone)]
pub struct TestCaseReport {
    pub name: String,
    #[serde(serialize_with = "to_seconds_str")]
    pub time: Duration,
    pub status: TestStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_message: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TestSuiteReport {
    pub name: String,
    pub tests: usize,
    pub failures: usize,
    #[serde(serialize_with = "to_seconds_str")]
    pub time: Duration,
    #[serde(rename = "testcase")]
    pub testcases: Vec<TestCaseReport>,
}
