use crate::backend::filesystem_backend::FileSystemBackend;
use crate::backend::terminal_backend::TerminalBackend;
use crate::backend::web_backend::WebBackend;
use crate::colours;
use crate::error::AppError;
use crate::parser::ast::{
    Action, Condition, GivenStep, ReportFormat, Statement, TestCase, TestState, TestSuite,
    TestSuiteSettings,
};
use crate::parser::helpers::{
    check_all_conditions_met, is_synchronous, substitute_variables_in_action,
};
use crate::reporting::generate_choreo_report;
use itertools::Itertools;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct TestRunner {
    test_suite: TestSuite,
    base_dir: PathBuf,
    env_vars: HashMap<String, String>,
    verbose: bool,
}

impl TestRunner {
    pub fn new(
        test_suite: TestSuite,
        base_dir: PathBuf,
        env_vars: HashMap<String, String>,
        verbose: bool,
    ) -> Self {
        Self {
            test_suite,
            base_dir,
            env_vars,
            verbose,
        }
    }

    pub fn run(
        &mut self,
        suite_name: &str,
        scenarios: &[crate::parser::ast::Scenario],
    ) -> Result<(), AppError> {
        let mut settings = TestSuiteSettings::default();
        let mut feature_name = "Choreo Test Feature".to_string(); // Default name
                                                                  //let mut scenarios: Vec<crate::parser::ast::Scenario> = Vec::new();

        for s in &self.test_suite.statements {
            match s {
                Statement::SettingsDef(s_def) => settings = s_def.clone(),
                Statement::FeatureDef(name) => feature_name = name.clone(),
                _ => {}
            }
        }

        // Set a default shell path if not provided
        if settings.shell_path.is_none() {
            settings.shell_path = Some("/bin/sh".to_string());
        }

        // --- Backend and State Initialisation ---
        let mut test_states: HashMap<String, TestState> = HashMap::new();
        let mut test_start_times: HashMap<String, Instant> = HashMap::new();

        // --- Main Test Loop ---
        let suite_start_time = Instant::now();

        // Separate parallel and sequential scenarios
        let (parallel_scenarios, sequential_scenarios): (Vec<_>, Vec<_>) =
            scenarios.iter().cloned().partition(|s| s.parallel);

        if !parallel_scenarios.is_empty() {
            if self.verbose {
                colours::info(&format!(
                    "\nRunning {} scenarios in parallel... but not running yet",
                    parallel_scenarios.len()
                ));
            }
            // TODO: Implement parallel execution later
        }

        if !sequential_scenarios.is_empty() {
            if self.verbose {
                colours::info(&format!(
                    "\nRunning {} scenarios sequentially...",
                    sequential_scenarios.len()
                ));
            }

            // Call the sequential scenario runner with proper parameters
            let (states, start_times) = run_scenarios_seq(
                &sequential_scenarios,
                &settings,
                self.env_vars.clone(),
                self.verbose,
                &self.base_dir,
            )?;
            test_states = states;
            test_start_times = start_times;
        }

        // --- Final Reporting ---
        let suite_duration = suite_start_time.elapsed();

        // This is the logic from the old print_summary function
        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;
        for state in test_states.values() {
            match state {
                TestState::Passed => passed += 1,
                TestState::Failed(_) => failed += 1,
                TestState::Skipped => skipped += 1,
                _ => {}
            }
        }
        colours::info(&format!(
            "\nTest suite '{}' summary: {} tests run in {:.2}s ({} passed, {} failed, {} skipped)",
            suite_name,
            test_states.len(),
            suite_duration.as_secs_f32(),
            passed,
            failed,
            skipped
        ));

        if settings.report_format != ReportFormat::None {
            generate_choreo_report(
                suite_name,
                suite_start_time.elapsed(),
                &feature_name,
                &*scenarios,
                &test_states,
                &test_start_times,
                &mut self.env_vars,
                &settings,
                self.verbose,
            )?;

            if self.verbose {
                colours::success("Reports generated successfully.");
            }
        }

        let failures = test_states.values().filter(|s| s.is_failed()).count();
        if failures > settings.expected_failures {
            return Err(AppError::TestsFailed {
                count: failures,
                expected: settings.expected_failures,
            });
        }

        Ok(())
    }

