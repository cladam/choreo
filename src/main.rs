use choreo::cli;
use choreo::cli::{Cli, Commands};
use choreo::colours;
use choreo::error::AppError;
use choreo::parser::ast::Statement;
use choreo::parser::{linter, parser};
use choreo::runner::TestRunner;
use clap::Parser;
use colored::Colorize;
use std::collections::HashMap;
use std::{env, fs};

const INIT_TEMPLATE: &str = r#"# A test suite for your application
feature "My Application Feature"

settings {
  # Stop running tests in a scenario after the first failure.
  stop_on_failure = true
  # Set a timeout for each test case.
  timeout_seconds = 10
}

# Define the actors that will perform actions.
actors: Terminal

# A scenario groups related tests into a single workflow.
scenario "User can perform a basic workflow" {
    # A test case with a unique name and description.
    test CheckAppVersion "Check application version" {
        given:
            # Conditions that must be met before the test runs.
            wait >= 0s
        when:
            # Actions to be performed.
            Terminal runs "echo 'my-app version 1.0.0'"
        then:
            # Conditions that must be true after the actions.
            Terminal last_command succeeded
            Terminal output_contains "my-app version"
    }

    test CreateAndCapture "Create a resource and capture its ID" {
        given:
            # This test depends on the success of the previous one.
            Test has_succeeded CheckAppVersion
        when:
            Terminal runs "echo 'Created resource with ID: res-123'"
        then:
            # Capture part of the output into a variable named 'resourceId'.
            Terminal output_matches "Created resource with ID: (res-\d+)" as resourceId
    }

    # This block runs after all tests in the scenario are complete.
    after {
        # Use the captured 'resourceId' variable to clean up.
        Terminal runs "echo 'Cleaning up ${resourceId}'"
    }
}
"#;

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
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| std::path::Path::new("."));

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
        Commands::Init { file } => {
            if std::path::Path::new(&file).exists() {
                colours::error(&format!(
                    "File '{}' already exists. Aborting to prevent overwrite.",
                    file
                ));
                return Ok(());
            }
            fs::write(&file, INIT_TEMPLATE)?;
            colours::success(&format!(
                "Successfully created example test file '{}'",
                file
            ));
            Ok(())
        }
        Commands::Validate { file } => {
            let source = fs::read_to_string(&file)?;
            match parser::parse(&source) {
                Ok(_) => {
                    colours::success("Test suite is valid.");
                    Ok(())
                }
                Err(e) => Err(AppError::ParseError(e.to_string())),
            }
        }
        Commands::Lint { file } => {
            let source = fs::read_to_string(&file)?;
            match parser::parse(&source) {
                Ok(suite) => {
                    let warnings = linter::lint(&suite);
                    if warnings.is_empty() {
                        colours::success("No linting issues found.");
                    } else {
                        colours::warn(&format!("Found {} linting issue(s):", warnings.len()));
                        for warning in warnings {
                            println!("- {}", warning);
                        }
                    }
                    Ok(())
                }
                Err(e) => Err(AppError::ParseError(e.to_string())),
            }
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
