use choreo::cli;
use choreo::cli::{Cli, Commands};
use choreo::colours;
use choreo::error::AppError;
use choreo::parser::ast::{ReportFormat, Statement, TestState};
use choreo::parser::parser;
use choreo::reporting::generate_choreo_report;
use choreo::runner::TestRunner;
use clap::Parser;
use std::collections::HashMap;
use std::time::Instant;
use std::{env, fs};

fn main() {
    let cli = cli::Cli::parse();
    if let Err(e) = run(cli) {
        colours::error(&format!("Error: {}", e));
        std::process::exit(1);
    }
}

pub fn run(cli: Cli) -> Result<(), AppError> {
    match cli.command {
        Commands::Run { file, verbose } => {
            let suite_name = file.clone();
            if verbose {
                colours::info(&format!("Starting Choreo Test Runner: {}", file));
            }

            let source = fs::read_to_string(&file)?;
            let test_suite =
                parser::parse(&source).map_err(|e| AppError::PestParse(Box::new(e)))?;

            let test_file_path = std::path::Path::new(&file);
            let base_dir = test_file_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(""));

            let mut feature_name = "Choreo Test Feature".to_string();
            let mut env_vars: HashMap<String, String> = HashMap::new();
            let mut scenarios = Vec::new();
            let mut settings = Default::default();

            for s in &test_suite.statements {
                match s {
                    Statement::SettingsDef(s) => settings = s.clone(),
                    Statement::EnvDef(vars) => {
                        for var in vars {
                            let value =
                                env::var(var).map_err(|_| AppError::EnvVarNotFound(var.clone()))?;
                            env_vars.insert(var.clone(), value);
                        }
                    }
                    Statement::FeatureDef(name) => feature_name = name.clone(),
                    Statement::Scenario(scenario) => scenarios.push(scenario.clone()),
                    _ => {}
                }
            }

            let suite_start_time = Instant::now();
            let mut runner = TestRunner::new(
                &test_suite,
                base_dir.to_path_buf(),
                env_vars.clone(),
                verbose,
            );
            let (final_states, final_times) = runner.run(&scenarios);
            let suite_duration = suite_start_time.elapsed();

            print_summary(&suite_name, &final_states, suite_duration.as_secs_f32());

            if settings.report_format != ReportFormat::None {
                generate_choreo_report(
                    &suite_name,
                    suite_duration,
                    &feature_name,
                    &scenarios,
                    &final_states,
                    &final_times,
                    &env_vars,
                    &settings,
                    verbose,
                )?;
                if verbose {
                    colours::success("Reports generated successfully.");
                }
            }

            let failures = final_states.values().filter(|s| s.is_failed()).count();
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

fn print_summary(suite_name: &str, final_states: &HashMap<String, TestState>, duration_secs: f32) {
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for state in final_states.values() {
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
        final_states.len(),
        duration_secs,
        passed,
        failed,
        skipped
    ));
}