    /// Dispatches an action to the correct backend.
    fn execute_action(
        &mut self, // Make it a method
        action: &Action,
        terminal: &mut TerminalBackend,
        fs: &FileSystemBackend,
        web: &mut WebBackend,
        last_exit_code: &mut Option<i32>,
        timeout_seconds: u64,
    ) {
        if self.verbose {
            colours::info(&format!("[RUNNER] Executing action: {:?}", action));
        }
        // Substitute variables in the action
        let substituted_action = substitute_variables_in_action(action, &self.env_vars);

        let env_vars = &mut self.env_vars;

        // Check if it's a terminal action
        if terminal.execute_action(
            &substituted_action,
            last_exit_code,
            Some(Duration::from_secs(timeout_seconds)),
            env_vars,
        ) {
            return;
        }
        // Check if it's a filesystem action
        if fs.execute_action(&substituted_action, terminal.get_cwd(), env_vars) {
            return;
        }

        // Check if it's a web action
        if web.execute_action(&substituted_action, env_vars, self.verbose) {
            return;
        } else {
            println!(
                "[WARNING] Web action failed to execute: {:?}",
                substituted_action
            );
        }
        println!(
            "[WARNING] Action not recognised by any backend: {:?}",
            action
        );
    }
}

/// Executes a single scenario, vanilla
fn run_scenarios_seq(
    scenarios: &Vec<crate::parser::ast::Scenario>,
    settings: &TestSuiteSettings,
    env_vars: HashMap<String, String>,
    verbose: bool,
    base_dir: &PathBuf,
) -> Result<(HashMap<String, TestState>, HashMap<String, Instant>), AppError> {
    // --- Backend and State Initialisation ---
    let mut terminal_backend = TerminalBackend::new(base_dir.clone(), settings.clone());
    let mut web_backend = WebBackend::new();
    let fs_backend = FileSystemBackend::new();
    let mut last_exit_code: Option<i32> = None;
    let mut output_buffer = String::new();
    let mut test_states: HashMap<String, TestState> = HashMap::new();
    let mut test_start_times: HashMap<String, Instant> = HashMap::new();

    for scenario in scenarios {
        for test in &scenario.tests {
            test_states.insert(test.name.clone(), TestState::Pending);
        }
    }

    let mut variables = env_vars.clone();

    // --- Main Test Loop ---
    let suite_start_time = Instant::now();
    let test_timeout = Duration::from_secs(settings.timeout_seconds);

    'scenario_loop: for scenario in scenarios {
        colours::info(&format!("\nRunning scenario: '{}'", scenario.name));
        let scenario_start_time = Instant::now();

        loop {
            let elapsed_since_scenario_start = scenario_start_time.elapsed();
            let mut progress_made = false;

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
                            let (given_conditions, given_actions): (Vec<Condition>, Vec<Action>) =
                                test_case.given.iter().partition_map(|step| match step {
                                    GivenStep::Condition(c) => itertools::Either::Left(c.clone()),
                                    GivenStep::Action(a) => itertools::Either::Right(a.clone()),
                                });

                            let sync_test = is_synchronous(test_case);
                            if !sync_test {
                                terminal_backend.read_pty_output(&mut output_buffer);
                            }

                            if check_all_conditions_met(
                                "given",
                                &given_conditions,
                                &test_states,
                                &output_buffer,
                                &terminal_backend.last_stderr.clone(),
                                elapsed_since_scenario_start.as_secs_f32(),
                                &mut variables,
                                &last_exit_code,
                                &fs_backend,
                                &mut terminal_backend,
                                &mut web_backend,
                                verbose,
                            ) {
                                tests_to_start.push((test_case.name.clone(), given_actions));
                            }
                        }
                        TestState::Running => {
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
                                &terminal_backend.last_stderr.clone(),
                                test_start_times
                                    .get(&test_case.name)
                                    .map_or(0.0, |start| start.elapsed().as_secs_f32()),
                                &mut variables,
                                &last_exit_code,
                                &fs_backend,
                                &mut terminal_backend,
                                &mut web_backend,
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
                progress_made = true;
                for (name, given_actions) in tests_to_start {
                    let test_case = scenario.tests.iter().find(|tc| tc.name == name).unwrap();

                    if is_synchronous(test_case) {
                        println!(" ▶️ Starting SYNC test: {}", name);
                        test_states.insert(name.clone(), TestState::Running);
                        test_start_times.insert(name.clone(), Instant::now());
                        for given_action in &given_actions {
                            let substituted_action =
                                substitute_variables_in_action(given_action, &mut variables);
                            execute_action(
                                &substituted_action,
                                &mut terminal_backend,
                                &fs_backend,
                                &mut web_backend,
                                &mut last_exit_code,
                                settings.timeout_seconds,
                                &mut variables,
                                verbose,
                            );
                        }

                        for action in &test_case.when {
                            let substituted_action =
                                substitute_variables_in_action(action, &mut variables);
                            execute_action(
                                &substituted_action,
                                &mut terminal_backend,
                                &fs_backend,
                                &mut web_backend,
                                &mut last_exit_code,
                                settings.timeout_seconds,
                                &mut variables,
                                verbose,
                            );
                        }

                        if let Some(137) = last_exit_code {
                            break;
                        }

                        let passed = check_all_conditions_met(
                            "then",
                            &test_case.then,
                            &test_states,
                            &output_buffer,
                            &terminal_backend.last_stderr.clone(),
                            test_start_times
                                .get(&name)
                                .map_or(0.0, |start| start.elapsed().as_secs_f32()),
                            &mut variables,
                            &last_exit_code,
                            &fs_backend,
                            &mut terminal_backend,
                            &mut web_backend,
                            verbose,
                        );

                        if let Some(state) = test_states.get_mut(&name) {
                            if passed {
                                *state = TestState::Passed;
                                colours::success(&format!(" 🟢 Test Passed: {}", name));
                            } else {
                                let mut error_msg =
                                    "Synchronous test conditions not met".to_string();
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
                        // If a sync test fails and we should stop, break the scenario loop now.
                        if settings.stop_on_failure && test_states.values().any(|s| s.is_failed()) {
                            break;
                        }
                        // Force a re-evaluation of conditions for the next test
                        continue;
                    } else {
                        if let Some(state) = test_states.get_mut(&name) {
                            println!(" ▶  Starting ASYNC test: {}", name);
                            *state = TestState::Running;
                            test_start_times.insert(name.clone(), Instant::now());
                            for given_action in &given_actions {
                                let substituted_action =
                                    substitute_variables_in_action(given_action, &mut variables);
                                execute_action(
                                    &substituted_action,
                                    &mut terminal_backend,
                                    &fs_backend,
                                    &mut web_backend,
                                    &mut last_exit_code,
                                    settings.timeout_seconds,
                                    &mut variables,
                                    verbose,
                                );
                            }
                            for action in &test_case.when {
                                let substituted_action =
                                    substitute_variables_in_action(action, &mut variables);
                                execute_action(
                                    &substituted_action,
                                    &mut terminal_backend,
                                    &fs_backend,
                                    &mut web_backend,
                                    &mut last_exit_code,
                                    settings.timeout_seconds,
                                    &mut variables,
                                    verbose,
                                );
                            }
                        }
                    }
                }
            }
            if !tests_to_pass.is_empty() {
                progress_made = true;
                for name in tests_to_pass {
                    if let Some(state) = test_states.get_mut(&name) {
                        if !state.is_done() {
                            *state = TestState::Passed;
                            colours::success(&format!(" 🟢  Test Passed: {}", name));
                        }
                    }
                }
            }

            if !immediate_failures.is_empty() {
                progress_made = true;
                for (name, error_msg) in immediate_failures {
                    if let Some(state) = test_states.get_mut(&name) {
                        if !state.is_done() {
                            *state = TestState::Failed(error_msg.clone());
                            colours::error(&format!(" 🔴  Test Failed: {} - {}", name, error_msg));
                        }
                    }
                }
            }

            let all_done = scenario
                .tests
                .iter()
                .all(|t| test_states.get(&t.name).unwrap().is_done());

            if all_done {
                if !scenario.after.is_empty() {
                    colours::info("\nRunning after block...");
                    for action in &scenario.after {
                        let substituted_action =
                            substitute_variables_in_action(action, &mut variables);
                        execute_action(
                            &substituted_action,
                            &mut terminal_backend,
                            &fs_backend,
                            &mut web_backend,
                            &mut last_exit_code,
                            settings.timeout_seconds,
                            &mut variables,
                            verbose,
                        );
                    }
                }
                break;
            }

            if settings.stop_on_failure && test_states.values().any(|s| s.is_failed()) {
                for (_name, state) in test_states.iter_mut() {
                    if matches!(*state, TestState::Pending | TestState::Running) {
                        *state = TestState::Skipped;
                    }
                }
                colours::error("\nStopping test run due to failure (stop_on_failure is true).");
                break 'scenario_loop;
            }

            if !progress_made {
                // If no progress is made, we might be waiting for a time-based condition.
                // Add a small delay to prevent a tight loop from consuming CPU.
                thread::sleep(Duration::from_millis(50));

                // Check if we are truly stuck
                let elapsed_since_suite_start = suite_start_time.elapsed();
                if elapsed_since_suite_start > test_timeout + Duration::from_secs(1) {
                    colours::warn("\nWarning: No progress was made in the last loop iteration, and the scenario is not complete. Breaking to avoid a hang.");
                    // Mark any remaining pending tests as skipped to ensure a clean exit.
                    for test in &scenario.tests {
                        if let Some(state) = test_states.get_mut(&test.name) {
                            if matches!(*state, TestState::Pending | TestState::Running) {
                                *state = TestState::Skipped;
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    Ok((test_states, test_start_times))
}

/// Executes a single scenario, managing its entire lifecycle. This function is thread-safe.
fn run_scenario(
    scenario: &crate::parser::ast::Scenario,
    settings: &TestSuiteSettings,
    background_steps: &[GivenStep],
    test_states: Arc<Mutex<HashMap<String, TestState>>>,
    test_start_times: Arc<Mutex<HashMap<String, Instant>>>,
    env_vars: HashMap<String, String>,
    verbose: bool,
    base_dir: &PathBuf,
) {
    todo!("Working on it, in parallel...")
}

/// Dispatches an action to the correct backend.
fn execute_action(
    action: &Action,
    terminal: &mut TerminalBackend,
    fs: &FileSystemBackend,
    web: &mut WebBackend,
    last_exit_code: &mut Option<i32>,
    timeout_seconds: u64,
    env_vars: &mut HashMap<String, String>,
    verbose: bool,
) {
    if verbose {
        colours::info(&format!("[RUNNER] Executing action: {:?}", action));
    }
    // Substitute variables in the action
    let substituted_action = substitute_variables_in_action(action, env_vars);

    // Check if it's a terminal action
    if terminal.execute_action(
        &substituted_action,
        last_exit_code,
        Some(Duration::from_secs(timeout_seconds)),
        env_vars,
    ) {
        return;
    }
    // Check if it's a filesystem action
    if fs.execute_action(&substituted_action, terminal.get_cwd(), env_vars) {
        return;
    }

    // Check if it's a web action
    if web.execute_action(&substituted_action, env_vars, verbose) {
        return;
    } else {
        println!(
            "[WARNING] Web action failed to execute: {:?}",
            substituted_action
        );
    }
    println!(
        "[WARNING] Action not recognised by any backend: {:?}",
        action
    );
}
