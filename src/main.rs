use choreo::backend::terminal_backend::TerminalBackend;
use choreo::parser::ast::Statement;
use choreo::parser::helpers::*;
use choreo::{parser::ast, parser::parser};
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::{Duration, Instant};
use std::{env, fs};

fn main() {
    println!("Starting Choreo Test Runner...");

    // Read and parse the test file.
    let file_path = "examples/test_ls.chor";
    let source = fs::read_to_string(file_path).expect("Failed to read test file");
    let test_suite = match parser::parse(&source) {
        Ok(suite) => {
            println!("✅ Test suite parsed successfully.");
            suite
        }
        Err(e) => {
            eprintln!("❌ Parsing failed!\n{}", e);
            return;
        }
    };

    let mut test_state: HashMap<String, String> = HashMap::new();

    // Find the EnvDef statement and load the variables.
    for s in &test_suite.statements {
        if let Statement::EnvDef(vars) = s {
            for var_name in vars {
                let value = env::var(var_name)
                    .expect(&format!("Environment variable '{}' not set", var_name));
                test_state.insert(var_name.clone(), value);
            }
        }
    }

    // Initialise the backend and test state.
    let mut terminal = TerminalBackend::new();
    let mut output_buffer = String::new();
    let mut succeeded_outcomes: HashSet<String> = HashSet::new();
    let mut fired_rules: HashSet<String> = HashSet::new();

    // Extract all defined outcomes for the final report.
    let all_outcomes = get_all_outcomes(&test_suite);

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

        // Check each rule to see if it should fire.
        for s in &test_suite.statements {
            if let Statement::Rule(rule) = s {
                // Skip rules that have already fired.
                if fired_rules.contains(&rule.name) {
                    continue;
                }
                let substituted_rule = ast::Rule {
                    name: rule.name.clone(),
                    when: rule
                        .when
                        .iter()
                        .map(|c| substitute_variables_in_condition(c, &test_state))
                        .collect(),
                    then: rule.then.clone(), // `then` is substituted later, so we just clone it here.
                };

                // Check the conditions of the NEW substituted rule.
                if check_all_conditions_met(
                    &substituted_rule, // Use the substituted rule for checking
                    &succeeded_outcomes,
                    &output_buffer,
                    elapsed_secs,
                    &mut test_state,
                ) {
                    println!("🔥 Firing rule: {}", rule.name);
                    for action in &rule.then {
                        // Create a new, substituted action before executing.
                        let substituted_action = substitute_variables(action, &test_state);

                        // Use the new action
                        terminal.execute_action(&substituted_action);
                        if let ast::Action::Succeeds { outcome } = &substituted_action {
                            succeeded_outcomes.insert(outcome.clone());
                        }
                    }
                    fired_rules.insert(rule.name.clone());
                }
            }
        }

        // Check for completion or timeout.
        if succeeded_outcomes.len() == all_outcomes.len() {
            println!("\n🎉 All outcomes achieved!");
            break;
        }
        if elapsed > test_timeout {
            eprintln!(
                "\n⏰ Test timed out after {} seconds!",
                test_timeout.as_secs()
            );
            break;
        }

        // Sleep to prevent the loop from consuming 100% CPU.
        thread::sleep(Duration::from_millis(50));
    }

    // 4. Print the final report.
    println!("\n--- Test Report ---");
    for outcome in &all_outcomes {
        if succeeded_outcomes.contains(outcome) {
            println!("✅ PASSED: {}", outcome);
        } else {
            println!("❌ FAILED: {}", outcome);
        }
    }
    println!("-------------------");
    println!("Test run complete.");
}
