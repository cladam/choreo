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
    Action, Condition, GivenStep, ReportFormat, Statement, TestCase, TestState, TestSuiteSettings,
};
use choreo::parser::helpers::*;
use choreo::parser::parser;
use clap::Parser;
use itertools::Itertools;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};
use std::{env, fs};

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
                        colours::success("Test suite parsed successfully.");
                    }
                    suite
                }
                Err(e) => {
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
            let mut settings = TestSuiteSettings::default();

            for s in &test_suite.statements {
                match s {
                    Statement::SettingsDef(s) => settings = s.clone(),
                    Statement::BackgroundDef(steps) => {
                        // Convert background steps to a scenario
                        let bg_scenario = choreo::parser::ast::Scenario {
                            name: "Background".to_string(),
                            tests: vec![choreo::parser::ast::TestCase {
                                name: "Background Setup".to_string(),
                                description: "Setup steps from Background".to_string(),
                                given: steps.clone(),
                                when: vec![],
                                then: vec![],
                            }],
                            after: vec![],
                        };
                        scenarios.insert(0, bg_scenario); // Ensure background is first
                    }
                    Statement::EnvDef(vars) => {
                        for var in vars {
                            let value =
                                env::var(var).map_err(|_| AppError::EnvVarNotFound(var.clone()))?;
                            env_vars.insert(var.clone(), value);
                        }
                    }
                    Statement::VarsDef(vars) => {
                        for (key, value) in vars {
                            env_vars.insert(key.clone(), value.as_string());
                        }
                    }
                    Statement::FeatureDef(name) => feature_name = name.clone(),
                    Statement::Scenario(scenario) => scenarios.push(scenario.clone()),
                    _ => {} // Ignore other statement types
                }
            }

            // --- Backend and State Initialisation ---
            let mut terminal_backend =
                TerminalBackend::new(base_dir.to_path_buf(), settings.clone());
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
                            time: Duration::default(),
                            status: TestStatus::Pending,
                            failure_message: None,
                        },
                    )
                })
                .collect();

            // --- Main Test Loop ---
            let suite_start_time = Instant::now();
            let test_timeout = Duration::from_secs(settings.timeout_seconds);

            'scenario_loop: for scenario in &scenarios {
                colours::info(&format!("\nRunning scenario: '{}'", scenario.name));
                let scenario_start_time = Instant::now();

                loop {
                    //terminal_backend.read_pty_output(&mut output_buffer);
                    let elapsed_since_scenario_start = scenario_start_time.elapsed();

                    let mut tests_to_start: Vec<(String, Vec<Action>)> = Vec::new();
                    let mut tests_to_pass = Vec::new();
                    let mut immediate_failures = Vec::new();

                    // --- Checking Phase (Immutable Borrows) ---
                    {
                        let tests_to_check: Vec<TestCase> = scenario
                            .tests
                            .iter()
                            .filter(|tc| !test_states.get(&tc.name).unwrap().is_done())
                            .cloned()
                            .collect();

                        for test_case in &tests_to_check {
                            let current_state = test_states.get(&test_case.name).unwrap();

                            match current_state {
                                TestState::Pending => {
                                    let (given_conditions, given_actions): (
                                        Vec<Condition>,
                                        Vec<Action>,
                                    ) = test_case.given.iter().partition_map(|step| match step {
                                        GivenStep::Condition(c) => {
                                            itertools::Either::Left(c.clone())
                                        }
                                        GivenStep::Action(a) => itertools::Either::Right(a.clone()),
                                    });

                                    // For synchronous tests, don't read the buffer during given checks
                                    // as they manage their own buffer lifecycle
                                    let sync_test = is_synchronous(test_case);
                                    if !sync_test {
                                        terminal_backend.read_pty_output(&mut output_buffer);
                                    }

                                    if check_all_conditions_met(
                                        "given",
                                        &given_conditions,
                                        &test_states,
                                        &output_buffer,
                                        &terminal_backend.last_stderr,
                                        elapsed_since_scenario_start.as_secs_f32(),
                                        &mut env_vars,
                                        &last_exit_code,
                                        &fs_backend,
                                        verbose,
                                    ) {
                                        tests_to_start
                                            .push((test_case.name.clone(), given_actions));
                                    }
                                }
                                TestState::Running => {
                                    // Only read output for ASYNC tests
                                    let test_case_for_running = scenario
                                        .tests
                                        .iter()
                                        .find(|tc| tc.name == test_case.name)
                                        .unwrap();

                                    if !is_synchronous(test_case_for_running) {
                                        terminal_backend.read_pty_output(&mut output_buffer);
                                    }

                                    if check_all_conditions_met(
                                        "then",
                                        &test_case.then,
                                        &test_states,
                                        &output_buffer,
                                        &terminal_backend.last_stderr,
                                        test_start_times
                                            .get(&test_case.name)
                                            .map_or(0.0, |start| start.elapsed().as_secs_f32()),
                                        &mut env_vars,
                                        &last_exit_code,
                                        &fs_backend,
                                        verbose,
                                    ) {
                                        tests_to_pass.push(test_case.name.clone());
                                    } else if test_start_times
                                        .get(&test_case.name)
                                        .map_or(false, |start| start.elapsed() > test_timeout)
                                    {
                                        immediate_failures.push((
                                            test_case.name.clone(),
                                            format!(
                                                "Test timed out after {} seconds",
                                                settings.timeout_seconds
                                            ),
                                        ));
                                    }
                                }
                                _ => {}
                            }
                        }
                    } // End of checking phase scope

                    // --- Updating Phase (Mutable Borrows) ---
                    if !tests_to_start.is_empty() {
                        for (name, given_actions) in tests_to_start {
                            let test_case =
                                scenario.tests.iter().find(|tc| tc.name == name).unwrap();

                            if is_synchronous(test_case) {
                                println!(" ▶️ Starting SYNC test: {}", name);
                                test_states.insert(name.clone(), TestState::Running); // Mark as running immediately
                                test_start_times.insert(name.clone(), Instant::now());
                                // Execute given actions
                                for given_action in &given_actions {
                                    let substituted_action =
                                        substitute_variables_in_action(given_action, &env_vars);
                                    execute_action(
                                        &substituted_action,
                                        &mut terminal_backend,
                                        &fs_backend,
                                        &mut last_exit_code,
                                        settings.timeout_seconds,
                                    );
                                    // Capture output after each action
                                    terminal_backend.read_pty_output(&mut output_buffer);
                                }

                                // Clear buffer, run actions, get output
                                output_buffer.clear();

                                for action in &test_case.when {
                                    let substituted_action =
                                        substitute_variables_in_action(action, &env_vars);
                                    execute_action(
                                        &substituted_action,
                                        &mut terminal_backend,
                                        &fs_backend,
                                        &mut last_exit_code,
                                        settings.timeout_seconds,
                                    );
                                }

                                // Capture output immediately after the action
                                terminal_backend.read_pty_output(&mut output_buffer);

                                // Check if the action itself timed out
                                if let Some(137) = last_exit_code {
                                    break; // Exit the action loop on timeout
                                }

                                let passed = check_all_conditions_met(
                                    "then",
                                    &test_case.then,
                                    &test_states,
                                    &output_buffer,
                                    &terminal_backend.last_stderr,
                                    test_start_times
                                        .get(&name)
                                        .map_or(0.0, |start| start.elapsed().as_secs_f32()),
                                    &mut env_vars,
                                    &last_exit_code,
                                    &fs_backend,
                                    verbose,
                                );

                                // Now, get a mutable borrow to update the state
                                if let Some(state) = test_states.get_mut(&name) {
                                    if passed {
                                        *state = TestState::Passed;
                                        colours::success(&format!(" 🟢 Test Passed: {}", name));
                                    } else {
                                        let mut error_msg =
                                            "Synchronous test conditions not met".to_string();
                                        // Provide a better error if stderr has content
                                        if !terminal_backend.last_stderr.is_empty() {
                                            error_msg = format!(
                                                "Synchronous test failed. Stderr: {}",
                                                terminal_backend.last_stderr.trim()
                                            );
                                        }
                                        *state = TestState::Failed(error_msg.clone());
                                        colours::error(&format!(
                                            " 🔴 Test Failed: {} - {}",
                                            name, error_msg
                                        ));
                                    }
                                }
                            } else {
                                // Handle ASYNC tests as before
                                if let Some(state) = test_states.get_mut(&name) {
                                    println!(" ▶  Starting ASYNC test: {}", name);
                                    *state = TestState::Running;
                                    test_start_times.insert(name.clone(), Instant::now());
                                    // Execute given actions
                                    for given_action in &given_actions {
                                        let substituted_action =
                                            substitute_variables_in_action(given_action, &env_vars);
                                        execute_action(
                                            &substituted_action,
                                            &mut terminal_backend,
                                            &fs_backend,
                                            &mut last_exit_code,
                                            settings.timeout_seconds,
                                        );
                                    }
                                    // Execute when actions
                                    for action in &test_case.when {
                                        let substituted_action =
                                            substitute_variables_in_action(action, &env_vars);
                                        execute_action(
                                            &substituted_action,
                                            &mut terminal_backend,
                                            &fs_backend,
                                            &mut last_exit_code,
                                            settings.timeout_seconds,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    for name in tests_to_pass {
                        if let Some(state) = test_states.get_mut(&name) {
                            if !state.is_done() {
                                *state = TestState::Passed;
                                colours::success(&format!(" 🟢  Test Passed: {}", name));
                            }
                        }
                    }

                    for (name, error_msg) in immediate_failures {
                        if let Some(state) = test_states.get_mut(&name) {
                            if !state.is_done() {
                                *state = TestState::Failed(error_msg.clone());
                                colours::error(&format!(
                                    " 🔴  Test Failed: {} - {}",
                                    name, error_msg
                                ));
                            }
                        }
                    }

                    // Check if all tests in this scenario are done
                    let all_done = scenario
                        .tests
                        .iter()
                        .all(|t| test_states.get(&t.name).unwrap().is_done());

                    // Also check if there are any tests that are still pending. If so, we are not done.
                    let any_pending = scenario
                        .tests
                        .iter()
                        .any(|t| matches!(test_states.get(&t.name).unwrap(), TestState::Pending));

                    if all_done && !any_pending {
                        // Execute after block if it exists
                        if !scenario.after.is_empty() {
                            colours::info("\nRunning after block...");
                            for action in &scenario.after {
                                let substituted_action =
                                    substitute_variables_in_action(action, &env_vars);
                                execute_action(
                                    &substituted_action,
                                    &mut terminal_backend,
                                    &fs_backend,
                                    &mut last_exit_code,
                                    settings.timeout_seconds,
                                );
                            }
                        }
                        break;
                    }

                    // Check for stop_on_failure
                    if settings.stop_on_failure && test_states.values().any(|s| s.is_failed()) {
                        // Mark all pending tests as skipped
                        for (_name, state) in test_states.iter_mut() {
                            if matches!(*state, TestState::Pending) {
                                *state = TestState::Skipped;
                            }
                        }
                        colours::error(
                            "\nStopping test run due to failure (stop_on_failure is true).",
                        );
                        break 'scenario_loop;
                    }

                    //thread::sleep(Duration::from_millis(100));
                }
            }

            let suite_duration = suite_start_time.elapsed();

            // Update reports with final states and times
            for (name, state) in &test_states {
                if let Some(report) = test_reports.get_mut(name) {
                    report.status = match state {
                        TestState::Passed => TestStatus::Passed,
                        TestState::Failed(_) => TestStatus::Failed,
                        _ => TestStatus::Skipped,
                    };
                    if let TestState::Failed(reason) = state {
                        report.failure_message = Some(reason.clone());
                    }
                    if let Some(start_time) = test_start_times.get(name) {
                        report.time = start_time.elapsed();
                    }
                }
            }

            let final_reports: Vec<TestCaseReport> = test_cases
                .iter()
                .map(|tc| test_reports.get(&tc.name).unwrap().clone())
                .collect();

            if verbose {
                colours::info(&format!(
                    "\nTest suite '{}' completed in {:.2}s",
                    suite_name,
                    suite_duration.as_secs_f32()
                ));
            }

            for report in &final_reports {
                match &report.status {
                    TestStatus::Pending => {
                        // This state should not be possible here
                        colours::error(&format!(" ❓ UNKNOWN: {}", report.name));
                    }
                    TestStatus::Passed => {
                        colours::success(&format!(" 🟢  PASSED: {}", report.name));
                    }
                    TestStatus::Failed => {
                        colours::error(&format!(" 🔴  FAILED: {}", report.name));
                        if let Some(msg) = &report.failure_message {
                            colours::error(&format!("      Reason: {}", msg));
                        }
                    }
                    TestStatus::Skipped => {
                        colours::warn(&format!(" ➖ SKIPPED: {}", report.name));
                    }
                    TestStatus::Running => {
                        // This state should not be possible here
                        colours::error(&format!(" ❓ UNKNOWN: {}", report.name));
                    }
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

            // This makes sure that choreo exits with a non-zero
            // status code if any tests failed. This is special for the regression tests in my CI pipeline
            let failures = test_states.values().filter(|s| s.is_failed()).count();
            if failures > settings.expected_failures {
                return Err(AppError::TestsFailed {
                    count: failures,
                    expected: settings.expected_failures,
                });
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
    timeout_seconds: u64,
) {
    // Check if it's a terminal action
    if terminal.execute_action(
        action,
        last_exit_code,
        Some(Duration::from_secs(timeout_seconds)),
    ) {
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

        // After hooks
        for action in &scenario.after {
            // Substitute variables before formatting for the report
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
