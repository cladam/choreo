use choreo::parser::ast::{Condition, Rule, Statement, TestSuite};
use choreo::backend::terminal_backend::TerminalBackend;
use choreo::{parser::ast, parser::parser};
use predicates::prelude::*;
use strip_ansi_escapes::strip;
use std::collections::{HashMap, HashSet};
use std::{env, fs};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    println!("Starting Choreo Test Runner...");

    // Read and parse the test file.
    let file_path = "src/example/medi_env_workflow.chor";
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
                    when: rule.when.iter().map(|c| substitute_variables_in_condition(c, &test_state)).collect(),
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

/// Helper to check if all `when` conditions for a rule are true.
fn check_all_conditions_met(
    rule: &Rule,
    succeeded_outcomes: &HashSet<String>,
    output_buffer: &str,
    current_time: f32,
    test_state: &mut HashMap<String, String>,
) -> bool {
    rule.when.iter().all(|condition| {
        check_condition(condition, succeeded_outcomes, output_buffer, current_time, test_state)
    })
}

/// Helper to check a single condition.
fn check_condition(
    condition: &Condition,
    succeeded_outcomes: &HashSet<String>,
    output_buffer: &str,
    current_time: f32,
    test_state: &mut HashMap<String, String>,
) -> bool {
    match condition {
        Condition::Time { op, time } => match op.as_str() {
            ">=" => current_time >= *time,
            "<=" => current_time <= *time,
            ">" => current_time > *time,
            "<" => current_time < *time,
            "==" => (current_time - *time).abs() < f32::EPSILON,
            _ => false,
        },
        Condition::OutputContains { text, .. } => {

            // First, strip all ANSI codes from the buffer.
            let cleaned_buffer = strip(output_buffer);
            let buffer = String::from_utf8_lossy(&cleaned_buffer).to_string();

            // Now, perform the check on the cleaned string.
            let predicate = predicate::str::contains(text.as_str());
            predicate.eval(&buffer)
        }
        Condition::OutputMatches { regex, actor: _, capture_as } => {
            let re = regex::Regex::new(regex).unwrap();

            // Check if there is a match AND a variable to capture.
            if let (Some(captures), Some(var_name)) = (re.captures(output_buffer), capture_as) {
                // The first capture group (at index 1) is what we want.
                if let Some(value) = captures.get(1) {
                    println!("  [CAPTURE] Captured '{}' into variable '{}'", value.as_str(), var_name);
                    test_state.insert(var_name.clone(), value.as_str().to_string());
                }
            }

            // The condition is true if the regex simply finds a match.
            re.is_match(output_buffer)
        }
        Condition::StateSucceeded { outcome } => { succeeded_outcomes.contains(outcome) },
    }
}

/// Helper to extract all defined outcome names from the AST.
fn get_all_outcomes(test_suite: &TestSuite) -> Vec<String> {
    test_suite
        .statements
        .iter()
        .find_map(|s| match s {
            Statement::OutcomeDef(outcomes) => Some(outcomes.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Creates a new Action with its string values substituted from the state map.
fn substitute_variables(action: &ast::Action, state: &HashMap<String, String>) -> ast::Action {
    match action {
        ast::Action::Type { actor, content } => ast::Action::Type {
            actor: actor.clone(),
            content: substitute_string(content, state),
        },
        ast::Action::Run { actor, command } => ast::Action::Run {
            actor: actor.clone(),
            command: substitute_string(command, state),
        },
        // Other actions that don't have strings can be cloned directly.
        _ => action.clone(),
    }
}

/// Finds and replaces all ${...} placeholders in a string.
fn substitute_string(content: &str, state: &HashMap<String, String>) -> String {
    let mut result = content.to_string();
    for (key, value) in state {
        let placeholder = format!("${{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

/// Creates a new Condition with its string values substituted from the state map.
fn substitute_variables_in_condition(
    condition: &Condition,
    state: &HashMap<String, String>,
) -> Condition {
    match condition {
        Condition::OutputContains { actor, text } => Condition::OutputContains {
            actor: actor.clone(),
            text: substitute_string(text, state),
        },
        Condition::OutputMatches { actor, regex, capture_as } => Condition::OutputMatches {
            actor: actor.clone(),
            regex: substitute_string(regex, state),
            capture_as: capture_as.clone(),
        },
        // Other conditions that don't have strings can be cloned directly.
        _ => condition.clone(),
    }
}