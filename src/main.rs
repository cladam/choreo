use choreo::cli;
use choreo::cli::{Cli, Commands};
use choreo::colours;
use choreo::error::AppError;
use choreo::parser::ast::Statement;
use choreo::parser::parser;
use choreo::runner::TestRunner;
use clap::Parser;
use colored::Colorize;
use std::collections::HashMap;
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

            let mut env_vars: HashMap<String, String> = HashMap::new();
            let mut scenarios: Vec<choreo::parser::ast::Scenario> = Vec::new();
            let test_file_path = std::path::Path::new(&file);
            let base_dir = test_file_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(""));

            for s in &test_suite.statements {
                match s {
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
                    Statement::VarDef(key, value) => {
                        env_vars.insert(key.clone(), value.as_string());
                    }
                    Statement::Scenario(scenario) => scenarios.push(scenario.clone()),
                    _ => {} // Ignore other statement types
                }
            }

            let mut runner = TestRunner::new(
                test_suite,
                base_dir.to_path_buf(),
                env_vars.clone(),
                verbose,
            );

            // Call the runner and return its result
            runner.run(&suite_name, &scenarios)
        }
        Commands::Update => {
            println!("{}", "--- Checking for updates ---".blue());
            let status = self_update::backends::github::Update::configure()
                .repo_owner("cladam")
                .repo_name("choreo")
                .bin_name("choreo")
                .show_download_progress(true)
                .current_version(self_update::cargo_crate_version!())
                .build()?
                .update()?;

            println!("Update status: `{}`!", status.version());
            if status.updated() {
                println!("{}", "Successfully updated choreo!".green());
            } else {
                println!("{}", "choreo is already up to date.".green());
            }
            Ok(())
        }
    }
}
