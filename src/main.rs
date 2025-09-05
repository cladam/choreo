use choreo::backend::terminal_backend::TerminalBackend;
use choreo::cli;
use choreo::cli::{Cli, Commands};
use choreo::colours;
use choreo::error::AppError;
use choreo::parser::ast::{Statement, TestCase, TestState};
use choreo::parser::helpers::*;
use choreo::parser::parser;
use clap::Parser;
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::{Duration, Instant};
use std::{env, fs};

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
                colours::info(&format!("Starting Choreo Test Runner: {}", file));
                // Read and parse the test file.
                if verbose {
                    colours::info("Verbose mode enabled.");
                }
                // Read the test file.
                if !fs::metadata(&file).is_ok() {
                    return Err(AppError::FileNotFound(file));
                }

                let source = fs::read_to_string(file)?;
                let test_suite = match parser::parse(&source) {
                    Ok(suite) => {
                        colours::success("Test suite parsed successfully.");
                        suite
                    }
                    Err(e) => {
                        return Err(AppError::ParseError(e.to_string()));
                    }
                };

                // Find the EnvDef statement and load the variables.
                let mut env_vars: HashMap<String, String> = HashMap::new();
                for s in &test_suite.statements {
                    if let Statement::EnvDef(vars) = s {
                        for var_name in vars {
                            let value = env::var(var_name)
                                .expect(&format!("Environment variable '{}' not set", var_name));
                            env_vars.insert(var_name.clone(), value);
                        }
                    }
                }

                // Initialise the backend and test state.
                let mut terminal = TerminalBackend::new();
                let mut output_buffer = String::new();

                // --- Test State Management ---
                let mut test_cases: Vec<TestCase> = test_suite
                    .statements
                    .into_iter()
                    .filter_map(|s| match s {
                        Statement::TestCase(tc) => Some(tc),
                        _ => None,
                    })
                    .collect();
                let mut test_states: HashMap<String, TestState> = test_cases
                    .iter()
                    .map(|tc| (tc.name.clone(), TestState::Pending))
                    .collect();

                // Run the main simulation loop.
                let start_time = Instant::now();
                let test_timeout = Duration::from_secs(30); // Max duration for the test.

                // Loop until timeout or all outcomes succeeded.
                loop {
                    // Read any new input from the terminal.
                    terminal.read_output(&mut output_buffer);
                    let elapsed = start_time.elapsed();
                    let elapsed_secs = elapsed.as_secs_f32();
                    //println!("Elapsed time: {:.2} seconds", elapsed_secs);

                    let mut all_tests_done = true;
                    // Check each rule to see if it should fire.
                    for test_case in &mut test_cases {
                        let current_state = test_states.get(&test_case.name).unwrap().clone();

                        match current_state {
                            TestState::Pending => {
                                all_tests_done = false;
                                let given_conditions_met = test_case.given.iter().all(|c| {
                                    let substituted_c =
                                        substitute_variables_in_condition(c, &env_vars);
                                    check_condition(
                                        &substituted_c,
                                        &HashSet::new(),
                                        &output_buffer,
                                        elapsed_secs,
                                        &mut env_vars,
                                    )
                                });

                                if given_conditions_met {
                                    println!("▶️ Starting test: {}", test_case.description);
                                    for action in &test_case.when {
                                        let substituted_a = substitute_variables(action, &env_vars);
                                        terminal.execute_action(&substituted_a);
                                    }
                                    test_states.insert(test_case.name.clone(), TestState::Running);
                                }
                            }
                            TestState::Running => {
                                all_tests_done = false;
                                let then_conditions_met = test_case.then.iter().all(|c| {
                                    let substituted_c =
                                        substitute_variables_in_condition(c, &env_vars);
                                    check_condition(
                                        &substituted_c,
                                        &HashSet::new(),
                                        &output_buffer,
                                        elapsed_secs,
                                        &mut env_vars,
                                    )
                                });

                                if then_conditions_met {
                                    println!("✅ Test Passed: {}", test_case.description);
                                    test_states.insert(test_case.name.clone(), TestState::Passed);
                                }
                            }
                            _ => {} // Test is already Passed or Failed, do nothing.
                        }
                    }

                    if all_tests_done {
                        println!("\n🎉 All tests completed!");
                        break;
                    }
                    if elapsed > test_timeout {
                        eprintln!("\n⏰ Test run timed out!");
                        let to_fail: Vec<String> = test_states
                            .iter()
                            .filter_map(|(name, state)| {
                                if matches!(state, TestState::Running | TestState::Pending) {
                                    Some(name.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        for name in to_fail {
                            test_states.insert(name, TestState::Failed("Timeout".to_string()));
                        }
                        break;
                    }

                    thread::sleep(Duration::from_millis(50));
                }

                // --- New Final Report ---
                println!("\n--- Test Report ---");
                for test_case in &test_cases {
                    match test_states.get(&test_case.name) {
                        Some(TestState::Passed) => println!("✅ PASSED: {}", test_case.description),
                        Some(TestState::Failed(reason)) => {
                            println!("❌ FAILED: {} ({})", test_case.description, reason)
                        }
                        _ => println!("❔ SKIPPED: {}", test_case.description),
                    }
                }
                println!("-------------------");
                println!("Test run complete.");
                Ok(())
            }
        }
    }
}
