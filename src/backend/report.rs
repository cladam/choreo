use serde::{Serialize, Serializer};
use std::time::Duration;

// A custom serializer for converting Duration to fractional seconds for JUnit.
fn to_seconds_str<S>(d: &Duration, s: S) -> core::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&format!("{:.3}", d.as_secs_f64()))
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum TestStatus {
    Pending,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report(pub Vec<Feature>);

#[derive(Debug, Serialize)]
pub struct Feature {
    pub uri: String,
    pub keyword: String,
    pub name: String,
    pub elements: Vec<Scenario>,
    pub summary: Summary,
}

#[derive(Debug, Serialize)]
pub struct Scenario {
    pub keyword: String,
    pub name: String,
    pub steps: Vec<Step>,
    pub after: Vec<AfterHook>,
}

#[derive(Debug, Serialize)]
pub struct Step {
    pub name: String,
    pub description: String,
    pub result: Result,
}

#[derive(Debug, Serialize)]
pub struct AfterHook {
    pub name: String,
    pub result: Result,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Result {
    pub status: String,
    pub duration_in_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub tests: usize,
    pub failures: usize,
    pub total_time_in_seconds: f32,
}
