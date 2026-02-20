use crate::backend::filesystem_backend::FileSystemBackend;
use crate::backend::system_backend::SystemBackend;
use crate::backend::terminal_backend::TerminalBackend;
use crate::backend::web_backend::WebBackend;
use crate::colours;
use crate::error::AppError;
use crate::parser::ast::{
    Action, Condition, GivenStep, ReportFormat, Scenario, Statement, TestCase, TestState,
    TestSuite, TestSuiteSettings,
};
use crate::parser::helpers::{
    check_all_conditions_met, is_synchronous, substitute_variables_in_action,
};
use crate::parser::parser::expand_foreach_blocks;
use crate::reporting::generate_choreo_report;
use itertools::Itertools;
use rayon::prelude::*;
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

    pub fn run(&mut self, suite_name: &str, scenarios: &[Scenario]) -> Result<(), AppError> {
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

        // Clone scenarios into a mutable Vec so we can remove Background and run it first.
        let mut scenarios_vec: Vec<_> = scenarios.iter().cloned().collect();
        let mut bg_http_headers: HashMap<String, String> = HashMap::new();

        // Run any Background scenario first so its actions (e.g. Web set_header) modify `self.env_vars`.
        if let Some(pos) = scenarios_vec.iter().position(|s| s.name == "Background") {
            if self.verbose {
                colours::info("Running Background setup first...");
            }

            let bg = scenarios_vec.remove(pos);

            // Per-background backends (run on main thread, mutating self.env_vars)
            let mut terminal_backend =
                TerminalBackend::new(self.base_dir.clone(), settings.clone());
            let fs_backend = FileSystemBackend::new();
            let mut web_backend = WebBackend::new();
            let mut last_exit_code: Option<i32> = None;

            // Background was created in main as a single test with given steps.
            for test in &bg.tests {
                for step in &test.given {
                    if let GivenStep::Action(action) = step {
                        // Use the runner method which mutates self.env_vars
                        self.execute_action(
                            action,
                            &mut terminal_backend,
                            &fs_backend,
                            &mut web_backend,
                            &mut last_exit_code,
                            settings.timeout_seconds,
                        );
                    }
                }
            }

            // Capture headers set by the background web backend so scenarios inherit them
            bg_http_headers = web_backend.get_headers();

            if self.verbose {
                colours::success("Background setup applied to runner env_vars.");
            }
        }

        // -- Parallel execution using Rayon --
        //let mut test_states: HashMap<String, TestState> = HashMap::new();
        //let mut test_start_times: HashMap<String, Instant> = HashMap::new();

        let test_states = Arc::new(Mutex::new(HashMap::new()));
        let test_start_times = Arc::new(Mutex::new(HashMap::new()));

        // --- Main Test Loop ---
        let suite_start_time = Instant::now();

        {
            let mut states = test_states.lock().unwrap();
            for sc in scenarios {
                for t in &sc.tests {
                    states.entry(t.name.clone()).or_insert(TestState::Pending);
                }
            }
        }

        // Separate parallel and sequential scenarios
        let (parallel_scenarios, sequential_scenarios): (Vec<_>, Vec<_>) =
            scenarios.iter().cloned().partition(|s| s.parallel);

        if !parallel_scenarios.is_empty() {
            if self.verbose {
                colours::info(&format!(
                    "\nRunning {} scenarios in parallel...",
                    parallel_scenarios.len()
                ));
            }
            let parallel_results: Vec<Result<(), AppError>> = parallel_scenarios
                .par_iter()
                .cloned() // clone Scenario if needed so closure owns it
                .map(|scenario| {
                    // each closure runs in parallel; pass cloned Arcs
                    run_scenario(
                        &scenario,
                        &settings,
                        self.env_vars.clone(),
                        self.verbose,
                        &self.base_dir,
                        Arc::clone(&test_states),
                        Arc::clone(&test_start_times),
                        bg_http_headers.clone(),
                    )
                })
                .collect();

            // Propagate any errors (return first Err)
            for res in parallel_results {
                res?;
            }
        }

        if !sequential_scenarios.is_empty() {
            if self.verbose {
                colours::info(&format!(
                    "\nRunning {} scenarios sequentially...",
                    sequential_scenarios.len()
                ));
            }

            // Run sequential scenarios on current thread (or spawn them too, depending on desired semantics)
            for scenario in sequential_scenarios {
                run_scenario(
                    &scenario,
                    &settings,
                    self.env_vars.clone(),
                    self.verbose,
                    &self.base_dir,
                    Arc::clone(&test_states),
                    Arc::clone(&test_start_times),
                    bg_http_headers.clone(),
                )?;
            }
        }

        // --- Final Reporting ---
        let suite_duration = suite_start_time.elapsed();
        // Snapshot final maps for reporting
        let test_states_final = test_states.lock().unwrap().clone();
        let test_start_times_final = test_start_times.lock().unwrap().clone();

        // This is the logic from the old print_summary function
        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;
        for state in test_states_final.values() {
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
            test_states_final.len(),
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
                &test_states_final,
                &test_start_times_final,
                &mut self.env_vars,
                &settings,
                self.verbose,
            )?;

            if self.verbose {
                colours::success("Reports generated successfully.");
            }
        }

        let failures = test_states_final.values().filter(|s| s.is_failed()).count();
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
            self.verbose,
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

