use choreo::backend::filesystem_backend::FileSystemBackend;
use choreo::backend::report::{
    AfterHook, Feature, Report, Result as StepResult, Scenario, Step, Summary, TestCaseReport,
    TestStatus,
};
use choreo::backend::terminal_backend::TerminalBackend;
use choreo::cli;
use choreo::cli::{Cli, Commands};
use choreo::colours;
use choreo::error::AppError;
use choreo::parser::ast::{Action, GivenStep, Statement, TestCase, TestState};
use choreo::parser::helpers::*;
use choreo::parser::parser;
use clap::Parser;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};
use std::{env, fs, thread};

fn main() {
    let cli = cli::Cli::parse();
    if let Err(e) = run(cli) {
        colours::error(&format!("Error: {}", e));
        std::process::exit(1);
    }

    // The main logic function, which takes the parsed CLI commands
    pub fn run(cli: Cli) -> Result<(), AppError> {
        match cli.command {
            Commands::Run { file, verbose } => {
                let suite_name = file.clone();

                if verbose {
                    colours::info(&format!("Starting Choreo Test Runner: {}", file));
                }

                let source = fs::read_to_string(&file)?;
                let test_suite = match parser::parse(&source) {
                    Ok(suite) => {
                        if verbose {
                            colours::info("Test suite parsed successfully.");
                        }
                        suite
                    }
                    Err(e) => {
                        if verbose {
                            colours::error("Failed to parse test suite.");
                        }
                        return Err(AppError::ParseError(e.to_string()));
                    }
                };

                let test_file_path = std::path::Path::new(&file);
                let base_dir = test_file_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new(""));

                // Find the FeatureDef statement to get the feature name.
                let mut feature_name = "Choreo Test Feature".to_string(); // Default name
                let mut env_vars: HashMap<String, String> = HashMap::new();
                let mut scenarios: Vec<choreo::parser::ast::Scenario> = Vec::new();

                for s in &test_suite.statements {
                    match s {
                        Statement::EnvDef(vars) => {
                            for var_name in vars {
                                let value = env::var(var_name)
                                    .map_err(|_| AppError::EnvVarNotFound(var_name.clone()))?;
                                env_vars.insert(var_name.clone(), value);
                            }
                        }
                        Statement::FeatureDef(name) => feature_name = name.clone(),
                        Statement::Scenario(scenario) => scenarios.push(scenario.clone()),
                        _ => {} // Ignore other statement types
                    }
                }

                // --- Backend and State Initialisation ---
                let mut terminal_backend = TerminalBackend::new(base_dir.to_path_buf());
                let fs_backend = FileSystemBackend::new(base_dir.to_path_buf());
                let output_buffer = String::new();
                let mut last_exit_code: Option<i32> = None;
                let test_states: HashMap<String, TestState> = HashMap::new();
                let test_start_times: HashMap<String, Instant> = HashMap::new();
                let mut output_buffer = String::new();
                let mut test_states: HashMap<String, TestState> = HashMap::new();
                let mut test_start_times: HashMap<String, Instant> = HashMap::new();

                for scenario in &scenarios {
                    for test in &scenario.tests {
                        test_states.insert(test.name.clone(), TestState::Pending);
                    }
                }

                let test_cases: Vec<TestCase> = test_suite
                    .statements
                    .into_iter()
                    .filter_map(|s| match s {
                        Statement::TestCase(tc) => Some(tc),
                        _ => None,
                    })
                    .collect();

                let test_reports: HashMap<String, TestCaseReport> = test_cases
                    .iter()
                    .map(|tc| {
                        (
                            tc.name.clone(),
                            TestCaseReport {
                                name: tc.description.clone(),
                                time: Duration::default(),
                                status: TestStatus::Skipped,
                                failure_message: None,
                            },
                        )
                    })
                    .collect();

                // --- Main Test Loop ---
                let suite_start_time = Instant::now();
                let test_timeout = Duration::from_secs(30);
                loop {
                    terminal_backend.read_output(&mut output_buffer, &mut last_exit_code);
                    let elapsed = suite_start_time.elapsed();

                    let mut tests_to_start = Vec::new();
                    let mut tests_to_pass = Vec::new();

                    // --- Checking Phase (Immutable Borrows) ---
                    for scenario in &scenarios {
                        for test_case in &scenario.tests {
                            let current_state = test_states.get(&test_case.name).unwrap();

                            if *current_state == TestState::Pending {
                                // Separate actions from conditions in the given block
                                let given_actions: Vec<_> = test_case
                                    .given
                                    .iter()
                                    .filter_map(|s| match s {
                                        GivenStep::Action(a) => Some(a.clone()),
                                        _ => None,
                                    })
                                    .collect();
                                let given_conditions: Vec<_> = test_case
                                    .given
                                    .iter()
                                    .filter_map(|s| match s {
                                        GivenStep::Condition(c) => Some(c.clone()),
                                        _ => None,
                                    })
                                    .collect();

                                // Check if all pre-conditions are met
                                if check_all_conditions_met(
                                    "given",
                                    &given_conditions,
                                    &test_states,
                                    &output_buffer,
                                    elapsed.as_secs_f32(),
                                    &mut env_vars,
                                    &last_exit_code,
                                    &fs_backend,
                                    verbose,
                                ) {
                                    // If conditions are met, this test is ready to start.
                                    // We pass along its setup actions.
                                    tests_to_start.push((test_case.name.clone(), given_actions));
                                }
                            } else if *current_state == TestState::Running {
                                if check_all_conditions_met(
                                    "then",
                                    &test_case.then,
                                    &test_states,
                                    &output_buffer,
                                    elapsed.as_secs_f32(),
                                    &mut env_vars,
                                    &last_exit_code,
                                    &fs_backend,
                                    verbose,
                                ) {
                                    tests_to_pass.push(test_case.name.clone());
                                }
                            }
                        }
                    }
                    // --- Updating Phase (Mutable Borrows) ---
                    for (name, setup_actions) in tests_to_start {
                        if let Some(state) = test_states.get_mut(&name) {
                            if *state == TestState::Pending {
                                let test_case = scenarios
                                    .iter()
                                    .flat_map(|s| &s.tests)
                                    .find(|t| t.name == name)
                                    .unwrap();
                                if verbose {
                                    colours::info(&format!(
                                        "▶️ Starting test: {}",
                                        test_case.description
                                    ));
                                }
                                test_start_times.insert(name.clone(), Instant::now());

                                // Execute setup actions from the 'given' block first.
                                for action in setup_actions {
                                    let substituted_a =
                                        substitute_variables_in_action(&action, &env_vars);
                                    execute_action(
                                        &substituted_a,
                                        &mut terminal_backend,
                                        &fs_backend,
                                        &mut last_exit_code,
                                    );
                                }

                                // Execute the main actions from the 'when' block
                                for action in &test_case.when {
                                    let substituted_a =
                                        substitute_variables_in_action(action, &env_vars);
                                    execute_action(
                                        &substituted_a,
                                        &mut terminal_backend,
                                        &fs_backend,
                                        &mut last_exit_code,
                                    );
                                }
                                *state = TestState::Running;
                            }
                        }
                    }

                    for name in tests_to_pass {
                        if let Some(state) = test_states.get_mut(&name) {
                            if *state == TestState::Running {
                                let test_case = scenarios
                                    .iter()
                                    .flat_map(|s| &s.tests)
                                    .find(|t| t.name == name)
                                    .unwrap();
                                if verbose {
                                    colours::success(&format!(
                                        "✅ Test Passed: {}",
                                        test_case.description
                                    ));
                                }
                                *state = TestState::Passed;
                            }
                        }
                    }

                    // --- Check for Completion ---
                    if test_states.values().all(|s| s.is_done()) {
                        if verbose {
                            println!("\n🎉 All tests completed!");
                        }
                        break;
                    }

                    if elapsed > test_timeout {
                        if verbose {
                            eprintln!("\n⏰ Test run timed out!");
                        }
                        for (_name, state) in test_states.iter_mut() {
                            if matches!(*state, TestState::Pending | TestState::Running) {
                                *state = TestState::Failed(format!(
                                    "Test timed out after {}s",
                                    test_timeout.as_secs()
                                ));
                            }
                        }
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                }

                let suite_duration = suite_start_time.elapsed();
                let final_reports: Vec<TestCaseReport> = test_cases
                    .iter()
                    .map(|tc| test_reports.get(&tc.name).unwrap().clone())
                    .collect();

                if verbose {
                    colours::info(&format!(
                        "Test suite '{}' completed in {:.2}s",
                        suite_name,
                        suite_duration.as_secs_f32()
                    ));
                }
                for report in &final_reports {
                    match &report.status {
                        TestStatus::Passed => {
                            if verbose {
                                colours::success(&format!(
                                    "Test '{}' passed in {:.2}s",
                                    report.name,
                                    report.time.as_secs_f32()
                                ));
                            } else {
                                println!(
                                    "✅ PASSED: {} ({:.2}s)",
                                    report.name,
                                    report.time.as_secs_f32()
                                )
                            }
                        }
                        TestStatus::Failed => {
                            if verbose {
                                colours::error(&format!(
                                    "Test '{}' failed in {:.2}s - {}",
                                    report.name,
                                    report.time.as_secs_f32(),
                                    report.failure_message.as_deref().unwrap_or("")
                                ));
                            } else {
                                println!(
                                    "❌ FAILED: {} ({:.2}s) - {}",
                                    report.name,
                                    report.time.as_secs_f32(),
                                    report.failure_message.as_deref().unwrap_or("")
                                )
                            }
                        }
                        TestStatus::Skipped => {
                            if verbose {
                                colours::warn(&format!("Test '{}' was skipped.", report.name));
                            } else {
                                println!("❔ SKIPPED: {}", report.name)
                            }
                        }
                        TestStatus::Running => {
                            if verbose {
                                colours::warn(&format!("Test '{}' is still running.", report.name));
                            } else {
                                println!("❔ HUNG: {}", report.name)
                            }
                        }
                    }
                }
                if verbose {
                    colours::info(&format!(
                        "Test suite '{}' summary: {} tests, {} failures, total time {:.2}s",
                        suite_name,
                        final_reports.len(),
                        final_reports
                            .iter()
                            .filter(|r| r.status == TestStatus::Failed)
                            .count(),
                        suite_duration.as_secs_f32()
                    ));
                }

                generate_better_report(
                    &suite_name,
                    suite_start_time.elapsed(),
                    &feature_name,
                    &scenarios,
                    &test_states,
                    &test_start_times,
                    verbose,
                )?;

                if verbose {
                    colours::success("Reports generated successfully.");
                }
                Ok(())
            }
        }
    }
}

