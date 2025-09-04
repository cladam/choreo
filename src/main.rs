use choreo::ast::{Condition, Rule, Statement, TestSuite};
use choreo::terminal_backend::TerminalBackend;
use choreo::{ast, parser};
use predicates::prelude::*;
use strip_ansi_escapes::strip;
use std::collections::HashSet;
use std::fs;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    println!("Starting Choreo Test Runner...");

    // Read and parse the test file.
    let file_path = "src/resources/test_medi_workflow.chor";
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
        for statement in &test_suite.statements {
            if let Statement::Rule(rule) = statement {
                // Skip rules that have already fired.
                if fired_rules.contains(&rule.name) {
                    continue;
                }
                if check_all_conditions_met(rule, &succeeded_outcomes, &output_buffer, elapsed_secs)
                {
                    println!("🔥 Firing rule: {}", rule.name);
                    for action in &rule.then {
                        terminal.execute_action(action);
                        if let ast::Action::Succeeds { outcome } = action {
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
) -> bool {
    rule.when.iter().all(|condition| {
        check_condition(condition, succeeded_outcomes, output_buffer, current_time)
    })
}

/// Helper to check a single condition.
fn check_condition(
    condition: &Condition,
    succeeded_outcomes: &HashSet<String>,
    output_buffer: &str,
    current_time: f32,
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
        // Condition::OutputMatches { regex, .. } => { ... }
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
