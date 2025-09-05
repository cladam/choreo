use choreo::backend::report::{TestCaseReport, TestStatus, TestSuiteReport};
use choreo::backend::terminal_backend::TerminalBackend;
use choreo::cli;
use choreo::cli::{Cli, Commands};
use choreo::colours;
use choreo::error::AppError;
use choreo::parser::ast::{Statement, TestCase};
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
                    Err(e) => return Err(AppError::ParseError(e.to_string())),
                };

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

                let mut terminal = TerminalBackend::new();
                let mut output_buffer = String::new();

                let test_cases: Vec<TestCase> = test_suite
                    .statements
                    .into_iter()
                    .filter_map(|s| match s {
                        Statement::TestCase(tc) => Some(tc),
                        _ => None,
                    })
                    .collect();

                let mut test_reports: HashMap<String, TestCaseReport> = test_cases
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
                let mut test_start_times: HashMap<String, Instant> = HashMap::new();

                let suite_start_time = Instant::now();
                let test_timeout = Duration::from_secs(30);

                loop {
                    terminal.read_output(&mut output_buffer, verbose);
                    let elapsed = suite_start_time.elapsed();

                    let mut tests_to_start = Vec::new();
                    let mut tests_to_pass = Vec::new();

                    // --- Checking Phase (Immutable Borrows) ---
                    for test_case in &test_cases {
                        let report = test_reports.get(&test_case.name).unwrap();
                        match report.status {
                            TestStatus::Skipped => {
                                if check_all_conditions_met(
                                    &test_case.given,
                                    &test_reports,
                                    &output_buffer,
                                    elapsed.as_secs_f32(),
                                    &mut env_vars,
                                    verbose,
                                ) {
                                    tests_to_start.push(test_case.name.clone());
                                }
                            }
                            TestStatus::Running => {
                                if check_all_conditions_met(
                                    &test_case.then,
                                    &test_reports,
                                    &output_buffer,
                                    elapsed.as_secs_f32(),
                                    &mut env_vars,
                                    verbose,
                                ) {
                                    tests_to_pass.push(test_case.name.clone());
                                }
                            }
                            _ => {}
                        }
                    }

                    // --- Updating Phase (Mutable Borrows) ---
                    for name in tests_to_start {
                        if let Some(report) = test_reports.get_mut(&name) {
                            let test_case = test_cases.iter().find(|tc| tc.name == name).unwrap();
                            if verbose {
                                colours::info(&format!("Starting test: {}", test_case.description));
                            }
                            test_start_times.insert(name.clone(), Instant::now());
                            for action in &test_case.when {
                                let substituted_a = substitute_variables(action, &env_vars);
                                terminal.execute_action(&substituted_a);
                            }
                            report.status = TestStatus::Running;
                        }
                    }

                    for name in tests_to_pass {
                        if let Some(report) = test_reports.get_mut(&name) {
                            let test_case = test_cases.iter().find(|tc| tc.name == name).unwrap();
                            if verbose {
                                colours::info(&format!("Test passed: {}", test_case.description));
                            }
                            report.status = TestStatus::Passed;
                            report.time = test_start_times.get(&name).unwrap().elapsed();
                        }
                    }

                    // --- Check for Completion ---
                    if test_reports
                        .values()
                        .all(|r| r.status == TestStatus::Passed || r.status == TestStatus::Failed)
                    {
                        if verbose {
                            colours::success("All tests completed.");
                        }
                        break;
                    }

                    if elapsed > test_timeout {
                        if verbose {
                            colours::error("Test run timed out.");
                        }
                        for (name, report) in test_reports.iter_mut() {
                            if matches!(report.status, TestStatus::Skipped | TestStatus::Running) {
                                report.status = TestStatus::Failed;
                                report.failure_message = Some("Timeout".to_string());
                                report.time = test_start_times
                                    .get(name)
                                    .map_or(test_timeout, |s| s.elapsed());
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

                generate_reports(&suite_name, suite_duration, final_reports, verbose)?;

                if verbose {
                    colours::success("Reports generated successfully.");
                }
                Ok(())
            }
        }
    }
}

/// Generates JSON and JUnit XML reports from the test results.
fn generate_reports(
    suite_name: &str,
    suite_duration: Duration,
    reports: Vec<TestCaseReport>,
    verbose: bool,
) -> Result<(), AppError> {
    let test_suite_report = TestSuiteReport {
        name: suite_name.to_string(),
        tests: reports.len(),
        failures: reports
            .iter()
            .filter(|r| r.status == TestStatus::Failed)
            .count(),
        time: suite_duration,
        testcases: reports,
    };

    // Generate JSON Report
    let json = serde_json::to_string_pretty(&test_suite_report)?;
    let mut json_file = File::create("report.json")?;
    json_file.write_all(json.as_bytes())?;
    if verbose {
        colours::info("JSON report content:");
        println!("{}", json);
    }

    // Generate JUnit XML Report
    /*    let mut xml_buffer = Vec::new();
        let mut writer = quick_xml::Writer::new_with_indent(&mut xml_buffer, b' ', 2);
        let mut serializer = quick_xml::se::Serializer::new(&mut writer);
        test_suite_report
            .serialize(&mut serializer)
            .map_err(|e| AppError::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;

        let mut xml_file = File::create("junit.xml")?;
        xml_file.write_all(&xml_buffer)?;
        colours::success("JUnit XML report generated at junit.xml");
    */
    Ok(())
}
