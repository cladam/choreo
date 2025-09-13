// src/runner.rs
use crate::backend::filesystem_backend::FileSystemBackend;
use crate::backend::terminal_backend::TerminalBackend;
use crate::colours;
use crate::parser::ast::{
    Action, GivenStep, Scenario, TestCase, TestState, TestSuite, TestSuiteSettings,
};
use crate::parser::helpers::{
    check_all_conditions_met, is_synchronous, substitute_variables_in_action,
};
use itertools::Itertools;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct TestRunner {
    settings: TestSuiteSettings,
    terminal_backend: TerminalBackend,
    fs_backend: FileSystemBackend,
    test_states: HashMap<String, TestState>,
    test_start_times: HashMap<String, Instant>,
    env_vars: HashMap<String, String>,
    last_exit_code: Option<i32>,
    output_buffer: String,
    verbose: bool,
}

impl TestRunner {
    pub fn new(
        test_suite: &TestSuite,
        base_dir: PathBuf,
        mut env_vars: HashMap<String, String>,
        verbose: bool,
    ) -> Self {
        let mut settings = TestSuiteSettings::default();
        for s in &test_suite.statements {
            if let crate::parser::ast::Statement::SettingsDef(s) = s {
                settings = s.clone();
            }
            if let crate::parser::ast::Statement::VarsDef(vars) = s {
                for (key, value) in vars {
                    env_vars.insert(key.clone(), value.as_string());
                }
            }
        }

        let terminal_backend = TerminalBackend::new(base_dir.clone(), settings.clone());
        let fs_backend = FileSystemBackend::new(base_dir);

        let test_cases: Vec<TestCase> = test_suite
            .statements
            .iter()
            .filter_map(|s| {
                if let crate::parser::ast::Statement::Scenario(sc) = s {
                    Some(sc.tests.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        let test_states: HashMap<String, TestState> = test_cases
            .iter()
            .map(|tc| (tc.name.clone(), TestState::Pending))
            .collect();

        Self {
            settings,
            terminal_backend,
            fs_backend,
            test_states,
            test_start_times: HashMap::new(),
            env_vars,
            last_exit_code: None,
            output_buffer: String::new(),
            verbose,
        }
    }

    pub fn run(
        &mut self,
        scenarios: &[Scenario],
    ) -> (HashMap<String, TestState>, HashMap<String, Instant>) {
        let test_timeout = Duration::from_secs(self.settings.timeout_seconds);

        'scenario_loop: for scenario in scenarios {
            colours::info(&format!("\nRunning scenario: '{}'", scenario.name));
            let scenario_start_time = Instant::now();

            loop {
                let elapsed_since_scenario_start = scenario_start_time.elapsed();
                let mut tests_to_start: Vec<(String, Vec<Action>)> = Vec::new();
                let mut tests_to_pass = Vec::new();
                let mut immediate_failures = Vec::new();

                // --- Checking Phase ---
                let tests_to_check: Vec<TestCase> = scenario
                    .tests
                    .iter()
                    .filter(|tc| !self.test_states.get(&tc.name).unwrap().is_done())
                    .cloned()
                    .collect();

                for test_case in &tests_to_check {
                    let current_state = self.test_states.get(&test_case.name).unwrap();
                    match current_state {
                        TestState::Pending => {
                            let (given_conditions, given_actions): (Vec<_>, Vec<_>) =
                                test_case.given.iter().partition_map(|step| match step {
                                    GivenStep::Condition(c) => itertools::Either::Left(c.clone()),
                                    GivenStep::Action(a) => itertools::Either::Right(a.clone()),
                                });

                            if !is_synchronous(test_case) {
                                self.terminal_backend
                                    .read_pty_output(&mut self.output_buffer);
                            }

                            if check_all_conditions_met(
                                "given",
                                &given_conditions,
                                &self.test_states,
                                &self.output_buffer,
                                &self.terminal_backend.last_stderr,
                                elapsed_since_scenario_start.as_secs_f32(),
                                &mut self.env_vars,
                                &self.last_exit_code,
                                &self.fs_backend,
                                self.verbose,
                            ) {
                                tests_to_start.push((test_case.name.clone(), given_actions));
                            }
                        }
                        TestState::Running => {
                            if !is_synchronous(test_case) {
                                self.terminal_backend
                                    .read_pty_output(&mut self.output_buffer);
                            }

                            if check_all_conditions_met(
                                "then",
                                &test_case.then,
                                &self.test_states,
                                &self.output_buffer,
                                &self.terminal_backend.last_stderr,
                                self.test_start_times
                                    .get(&test_case.name)
                                    .map_or(0.0, |start| start.elapsed().as_secs_f32()),
                                &mut self.env_vars,
                                &self.last_exit_code,
                                &self.fs_backend,
                                self.verbose,
                            ) {
                                tests_to_pass.push(test_case.name.clone());
                            } else if self
                                .test_start_times
                                .get(&test_case.name)
                                .map_or(false, |start| start.elapsed() > test_timeout)
                            {
                                immediate_failures.push((
                                    test_case.name.clone(),
                                    format!(
                                        "Test timed out after {} seconds",
                                        self.settings.timeout_seconds
                                    ),
                                ));
                            }
                        }
                        _ => {}
                    }
                }

                // --- Updating Phase ---
                self.update_test_states(
                    scenario,
                    tests_to_start,
                    tests_to_pass,
                    immediate_failures,
                );

                let all_done = scenario
                    .tests
                    .iter()
                    .all(|t| self.test_states.get(&t.name).unwrap().is_done());
                let any_pending = scenario
                    .tests
                    .iter()
                    .any(|t| matches!(self.test_states.get(&t.name).unwrap(), TestState::Pending));

                if all_done && !any_pending {
                    if !scenario.after.is_empty() {
                        colours::info("\nRunning after block...");
                        for action in &scenario.after {
                            let substituted_action =
                                substitute_variables_in_action(action, &self.env_vars);
                            self.execute_action(&substituted_action);
                        }
                    }
                    break;
                }

                if self.settings.stop_on_failure && self.test_states.values().any(|s| s.is_failed())
                {
                    for state in self.test_states.values_mut() {
                        if matches!(*state, TestState::Pending) {
                            *state = TestState::Skipped;
                        }
                    }
                    colours::error("\nStopping test run due to failure (stop_on_failure is true).");
                    break 'scenario_loop;
                }
            }
        }
        (self.test_states.clone(), self.test_start_times.clone())
    }

    fn update_test_states(
        &mut self,
        scenario: &Scenario,
        tests_to_start: Vec<(String, Vec<Action>)>,
        tests_to_pass: Vec<String>,
        immediate_failures: Vec<(String, String)>,
    ) {
        for (name, given_actions) in tests_to_start {
            let test_case = scenario.tests.iter().find(|tc| tc.name == name).unwrap();
            if is_synchronous(test_case) {
                self.run_sync_test(test_case, &given_actions);
            } else {
                self.run_async_test(test_case, &given_actions);
            }
        }

        for name in tests_to_pass {
            if let Some(state) = self.test_states.get_mut(&name) {
                if !state.is_done() {
                    *state = TestState::Passed;
                    colours::success(&format!(" 🟢  Test Passed: {}", name));
                }
            }
        }

        for (name, error_msg) in immediate_failures {
            if let Some(state) = self.test_states.get_mut(&name) {
                if !state.is_done() {
                    *state = TestState::Failed(error_msg.clone());
                    colours::error(&format!(" 🔴  Test Failed: {} - {}", name, error_msg));
                }
            }
        }
    }

    fn run_sync_test(&mut self, test_case: &TestCase, given_actions: &[Action]) {
        println!(" ▶️ Starting SYNC test: {}", test_case.name);
        self.test_states
            .insert(test_case.name.clone(), TestState::Running);
        self.test_start_times
            .insert(test_case.name.clone(), Instant::now());

        for given_action in given_actions {
            let substituted_action = substitute_variables_in_action(given_action, &self.env_vars);
            self.execute_action(&substituted_action);
            self.terminal_backend
                .read_pty_output(&mut self.output_buffer);
        }

        self.output_buffer.clear();

        for action in &test_case.when {
            let substituted_action = substitute_variables_in_action(action, &self.env_vars);
            self.execute_action(&substituted_action);
        }

        self.terminal_backend
            .read_pty_output(&mut self.output_buffer);

        let passed = check_all_conditions_met(
            "then",
            &test_case.then,
            &self.test_states,
            &self.output_buffer,
            &self.terminal_backend.last_stderr,
            self.test_start_times
                .get(&test_case.name)
                .map_or(0.0, |start| start.elapsed().as_secs_f32()),
            &mut self.env_vars,
            &self.last_exit_code,
            &self.fs_backend,
            self.verbose,
        );

        if let Some(state) = self.test_states.get_mut(&test_case.name) {
            if passed {
                *state = TestState::Passed;
                colours::success(&format!(" 🟢 Test Passed: {}", test_case.name));
            } else {
                let mut error_msg = "Synchronous test conditions not met".to_string();
                if !self.terminal_backend.last_stderr.is_empty() {
                    error_msg = format!(
                        "Synchronous test failed. Stderr: {}",
                        self.terminal_backend.last_stderr.trim()
                    );
                }
                *state = TestState::Failed(error_msg.clone());
                colours::error(&format!(
                    " 🔴 Test Failed: {} - {}",
                    test_case.name, error_msg
                ));
            }
        }
    }

    fn run_async_test(&mut self, test_case: &TestCase, given_actions: &[Action]) {
        if let Some(state) = self.test_states.get_mut(&test_case.name) {
            println!(" ▶  Starting ASYNC test: {}", test_case.name);
            *state = TestState::Running;
            self.test_start_times
                .insert(test_case.name.clone(), Instant::now());

            for given_action in given_actions {
                let substituted_action =
                    substitute_variables_in_action(given_action, &self.env_vars);
                self.execute_action(&substituted_action);
            }
            for action in &test_case.when {
                let substituted_action = substitute_variables_in_action(action, &self.env_vars);
                self.execute_action(&substituted_action);
            }
        }
    }

    fn execute_action(&mut self, action: &Action) {
        if self.terminal_backend.execute_action(
            action,
            &mut self.last_exit_code,
            Some(Duration::from_secs(self.settings.timeout_seconds)),
        ) {
            return;
        }
        if self.fs_backend.execute_action(action) {
            return;
        }
    }
}