// --- Thread-safe run_scenario ---
// Accepts shared Arcs for states/times so multiple scenarios can run in parallel.
fn run_scenario(
    scenario: &Scenario,
    settings: &TestSuiteSettings,
    env_vars: HashMap<String, String>,
    verbose: bool,
    base_dir: &PathBuf,
    test_states: Arc<Mutex<HashMap<String, TestState>>>,
    test_start_times: Arc<Mutex<HashMap<String, Instant>>>,
    initial_http_headers: HashMap<String, String>,
) -> Result<(), AppError> {
    // Per-scenario isolated backends and mutable state
    let mut terminal_backend = TerminalBackend::new(base_dir.clone(), settings.clone());
    let fs_backend = FileSystemBackend::new();
    let mut web_backend = WebBackend::with_headers(initial_http_headers);
    let mut system_backend = SystemBackend::new();
    let mut variables = env_vars.clone();
    let test_timeout = Duration::from_secs(settings.timeout_seconds);
    let mut last_exit_code: Option<i32> = None;
    let mut output_buffer = String::new();

    let expanded_tests = expand_foreach_blocks(scenario, &variables);
    let mut scenario_clone = scenario.clone();
    scenario_clone.tests = expanded_tests.clone();

    // Initialise tests states (insert Pending entries) under lock
    {
        let mut states = test_states.lock().unwrap();
        for test in &expanded_tests {
            states
                .entry(test.name.clone())
                .or_insert(TestState::Pending);
        }
    }

    let scenario_start_time = Instant::now();
    colours::info(&format!("\nRunning scenario: '{}'", scenario.name));
    'scenario_loop: loop {
        let elapsed_since_scenario_start = scenario_start_time.elapsed();
        let mut progress_made = false;

        //let mut tests_to_start: Vec<(String, Vec<Action>)> = Vec::new();
        let mut tests_to_start: Vec<(String, Vec<Action>, bool)> = Vec::new();
        let mut tests_to_pass: Vec<String> = Vec::new();
        let mut immediate_failures: Vec<(String, String)> = Vec::new();

        // --- Checking Phase: take snapshots of shared maps and operate on snapshots ---
        let states_snapshot: HashMap<String, TestState> = {
            let locked = test_states.lock().unwrap();
            locked.clone()
        };
        let start_times_snapshot: HashMap<String, Instant> = {
            let locked = test_start_times.lock().unwrap();
            locked.clone()
        };

        // Determine tests to evaluate (not done)
        let tests_to_check: Vec<TestCase> = expanded_tests
            .iter()
            .filter(|tc| !states_snapshot.get(&tc.name).unwrap().is_done())
            .cloned()
            .collect();

        //println!("How many tests? '{}'", tests_to_check.len());
        for test_case in &tests_to_check {
            let current_state = states_snapshot.get(&test_case.name).unwrap().clone();
            //println!("Running test: '{}'", test_case.name);

            match current_state {
                TestState::Pending => {
                    let (given_conditions, given_actions): (Vec<Condition>, Vec<Action>) =
                        test_case.given.iter().partition_map(|step| match step {
                            GivenStep::Condition(c) => itertools::Either::Left(c.clone()),
                            GivenStep::Action(a) => itertools::Either::Right(a.clone()),
                        });

                    let is_sync = is_synchronous(&test_case);
                    //println!("Test is sync = {}", is_sync);

                    let sync_test = is_synchronous(test_case);
                    if !sync_test {
                        terminal_backend.read_pty_output(&mut output_buffer);
                    }

                    if check_all_conditions_met(
                        "given",
                        &given_conditions,
                        &states_snapshot,
                        &output_buffer,
                        &terminal_backend.last_stderr.clone(),
                        elapsed_since_scenario_start.as_secs_f32(),
                        &mut variables,
                        &last_exit_code,
                        &fs_backend,
                        &mut terminal_backend,
                        &mut web_backend,
                        &system_backend,
                        verbose,
                    ) {
                        tests_to_start.push((
                            test_case.name.clone(),
                            given_actions.clone(),
                            is_sync,
                        ));
                        //tests_to_start.push((test_case.name.clone(), given_actions));
                    }
                }
                TestState::Running => {
                    if !is_synchronous(test_case) {
                        terminal_backend.read_pty_output(&mut output_buffer);
                    }

                    let elapsed_for_test = start_times_snapshot
                        .get(&test_case.name)
                        .map_or(0.0, |start| start.elapsed().as_secs_f32());

                    if check_all_conditions_met(
                        "then",
                        &test_case.then,
                        &states_snapshot,
                        &output_buffer,
                        &terminal_backend.last_stderr.clone(),
                        elapsed_for_test,
                        &mut variables,
                        &last_exit_code,
                        &fs_backend,
                        &mut terminal_backend,
                        &mut web_backend,
                        &system_backend,
                        verbose,
                    ) {
                        tests_to_pass.push(test_case.name.clone());
                    } else if start_times_snapshot
                        .get(&test_case.name)
                        .map_or(false, |start| start.elapsed() > test_timeout)
                    {
                        immediate_failures.push((
                            test_case.name.clone(),
                            format!("Test timed out after {} seconds", settings.timeout_seconds),
                        ));
                    }
                }
                _ => {}
            }
        }

        // --- Updating Phase: perform mutations under brief locks, but run actions without locks ---
        if !tests_to_start.is_empty() {
            progress_made = true;
            for (name, given_actions, is_sync) in tests_to_start {
                let test_case = expanded_tests.iter().find(|tc| tc.name == name).unwrap();

                if is_sync {
                    //println!(" ▶️ Starting SYNC test: {}", name);
                    {
                        let mut states = test_states.lock().unwrap();
                        states.insert(name.clone(), TestState::Running);
                    }
                    {
                        let mut starts = test_start_times.lock().unwrap();
                        starts.insert(name.clone(), Instant::now());
                    }

                    // execute given actions and when actions (no locks held)
                    for given_action in &given_actions {
                        let substituted_action =
                            substitute_variables_in_action(given_action, &mut variables);
                        execute_action(
                            &substituted_action,
                            &mut terminal_backend,
                            &fs_backend,
                            &mut web_backend,
                            &mut system_backend,
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
                            &mut system_backend,
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
                        &{
                            let locked = test_states.lock().unwrap();
                            locked.clone()
                        },
                        &output_buffer,
                        &terminal_backend.last_stderr.clone(),
                        test_start_times
                            .lock()
                            .unwrap()
                            .get(&name)
                            .map_or(0.0, |start| start.elapsed().as_secs_f32()),
                        &mut variables,
                        &last_exit_code,
                        &fs_backend,
                        &mut terminal_backend,
                        &mut web_backend,
                        &system_backend,
                        verbose,
                    );

                    if let Some(mut state_guard) = test_states.lock().ok() {
                        if passed {
                            state_guard.insert(name.clone(), TestState::Passed);
                            colours::success(&format!(" 🟢 Test Passed: {}", name));
                        } else {
                            let mut error_msg = "Synchronous test conditions not met".to_string();
                            if !terminal_backend.last_stderr.is_empty() {
                                error_msg = format!(
                                    "Synchronous test failed. Stderr: {}",
                                    terminal_backend.last_stderr.trim()
                                );
                            }
                            state_guard.insert(name.clone(), TestState::Failed(error_msg.clone()));
                            colours::error(&format!(" 🔴 Test Failed: {} - {}", name, error_msg));
                        }
                    }

                    // Mark that progress was made so the outer loop won't spin
                    progress_made = true;

                    if settings.stop_on_failure
                        && test_states.lock().unwrap().values().any(|s| s.is_failed())
                    {
                        break;
                    }
                    continue; // re-evaluate after a sync test finishes
                } else {
                    // async case: mark running and execute given/when without holding locks while executing actions
                    //println!(" ▶  Starting ASYNC test: {}", name);
                    {
                        let mut states = test_states.lock().unwrap();
                        states.insert(name.clone(), TestState::Running);
                    }
                    {
                        let mut starts = test_start_times.lock().unwrap();
                        starts.insert(name.clone(), Instant::now());
                    }

                    // Mark progress once the test has been started
                    progress_made = true;

                    for given_action in &given_actions {
                        let substituted_action =
                            substitute_variables_in_action(given_action, &mut variables);
                        execute_action(
                            &substituted_action,
                            &mut terminal_backend,
                            &fs_backend,
                            &mut web_backend,
                            &mut system_backend,
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
                            &mut system_backend,
                            &mut last_exit_code,
                            settings.timeout_seconds,
                            &mut variables,
                            verbose,
                        );
                    }
                }
            }
        }

        if !tests_to_pass.is_empty() {
            progress_made = true;
            let mut states = test_states.lock().unwrap();
            for name in tests_to_pass {
                if let Some(state) = states.get_mut(&name) {
                    if !state.is_done() {
                        *state = TestState::Passed;
                        colours::success(&format!(" 🟢  Test Passed: {}", name));
                    }
                }
            }
        }

        if !immediate_failures.is_empty() {
            progress_made = true;
            let mut states = test_states.lock().unwrap();
            for (name, error_msg) in immediate_failures {
                if let Some(state) = states.get_mut(&name) {
                    if !state.is_done() {
                        *state = TestState::Failed(error_msg.clone());
                        colours::error(&format!(" 🔴  Test Failed: {} - {}", name, error_msg));
                    }
                }
            }
        }

        // Check if all tests in this scenario are done
        let all_done = {
            let states = test_states.lock().unwrap();
            scenario
                .tests
                .iter()
                .all(|t| states.get(&t.name).unwrap().is_done())
        };

        if all_done {
            if !scenario.after.is_empty() {
                colours::info("\nRunning after block...");
                for action in &scenario.after {
                    let substituted_action = substitute_variables_in_action(action, &mut variables);
                    execute_action(
                        &substituted_action,
                        &mut terminal_backend,
                        &fs_backend,
                        &mut web_backend,
                        &mut system_backend,
                        &mut last_exit_code,
                        settings.timeout_seconds,
                        &mut variables,
                        verbose,
                    );
                }
            }
            break;
        }

        // stop_on_failure handling across global shared state
        if settings.stop_on_failure && test_states.lock().unwrap().values().any(|s| s.is_failed()) {
            let mut states = test_states.lock().unwrap();
            for (_name, state) in states.iter_mut() {
                if matches!(*state, TestState::Pending | TestState::Running) {
                    *state = TestState::Skipped;
                }
            }
            colours::error("\nStopping test run due to failure (stop_on_failure is true).");
            break 'scenario_loop;
        }

        if !progress_made {
            thread::sleep(Duration::from_millis(50));
            let elapsed_since_suite_start = scenario_start_time.elapsed();
            if elapsed_since_suite_start > test_timeout + Duration::from_secs(1) {
                colours::warn("\nWarning: No progress was made in the last loop iteration, and the scenario is not complete. Marking remaining tests as skipped.");
                let mut states = test_states.lock().unwrap();
                for test in &scenario.tests {
                    if let Some(state) = states.get_mut(&test.name) {
                        if matches!(*state, TestState::Pending | TestState::Running) {
                            *state = TestState::Skipped;
                        }
                    }
                }
                break;
            }
        }
    } // end scenario loop

    Ok(())
}

/// Dispatches an action to the correct backend.
fn execute_action(
    action: &Action,
    terminal: &mut TerminalBackend,
    fs: &FileSystemBackend,
    web: &mut WebBackend,
    system: &mut SystemBackend,
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

    // Check if it's a system action first
    if system.execute_action(&substituted_action, env_vars, verbose) {
        return;
    }

    // Check if it's a terminal action
    if terminal.execute_action(
        &substituted_action,
        last_exit_code,
        Some(Duration::from_secs(timeout_seconds)),
        env_vars,
        verbose,
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
