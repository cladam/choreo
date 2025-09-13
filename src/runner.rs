use crate::backend::filesystem_backend::FileSystemBackend;
use crate::backend::terminal_backend::TerminalBackend;
use crate::colours;
use crate::parser::ast::{
    Action, Condition, GivenStep, Statement, TestCase, TestState, TestSuiteSettings,
};
use crate::parser::helpers::*;
use itertools::Itertools;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct TestRunner {
    test_suite: crate::parser::ast::TestSuite,
    base_dir: std::path::PathBuf,
    env_vars: HashMap<String, String>,
    verbose: bool,
}

impl TestRunner {
    pub fn new(
        test_suite: crate::parser::ast::TestSuite,
        base_dir: std::path::PathBuf,
        env_vars: HashMap<String, String>,
        verbose: bool,
    ) -> Self {
        Self {
            test_suite: test_suite.clone(),
            base_dir,
            env_vars,
            verbose,
        }
    }

    pub fn run(
        &mut self,
        scenarios: &[crate::parser::ast::Scenario],
    ) -> (HashMap<String, TestState>, HashMap<String, Instant>) {
        let mut settings = TestSuiteSettings::default();
        for s in &self.test_suite.statements {
            if let Statement::SettingsDef(s_def) = s {
                settings = s_def.clone();
            }
        }

        let mut terminal_backend =
            TerminalBackend::new(self.base_dir.to_path_buf(), settings.clone());
        let fs_backend = FileSystemBackend::new(self.base_dir.to_path_buf());
        let mut last_exit_code: Option<i32> = None;
        let mut output_buffer = String::new();
        let mut test_states: HashMap<String, TestState> = HashMap::new();
        let mut test_start_times: HashMap<String, Instant> = HashMap::new();

        for scenario in scenarios {
            for test in &scenario.tests {
                test_states.insert(test.name.clone(), TestState::Pending);
            }
        }

        let test_timeout = Duration::from_secs(settings.timeout_seconds);

        'scenario_loop: for scenario in scenarios {
            if self.verbose {
                colours::info(&format!("\nRunning scenario: '{}'", scenario.name));
            }
            let scenario_start_time = Instant::now();

            loop {
                let mut state_changed = false;
                let elapsed_since_scenario_start = scenario_start_time.elapsed();

                let mut tests_to_start: Vec<(String, Vec<Action>)> = Vec::new();
                let mut tests_to_pass = Vec::new();
                let mut immediate_failures = Vec::new();

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
                                    &terminal_backend.last_stderr,
                                    elapsed_since_scenario_start.as_secs_f32(),
                                    &mut self.env_vars,
                                    &last_exit_code,
                                    &fs_backend,
                                    self.verbose,
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
                                    &terminal_backend.last_stderr,
                                    test_start_times
                                        .get(&test_case.name)
                                        .map_or(0.0, |start| start.elapsed().as_secs_f32()),
                                    &mut self.env_vars,
                                    &last_exit_code,
                                    &fs_backend,
                                    self.verbose,
                                ) {
                                    tests_to_pass.push(test_case.name.clone());
                                } else if test_start_times
                                    .get(&test_case.name)
                                    .is_some_and(|start| start.elapsed() > test_timeout)
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
                }

                if !tests_to_start.is_empty() {
                    state_changed = true;
                    for (name, given_actions) in tests_to_start {
                        let test_case = scenario.tests.iter().find(|tc| tc.name == name).unwrap();

                        if is_synchronous(test_case) {
                            if self.verbose {
                                println!(" ▶️ Starting SYNC test: {}", name);
                            }
                            test_states.insert(name.clone(), TestState::Running);
                            test_start_times.insert(name.clone(), Instant::now());

                            for given_action in &given_actions {
                                let substituted_action =
                                    substitute_variables_in_action(given_action, &self.env_vars);
                                execute_action(
                                    &substituted_action,
                                    &mut terminal_backend,
                                    &fs_backend,
                                    &mut last_exit_code,
                                    settings.timeout_seconds,
                                );
                                terminal_backend.read_pty_output(&mut output_buffer);
                            }

                            output_buffer.clear();

                            for action in &test_case.when {
                                let substituted_action =
                                    substitute_variables_in_action(action, &self.env_vars);
                                execute_action(
                                    &substituted_action,
                                    &mut terminal_backend,
                                    &fs_backend,
                                    &mut last_exit_code,
                                    settings.timeout_seconds,
                                );
                            }

                            terminal_backend.read_pty_output(&mut output_buffer);

                            if let Some(137) = last_exit_code {
                                break;
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
                                &mut self.env_vars,
                                &last_exit_code,
                                &fs_backend,
                                self.verbose,
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
                        } else {
                            if let Some(state) = test_states.get_mut(&name) {
                                if self.verbose {
                                    println!(" ▶  Starting ASYNC test: {}", name);
                                }
                                *state = TestState::Running;
                                test_start_times.insert(name.clone(), Instant::now());
                                for given_action in &given_actions {
                                    let substituted_action = substitute_variables_in_action(
                                        given_action,
                                        &self.env_vars,
                                    );
                                    execute_action(
                                        &substituted_action,
                                        &mut terminal_backend,
                                        &fs_backend,
                                        &mut last_exit_code,
                                        settings.timeout_seconds,
                                    );
                                }
                                for action in &test_case.when {
                                    let substituted_action =
                                        substitute_variables_in_action(action, &self.env_vars);
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
                if !tests_to_pass.is_empty() {
                    state_changed = true;
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
                    state_changed = true;
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
                }

                let all_done = scenario
                    .tests
                    .iter()
                    .all(|t| test_states.get(&t.name).unwrap().is_done());

                if all_done || !state_changed {
                    if !scenario.after.is_empty() {
                        if self.verbose {
                            colours::info("\nRunning after block...");
                        }
                        for action in &scenario.after {
                            let substituted_action =
                                substitute_variables_in_action(action, &self.env_vars);
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

                if settings.stop_on_failure && test_states.values().any(|s| s.is_failed()) {
                    for (_name, state) in test_states.iter_mut() {
                        if matches!(*state, TestState::Pending) {
                            *state = TestState::Skipped;
                        }
                    }
                    colours::error("\nStopping test run due to failure (stop_on_failure is true).");
                    break 'scenario_loop;
                }
            }
        }

        (test_states, test_start_times)
    }
}

fn execute_action(
    action: &Action,
    terminal: &mut TerminalBackend,
    fs: &FileSystemBackend,
    last_exit_code: &mut Option<i32>,
    timeout_seconds: u64,
) {
    if terminal.execute_action(
        action,
        last_exit_code,
        Some(Duration::from_secs(timeout_seconds)),
    ) {
        return;
    }
    if fs.execute_action(action) {
        return;
    }
}
