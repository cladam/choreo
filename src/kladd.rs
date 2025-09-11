// src/main.rs
use choreo::backend::filesystem_backend::FileSystemBackend;
use choreo::backend::report::{
    AfterHook, Feature, Report, Result as StepResult, Scenario as ReportScenario, Step, Summary,
    TestCaseReport, TestStatus,
};
use choreo::backend::terminal_backend::TerminalBackend;
use choreo::cli;
use choreo::cli::{Cli, Commands};
use choreo::colours;
use choreo::error::AppError;
use choreo::parser::ast::{
    Action, GivenStep, ReportFormat, Statement, TestCase, TestState, TestSuiteSettings,
};
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
                        colours::info("Parsed test file successfully.");
                    }
                    suite
                }
                Err(e) => {
                    return Err(AppError::PestError(e));
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
            let mut settings = TestSuiteSettings::default();

            for s in &test_suite.statements {
                match s {
                    Statement::FeatureDef(name) => feature_name = name.clone(),
                    Statement::EnvDef(vars) => {
                        for var in vars {
                            if let Ok(val) = env::var(var) {
                                env_vars.insert(var.clone(), val);
                            }
                        }
                    }
                    Statement::VarsDef(vars) => {
                        for (key, value) in vars {
                            env_vars.insert(key.clone(), value.as_string());
                        }
                    }
                    Statement::Scenario(scenario) => scenarios.push(scenario.clone()),
                    Statement::SettingsDef(s) => settings = s.clone(),
                    _ => {}
                }
            }

            // --- Backend and State Initialisation ---
            let mut terminal_backend =
                TerminalBackend::new(base_dir.to_path_buf(), settings.shell_path.clone());
            let fs_backend = FileSystemBackend::new(base_dir.to_path_buf());
            let mut last_exit_code: Option<i32> = None;
            let mut output_buffer = String::new();
            let mut test_states: HashMap<String, TestState> = HashMap::new();
            let mut test_start_times: HashMap<String, Instant> = HashMap::new();

            for scenario in &scenarios {
                for test in &scenario.tests {
                    test_states.insert(test.name.clone(), TestState::Pending);
                }
            }

            let test_cases: Vec<TestCase> = scenarios
                .iter()
                .flat_map(|scenario| scenario.tests.clone())
                .collect();

            let mut test_reports: HashMap<String, TestCaseReport> = test_cases
                .iter()
                .map(|tc| {
                    (
                        tc.name.clone(),
                        TestCaseReport {
                            name: tc.name.clone(),
                            status: TestStatus::Pending,
                            duration: 0.0,
                            error_message: None,
                        },
                    )
                })
                .collect();

            // --- Main Test Execution ---
            let suite_start_time = Instant::now();
            let test_timeout = Duration::from_secs(settings.timeout_seconds); // Timeout per test

            'scenario_loop: for scenario in &scenarios {
                // Always print the current scenario
                colours::info(&format!("\nRunning scenario: '{}'", scenario.name));

                for test in &scenario.tests {
                    let test_start_time = Instant::now();
                    test_start_times.insert(test.name.clone(), test_start_time);
                    test_states.insert(test.name.clone(), TestState::Running);
                    colours::info(&format!("Starting choreo test: {}", test.description));

                    // --- GIVEN ---
                    for step in &test.given {
                        match step {
                            GivenStep::Action(action) => {
                                let substituted_action =
                                    substitute_variables_in_action(action, &env_vars);
                                execute_action(
                                    &substituted_action,
                                    &mut terminal_backend,
                                    &fs_backend,
                                    &mut last_exit_code,
                                );
                            }
                            GivenStep::Condition(condition) => {
                                // Wait until condition is met, with a timeout
                                let wait_start = Instant::now();
                                while !check_condition(
                                    condition,
                                    &test_states,
                                    &terminal_backend.get_output(),
                                    &terminal_backend.get_stderr(),
                                    wait_start.elapsed().as_secs_f32(),
                                    &mut env_vars,
                                    &last_exit_code,
                                    &fs_backend,
                                    verbose,
                                ) {
                                    if wait_start.elapsed() > test_timeout {
                                        let msg =
                                            format!("'given' condition timed out: {:?}", condition);
                                        test_states.insert(
                                            test.name.clone(),
                                            TestState::Failed(msg.clone()),
                                        );
                                        colours::error(&msg);
                                        break;
                                    }
                                    thread::sleep(Duration::from_millis(100));
                                }
                            }
                        }
                        if test_states.get(&test.name).unwrap().is_failed() {
                            break;
                        }
                    }

                    if test_states.get(&test.name).unwrap().is_failed() {
                        if settings.stop_on_failure {
                            break 'scenario_loop;
                        }
                        continue; // next test in scenario
                    }

                    // --- WHEN ---
                    for action in &test.when {
                        let substituted_action = substitute_variables_in_action(action, &env_vars);
                        execute_action(
                            &substituted_action,
                            &mut terminal_backend,
                            &fs_backend,
                            &mut last_exit_code,
                        );
                    }

                    // --- THEN ---
                    let then_start_time = Instant::now();
                    loop {
                        output_buffer = terminal_backend.get_output().to_string();
                        let all_then_met = check_all_conditions_met(
                            "then",
                            &test.then,
                            &test_states,
                            &output_buffer,
                            &terminal_backend.get_stderr(),
                            then_start_time.elapsed().as_secs_f32(),
                            &mut env_vars,
                            &last_exit_code,
                            &fs_backend,
                            verbose,
                        );

                        if all_then_met {
                            test_states.insert(test.name.clone(), TestState::Passed);
                            break;
                        }

                        if then_start_time.elapsed() > test_timeout {
                            let msg = format!("Test timed out after {}s", test_timeout.as_secs());
                            test_states.insert(test.name.clone(), TestState::Failed(msg));
                            break;
                        }

                        thread::sleep(Duration::from_millis(200)); // Polling interval
                    }

                    // After test completion, check for stop_on_failure
                    if settings.stop_on_failure && test_states.get(&test.name).unwrap().is_failed()
                    {
                        break 'scenario_loop;
                    }
                }

                // --- After Hooks (Cleanup) Execution ---
                for action in &scenario.after {
                    let substituted_action = substitute_variables_in_action(action, &env_vars);
                    if verbose {
                        colours::info(&format!(
                            "Running after hook: {}",
                            format_action_for_report(&substituted_action)
                        ));
                    }
                    execute_action(
                        &substituted_action,
                        &mut terminal_backend,
                        &fs_backend,
                        &mut last_exit_code,
                    );
                }
            }

            let suite_duration = suite_start_time.elapsed();

            // Update reports with final states and times
            for (name, state) in &test_states {
                if let Some(report) = test_reports.get_mut(name) {
                    report.status = state.clone().into();
                    if let Some(start_time) = test_start_times.get(name) {
                        report.duration = start_time.elapsed().as_secs_f32();
                    }
                    if let TestState::Failed(reason) = state {
                        report.error_message = Some(reason.clone());
                    }
                }
            }

            let final_reports: Vec<TestCaseReport> = test_cases
                .iter()
                .map(|tc| test_reports.get(&tc.name).unwrap().clone())
                .collect();

            if verbose {
                colours::info("\n--- Test Results ---");
            }

            for report in &final_reports {
                match &report.status {
                    TestStatus::Passed => colours::success(&format!(
                        "  ✓ {} (took {:.2}s)",
                        report.name, report.duration
                    )),
                    TestStatus::Failed => colours::error(&format!(
                        "  ✗ {} (failed after {:.2}s)",
                        report.name, report.duration
                    )),
                    _ => colours::warn(&format!("  - {} (skipped)", report.name)),
                }
            }

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

            generate_choreo_report(
                &suite_name,
                suite_start_time.elapsed(),
                &feature_name,
                &scenarios,
                &test_states,
                &test_start_times,
                &env_vars,
                &settings,
                verbose,
            )?;

            if verbose {
                colours::success("Reports generated successfully.");
            }
            Ok(())
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

