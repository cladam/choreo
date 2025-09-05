use crate::parser::ast::{Action, Condition, Rule, Statement, TestSuite};
use predicates::prelude::*;
use std::collections::{HashMap, HashSet};
use strip_ansi_escapes::strip;

/// Helper to check if all `when` conditions for a rule are true.
pub fn check_all_conditions_met(
    rule: &Rule,
    succeeded_outcomes: &HashSet<String>,
    output_buffer: &str,
    current_time: f32,
    test_state: &mut HashMap<String, String>,
) -> bool {
    rule.when.iter().all(|condition| {
        check_condition(
            condition,
            succeeded_outcomes,
            output_buffer,
            current_time,
            test_state,
        )
    })
}

/// Helper to check a single condition.
pub fn check_condition(
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
            let cleaned_buffer = strip(output_buffer);
            let buffer = String::from_utf8_lossy(&cleaned_buffer).to_string();
            let predicate = predicate::str::contains(text.as_str());
            predicate.eval(&buffer)
        }
        Condition::OutputMatches {
            regex,
            actor: _,
            capture_as,
        } => {
            let re = regex::Regex::new(regex).unwrap();
            if let (Some(captures), Some(var_name)) = (re.captures(output_buffer), capture_as) {
                if let Some(value) = captures.get(1) {
                    println!(
                        "  [CAPTURE] Captured '{}' into variable '{}'",
                        value.as_str(),
                        var_name
                    );
                    test_state.insert(var_name.clone(), value.as_str().to_string());
                }
            }
            re.is_match(output_buffer)
        }
        Condition::StateSucceeded { outcome } => succeeded_outcomes.contains(outcome),
    }
}

/// Helper to extract all defined outcome names from the AST.
pub fn get_all_outcomes(test_suite: &TestSuite) -> Vec<String> {
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
pub fn substitute_variables(action: &Action, state: &HashMap<String, String>) -> Action {
    match action {
        Action::Type { actor, content } => Action::Type {
            actor: actor.clone(),
            content: substitute_string(content, state),
        },
        Action::Run { actor, command } => Action::Run {
            actor: actor.clone(),
            command: substitute_string(command, state),
        },
        _ => action.clone(),
    }
}

/// Finds and replaces all ${...} placeholders in a string.
pub fn substitute_string(content: &str, state: &HashMap<String, String>) -> String {
    let mut result = content.to_string();
    for (key, value) in state {
        let placeholder = format!("${{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

/// Creates a new Condition with its string values substituted from the state map.
pub fn substitute_variables_in_condition(
    condition: &Condition,
    state: &HashMap<String, String>,
) -> Condition {
    match condition {
        Condition::OutputContains { actor, text } => Condition::OutputContains {
            actor: actor.clone(),
            text: substitute_string(text, state),
        },
        Condition::OutputMatches {
            actor,
            regex,
            capture_as,
        } => Condition::OutputMatches {
            actor: actor.clone(),
            regex: substitute_string(regex, state),
            capture_as: capture_as.clone(),
        },
        _ => condition.clone(),
    }
}
