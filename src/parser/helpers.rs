use crate::backend::filesystem_backend::FileSystemBackend;
use crate::parser::ast::{Action, Condition, TestState};
use predicates::prelude::*;
use std::collections::HashMap;
use strip_ansi_escapes::strip;

/// Checks if all conditions in a list are met.
pub fn check_all_conditions_met(
    block_name: &str,
    conditions: &[Condition],
    test_states: &HashMap<String, TestState>,
    output_buffer: &str,
    stderr_buffer: &str,
    current_time: f32,
    env_vars: &mut HashMap<String, String>,
    last_exit_code: &Option<i32>,
    fs_backend: &FileSystemBackend,
    verbose: bool,
) -> bool {
    conditions.iter().all(|condition| {
        let substituted_c = substitute_variables_in_condition(condition, env_vars);
        let result = check_condition(
            &substituted_c,
            test_states,
            output_buffer,
            stderr_buffer,
            current_time,
            env_vars,
            last_exit_code,
            fs_backend,
            verbose,
        );
        if verbose {
            println!(
                "  [DEBUG] Checking {} condition: {:?} -> {}",
                block_name, substituted_c, result
            );
        }
        result
    })
}

/// Checks a single condition.
pub fn check_condition(
    condition: &Condition,
    test_states: &HashMap<String, TestState>,
    output_buffer: &str,
    stderr_buffer: &str,
    current_time: f32,
    env_vars: &mut HashMap<String, String>,
    last_exit_code: &Option<i32>,
    fs_backend: &FileSystemBackend,
    verbose: bool,
) -> bool {
    let cleaned_buffer = strip(output_buffer);
    let buffer = String::from_utf8_lossy(&cleaned_buffer);
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
            if verbose {
                println!("  [DEBUG] Checking output contains: '{}'", text);
            }
            predicate.eval(buffer.as_ref())
        }
        Condition::OutputMatches {
            regex, capture_as, ..
        } => {
            let re = regex::Regex::new(regex).unwrap();
            if let (Some(captures), Some(var_name)) = (re.captures(&buffer), capture_as) {
                if let Some(value) = captures.get(1) {
                    if verbose {
                        println!(
                            "  [CAPTURE] Captured '{}' into variable '{}'",
                            value.as_str(),
                            var_name
                        );
                    }
                    env_vars.insert(var_name.clone(), value.as_str().to_string());
                }
            }
            re.is_match(&buffer)
        }
        Condition::StateSucceeded { outcome } => test_states
            .get(outcome)
            .map_or(false, |s| *s == TestState::Passed),
        Condition::LastCommandSucceeded => last_exit_code.map_or(false, |code| code == 0),
        Condition::LastCommandFailed => last_exit_code.map_or(false, |code| code != 0),
        Condition::LastCommandExitCodeIs(expected_code) => {
            last_exit_code.map_or(false, |code| code == *expected_code)
        }
        Condition::FileExists { path } => {
            fs_backend.file_exists(&substitute_string(path, env_vars), verbose)
        }
        Condition::FileDoesNotExist { path } => {
            fs_backend.file_does_not_exist(&substitute_string(path, env_vars), verbose)
        }
        Condition::DirExists { path } => {
            fs_backend.dir_exists(&substitute_string(path, env_vars), verbose)
        }
        Condition::FileContains { path, content } => fs_backend.file_contains(
            &substitute_string(path, env_vars),
            &substitute_string(content, env_vars),
            verbose,
        ),
        Condition::StdoutIsEmpty => {
            let actual_output: Vec<&str> = buffer
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .filter(|line| {
                    let is_prompt = line.contains('%') || line.contains('$') || line.contains('>');
                    !is_prompt
                })
                .collect();
            println!("{}", actual_output.join("\n"));
            //println!("Actual output {}", actual_output);
            actual_output.is_empty()
        }
        Condition::StderrContains(text) => {
            // This is a simplification. A more robust implementation would need to
            // separate stdout and stderr from the terminal backend.
            // For now, we check the combined output buffer.
            println!("  [DEBUG] Checking stderr contains '{}'", text);
            println!("  [DEBUG] Stderr buffer: '{}'", stderr_buffer);
            stderr_buffer.contains(text)
        }
        Condition::OutputStartsWith(text) => {
            println!("  [DEBUG] Starting output starts with '{}'", text);
            println!("  [DEBUG] Checking the buffer '{}'", buffer);
            //buffer.trim_start().starts_with(text)
            buffer.lines().any(|line| line.trim().starts_with(text))
        }
        Condition::OutputEndsWith(text) => {
            println!("  [DEBUG] Checking output ends with '{}'", text);
            println!("  [DEBUG] Checking the buffer '{}'", buffer);
            buffer.lines().any(|line| line.trim().ends_with(text))
        }
        Condition::OutputEquals(text) => {
            println!("  [DEBUG] Checking output equals '{}'", text);
            println!("  [DEBUG] Checking the buffer '{}'", buffer);
            //buffer.lines().any(|line| line.trim() == text.as_str()
            buffer.lines().any(|line| line.trim() == text.trim())
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
        Action::CreateFile { path, content } => Action::CreateFile {
            path: substitute_string(path, state),
            content: substitute_string(content, state),
        },
        Action::DeleteFile { path } => Action::DeleteFile {
            path: substitute_string(path, state),
        },
        Action::CreateDir { path } => Action::CreateDir {
            path: substitute_string(path, state),
        },
        Action::DeleteDir { path } => Action::DeleteDir {
            path: substitute_string(path, state),
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
        Condition::FileExists { path } => Condition::FileExists {
            path: substitute_string(path, state),
        },
        Condition::FileDoesNotExist { path } => Condition::FileDoesNotExist {
            path: substitute_string(path, state),
        },
        Condition::DirExists { path } => Condition::DirExists {
            path: substitute_string(path, state),
        },
        Condition::FileContains { path, content } => Condition::FileContains {
            path: substitute_string(path, state),
            content: substitute_string(content, state),
        },
        Condition::StderrContains(text) => {
            Condition::StderrContains(substitute_string(text, state))
        }
        Condition::OutputStartsWith(text) => {
            Condition::OutputStartsWith(substitute_string(text, state))
        }
        Condition::OutputEndsWith(text) => {
            Condition::OutputEndsWith(substitute_string(text, state))
        }
        Condition::OutputEquals(text) => Condition::OutputEquals(substitute_string(text, state)),
        _ => condition.clone(),
    }
}

/// Creates a new Action with its string values substituted from the state map.
pub fn substitute_variables_in_action(action: &Action, state: &HashMap<String, String>) -> Action {
    match action {
        Action::Type { content, actor } => Action::Type {
            actor: actor.clone(),
            content: substitute_string(content, state),
        },
        Action::Run { command, actor } => Action::Run {
            actor: actor.clone(),
            command: substitute_string(command, state),
        },
        Action::CreateFile { path, content } => Action::CreateFile {
            path: substitute_string(path, state),
            content: substitute_string(content, state),
        },
        Action::DeleteFile { path } => Action::DeleteFile {
            path: substitute_string(path, state),
        },
        Action::CreateDir { path } => Action::CreateDir {
            path: substitute_string(path, state),
        },
        Action::DeleteDir { path } => Action::DeleteDir {
            path: substitute_string(path, state),
        },
        _ => action.clone(),
    }
}