fn generate_choreo_report(
    suite_name: &str,
    suite_duration: Duration,
    feature_name: &str,
    scenarios: &[choreo::parser::ast::Scenario],
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

        for (i, tc) in scenario.tests.iter().enumerate() {
            let mut keyword = "When";
            if i == 0 {
                keyword = "Given";
            }
            if i == scenario.tests.len() - 1 {
                keyword = "Then";
            }

            let (status, error_message) = match test_states.get(&tc.name) {
                Some(state) => (state.clone().into(), state.clone().get_error_message()),
                None => (TestStatus::Skipped, None),
            };

            let duration = test_start_times
                .get(&tc.name)
                .map(|start| start.elapsed().as_nanos() as u64)
                .unwrap_or(0);

            steps.push(Step {
                keyword: keyword.to_string(),
                name: tc.description.clone(),
                result: StepResult {
                    status,
                    duration,
                    error_message,
                },
            });
        }

        // After hooks
        for action in &scenario.after {
            // Substitute variables before formatting for the report
            let substituted_action = substitute_variables_in_action(action, env_vars);
            after_hooks.push(AfterHook {
                result: StepResult {
                    status: TestStatus::Passed, // After hooks don't have a fail state currently
                    duration: 0,
                    error_message: None,
                },
                action: format_action_for_report(&substituted_action),
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

/// Formats an Action enum into a human-readable string for reporting.
fn format_action_for_report(action: &Action) -> String {
    match action {
        Action::Type { actor, content } => format!("{} types '{}'", actor, content),
        Action::Press { actor, key } => format!("{} presses '{}'", actor, key),
        Action::Run { actor, command } => format!("{} runs '{}'", actor, command),
        Action::CreateFile { path, .. } => format!("FileSystem create_file '{}'", path),
        Action::DeleteFile { path } => format!("FileSystem delete_file '{}'", path),
        Action::CreateDir { path } => format!("FileSystem create_dir '{}'", path),
        Action::DeleteDir { path } => format!("FileSystem delete_dir '{}'", path),
    }
}