/// Dispatches an action to the correct backend.
fn execute_action(
    action: &Action,
    terminal: &mut TerminalBackend,
    fs: &FileSystemBackend,
    last_exit_code: &mut Option<i32>,
) {
    // Check if it's a terminal action
    if terminal.execute_action(action, last_exit_code) {
        return;
    }
    // Check if it's a filesystem action
    if fs.execute_action(action) {
        return;
    }
}

fn generate_better_report(
    suite_name: &str,
    suite_duration: Duration,
    feature_name: &str,
    scenarios: &[choreo::parser::ast::Scenario],
    test_states: &HashMap<String, TestState>,
    test_start_times: &HashMap<String, Instant>,
    verbose: bool,
) -> Result<(), AppError> {
    let mut report_scenarios = Vec::new();

    for scenario in scenarios {
        let mut steps = Vec::new();
        let mut after_hooks = Vec::new();

        let (cleanup_tests, main_tests): (Vec<_>, Vec<_>) = scenario
            .tests
            .iter()
            .partition(|tc| tc.description.to_lowercase().contains("cleanup"));

        for (i, tc) in main_tests.iter().enumerate() {
            let mut keyword = "When";
            if i == 0 {
                keyword = "Given";
            }
            if i == main_tests.len() - 1 {
                keyword = "Then";
            }

            let (status, error_message) = match test_states.get(&tc.name) {
                Some(TestState::Passed) => ("passed".to_string(), None),
                Some(TestState::Failed(reason)) => ("failed".to_string(), Some(reason.clone())),
                _ => ("skipped".to_string(), None),
            };

            let duration = test_start_times
                .get(&tc.name)
                .map_or(Duration::default(), |s| s.elapsed());

            steps.push(Step {
                keyword: format!("{} ", keyword),
                name: tc.description.clone(),
                result: StepResult {
                    status,
                    duration_in_ms: duration.as_millis(),
                    error_message,
                },
            });
        }

        for tc in cleanup_tests {
            let (status, error_message) = match test_states.get(&tc.name) {
                Some(TestState::Passed) => ("passed".to_string(), None),
                Some(TestState::Failed(reason)) => ("failed".to_string(), Some(reason.clone())),
                _ => ("skipped".to_string(), None),
            };
            let duration = test_start_times
                .get(&tc.name)
                .map_or(Duration::default(), |s| s.elapsed());

            after_hooks.push(AfterHook {
                name: tc.description.clone(),
                result: StepResult {
                    status,
                    duration_in_ms: duration.as_millis(),
                    error_message,
                },
            });
        }

        report_scenarios.push(Scenario {
            keyword: "Scenario".to_string(),
            name: scenario.name.clone(),
            steps,
            after: after_hooks,
        });
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
    let mut json_file = File::create("better_report.json")?;
    json_file.write_all(json.as_bytes())?;

    if verbose {
        colours::info("Better JSON report content:");
        println!("{}", json);
    }

    Ok(())
}
