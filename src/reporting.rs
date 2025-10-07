use crate::backend::report::{
    AfterHook, Feature, Report, Result as StepResult, Scenario as ReportScenario, Step, Summary,
};
use crate::colours;
use crate::error::AppError;
use crate::parser::ast::{Action, ReportFormat, Scenario, TestState, TestSuiteSettings};
use crate::parser::helpers::substitute_variables_in_action;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};

pub fn generate_choreo_report(
    suite_name: &str,
    suite_duration: Duration,
    feature_name: &str,
    scenarios: &[Scenario],
    test_states: &HashMap<String, TestState>,
    test_start_times: &HashMap<String, Instant>,
    env_vars: &HashMap<String, String>,
    settings: &TestSuiteSettings,
    verbose: bool,
) -> Result<(), AppError> {
    let mut report_scenarios = Vec::new();

    for scenario in scenarios {
        let mut steps = Vec::new();
        let mut after_hooks = Vec::new();

        for tc in scenario.tests.iter() {
            let (status, error_message) = match test_states.get(&tc.name) {
                Some(TestState::Passed) => ("passed".to_string(), None),
                Some(TestState::Failed(reason)) => ("failed".to_string(), Some(reason.clone())),
                _ => ("skipped".to_string(), None),
            };

            let duration = test_start_times
                .get(&tc.name)
                .map_or(Duration::default(), |s| s.elapsed());

            steps.push(Step {
                name: tc.name.clone(),
                description: tc.description.clone(),
                result: StepResult {
                    status,
                    duration_in_ms: duration.as_millis(),
                    error_message,
                },
            });
        }

        for action in &scenario.after {
            let substituted_action = substitute_variables_in_action(action, env_vars);
            after_hooks.push(AfterHook {
                name: format_action_for_report(&substituted_action),
                result: StepResult {
                    status: "passed".to_string(),
                    duration_in_ms: 0,
                    error_message: None,
                },
            });
        }

        report_scenarios.push(ReportScenario {
            keyword: "Scenario".to_string(),
            name: scenario.name.clone(),
            steps,
            after: after_hooks,
        });
    }

    if settings.report_format == ReportFormat::Junit {
        if verbose {
            colours::warn("JUnit report format is not yet supported. Skipping report generation.");
        }
        return Ok(());
    }

    let report = Report(vec![Feature {
        uri: suite_name.to_string(),
        keyword: "Feature".to_string(),
        name: feature_name.to_string(),
        elements: report_scenarios,
        summary: Summary {
            tests: test_states.len(),
            failures: test_states.values().filter(|s| s.is_failed()).count(),
            total_time_in_seconds: suite_duration.as_secs_f32(),
        },
    }]);

    let json = serde_json::to_string_pretty(&report)?;
    let date = chrono::Local::now().format("%Y%m%d_%H%M%S");
    fs::create_dir_all(&settings.report_path)?;
    let report_file_path = format!("{}choreo_test_report_{}.json", settings.report_path, date);
    let mut json_file = File::create(&report_file_path)?;
    json_file.write_all(json.as_bytes())?;

    if verbose {
        colours::info("JSON report content:");
        println!("{}", json);
    }

    Ok(())
}

fn format_action_for_report(action: &Action) -> String {
    match action {
        Action::Run { actor, command } => format!("{} runs '{}'", actor, command),
        Action::Pause { duration } => format!("duration of '{}'", duration),
        Action::Log { message } => format!("logs '{}'", message),
        Action::Timestamp { variable } => format!("timestamp at ({})", variable),
        Action::Uuid { variable } => format!("uuid of '{}'", variable),
        Action::CreateFile { path, .. } => format!("FileSystem create_file '{}'", path),
        Action::DeleteFile { path } => format!("FileSystem delete_file '{}'", path),
        Action::CreateDir { path } => format!("FileSystem create_dir '{}'", path),
        Action::DeleteDir { path } => format!("FileSystem delete_dir '{}'", path),
        Action::ReadFile { path, variable } => format!(
            "FileSystem read_file '{}' with variable: {:?}",
            path, variable
        ),
        Action::HttpGet { url, .. } => format!("HTTP GET '{}'", url),
        Action::HttpPost { url, .. } => format!("HTTP POST '{}'", url),
        Action::HttpPut { url, .. } => format!("HTTP PUT '{}'", url),
        Action::HttpPatch { url, .. } => format!("HTTP PATCH '{}'", url),
        Action::HttpDelete { url, .. } => format!("HTTP DELETE '{}'", url),
        Action::HttpSetHeader { key, value } => format!("HTTP set_header '{}: {}'", key, value),
        Action::HttpClearHeader { key } => format!("HTTP clear_header '{}'", key),
        Action::HttpClearHeaders => "HTTP clear_headers".to_string(),
        Action::HttpSetCookie { key, value } => format!("HTTP set_cookie '{}: {}'", key, value),
        Action::HttpClearCookie { key } => format!("HTTP clear_cookie '{}'", key),
        Action::HttpClearCookies => "HTTP clear_cookies".to_string(),
    }
}
