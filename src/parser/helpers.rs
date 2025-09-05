use crate::parser::ast::{Action, Condition, TestState};
use predicates::prelude::*;
use std::collections::HashMap;
use strip_ansi_escapes::strip;

/// Helper to check if all conditions in a list are true.
fn check_all_conditions_met(
    conditions: &[Condition],
    test_states: &HashMap<String, TestState>,
    output_buffer: &str,
    current_time: f32,
    env_vars: &mut HashMap<String, String>,
) -> bool {
    conditions.iter().all(|condition| {
        let substituted_c = substitute_variables_in_condition(condition, env_vars);
        check_condition(
            &substituted_c,
            test_states,
            output_buffer,
            current_time,
            env_vars,
        )
    })
}

/// Helper to check a single condition.
pub fn check_condition(
    condition: &Condition,
    test_states: &HashMap<String, TestState>,
    output_buffer: &str,
    current_time: f32,
    env_vars: &mut HashMap<String, String>,
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
            let buffer = String::from_utf8_lossy(&cleaned_buffer);
            let predicate = predicate::str::contains(text.as_str());
            predicate.eval(buffer.as_ref())
        }
        Condition::OutputMatches {
            regex, capture_as, ..
        } => {
            let re = regex::Regex::new(regex).unwrap();
            if let (Some(captures), Some(var_name)) = (re.captures(output_buffer), capture_as) {
                if let Some(value) = captures.get(1) {
                    println!(
                        "  [CAPTURE] Captured '{}' into variable '{}'",
                        value.as_str(),
                        var_name
                    );
                    env_vars.insert(var_name.clone(), value.as_str().to_string());
                }
            }
            re.is_match(output_buffer)
        }
        Condition::StateSucceeded { outcome } => {
            matches!(test_states.get(outcome), Some(&TestState::Passed))
        }
    }
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
fn substitute_string(content: &str, state: &HashMap<String, String>) -> String {
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
