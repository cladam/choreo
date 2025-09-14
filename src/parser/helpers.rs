use crate::backend::filesystem_backend::FileSystemBackend;
use crate::parser::ast::{Action, Condition, TestCase, TestState};
use std::collections::HashMap;
use strip_ansi_escapes::strip;

/// Checks if all conditions in a list are met.
pub fn check_all_conditions_met(
    block_name: &str,
    conditions: &[Condition],
    test_states: &HashMap<String, TestState>,
    output_buffer: &str,
    stderr_buffer: &str,
    current_wait: f32,
    env_vars: &mut HashMap<String, String>,
    last_exit_code: &Option<i32>,
    fs_backend: &FileSystemBackend,
    terminal_cwd: &std::path::Path,
    verbose: bool,
) -> bool {
    conditions.iter().all(|condition| {
        let substituted_c = substitute_variables_in_condition(condition, env_vars);
        let result = check_condition(
            &substituted_c,
            test_states,
            output_buffer,
            stderr_buffer,
            current_wait,
            env_vars,
            last_exit_code,
            fs_backend,
            terminal_cwd,
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
    current_wait: f32,
    env_vars: &mut HashMap<String, String>,
    last_exit_code: &Option<i32>,
    fs_backend: &FileSystemBackend,
    terminal_cwd: &std::path::Path,
    verbose: bool,
) -> bool {
    let cleaned_buffer = strip(output_buffer);
    let buffer = String::from_utf8_lossy(&cleaned_buffer);

    match condition {
        Condition::Wait { op, wait } => match op.as_str() {
            ">=" => current_wait >= *wait,
            "<=" => current_wait <= *wait,
            ">" => current_wait > *wait,
            "<" => current_wait < *wait,
            "==" => (current_wait - *wait).abs() < f32::EPSILON,
            _ => false,
        },
        Condition::OutputContains { text, .. } => {
            if verbose {
                println!(
                    "  [DEBUG](OutputContains) Checking output contains: '{}'",
                    text
                );
                println!("  [DEBUG](OutputContains) Raw buffer: '{}'", buffer);
            }
            let actual_output: Vec<&str> =
                buffer.lines().filter_map(extract_command_output).collect();

            let filtered_output = actual_output.join("\n");
            if verbose {
                println!("  [DEBUG] Filtered output: '{}'", filtered_output);
            }
            filtered_output.contains(text)
        }
        Condition::OutputMatches {
            regex, capture_as, ..
        } => {
            let actual_output: Vec<&str> =
                buffer.lines().filter_map(extract_command_output).collect();
            let filtered_output = actual_output.join("\n");

            if filtered_output.is_empty() {
                println!("  [DEBUG] No command output detected after filtering");
                return false;
            }

            match regex::Regex::new(regex) {
                Ok(re) => {
                    if let Some(captures) = re.captures(&filtered_output) {
                        if let Some(var_name) = capture_as {
                            // The first capture group (index 1) is the one we want.
                            if let Some(capture) = captures.get(1) {
                                let value = capture.as_str().to_string();
                                env_vars.insert(var_name.clone(), value);
                            }
                        }
                        true
                    } else {
                        println!("  [DEBUG] Regex did not match the filtered output");
                        false
                    }
                }
                Err(e) => {
                    println!("  [DEBUG] Invalid regex: {}", e);
                    false
                }
            }
        }
        Condition::StateSucceeded { outcome } => test_states
            .get(outcome)
            .is_some_and(|s| *s == TestState::Passed),
        Condition::LastCommandSucceeded => {
            if verbose {
                println!(
                    "  [DEBUG](LastCommandSucceeded) Last exit code: {:?}",
                    last_exit_code
                );
            }
            *last_exit_code == Some(0)
        }
        Condition::LastCommandFailed => last_exit_code.is_some_and(|code| code != 0),
        Condition::LastCommandExitCodeIs(expected_code) => *last_exit_code == Some(*expected_code),
        Condition::FileExists { path } => {
            fs_backend.file_exists(&substitute_string(path, env_vars), terminal_cwd, verbose)
        }
        Condition::FileDoesNotExist { path } => fs_backend.file_does_not_exist(
            &substitute_string(path, env_vars),
            terminal_cwd,
            verbose,
        ),
        Condition::FileIsEmpty { path } => {
            let resolved_path =
                fs_backend.resolve_path(&substitute_string(path, env_vars), terminal_cwd);
            if verbose {
                println!("Checking if file is empty: {}", resolved_path.display());
            }
            resolved_path.is_file()
                && resolved_path
                    .metadata()
                    .map(|m| m.len() == 0)
                    .unwrap_or(false)
        }
        Condition::FileIsNotEmpty { path } => {
            let resolved_path =
                fs_backend.resolve_path(&substitute_string(path, env_vars), terminal_cwd);
            if verbose {
                println!("Checking if file is not empty: {}", resolved_path.display());
            }
            resolved_path.is_file()
                && resolved_path
                    .metadata()
                    .map(|m| m.len() > 0)
                    .unwrap_or(false)
        }
        Condition::DirExists { path } => {
            fs_backend.dir_exists(&substitute_string(path, env_vars), terminal_cwd, verbose)
        }
        Condition::DirDoesNotExist { path } => {
            fs_backend.dir_does_not_exist(&substitute_string(path, env_vars), terminal_cwd, verbose)
        }
        Condition::FileContains { path, content } => fs_backend.file_contains(
            &substitute_string(path, env_vars),
            &substitute_string(content, env_vars),
            terminal_cwd,
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
            //println!("{}", actual_output.join("\n"));
            //println!("Actual output {}", actual_output);
            actual_output.is_empty()
        }
        Condition::StderrIsEmpty => {
            let stderr_cleaned = strip(stderr_buffer);
            let stderr_buffer = String::from_utf8_lossy(&stderr_cleaned);
            //println!("stderr: {}", stderr_buffer);
            stderr_buffer.trim().is_empty()
        }
        Condition::StderrContains(text) => stderr_buffer.contains(text),
        Condition::OutputStartsWith(text) => {
            let actual_output: Vec<&str> =
                buffer.lines().filter_map(extract_command_output).collect();
            let filtered_output = actual_output.join("\n");
            filtered_output.starts_with(text)
        }
        Condition::OutputEndsWith(text) => {
            let actual_output: Vec<&str> =
                buffer.lines().filter_map(extract_command_output).collect();
            let filtered_output = actual_output.join("\n");
            filtered_output.ends_with(text)
        }
        Condition::OutputEquals(text) => {
            let actual_output: Vec<&str> =
                buffer.lines().filter_map(extract_command_output).collect();

            actual_output.iter().any(|line| *line == text.trim())
        }
        Condition::OutputIsValidJson => {
            // For sync tests, the buffer contains the direct output.
            // For async, it might be mixed. We look for a valid JSON object
            // within the output.
            let content_to_check = buffer.trim();
            if serde_json::from_str::<serde_json::Value>(content_to_check).is_ok() {
                return true;
            }

            // Fallback for async or mixed content: find JSON within the lines.
            let actual_output: Vec<&str> =
                buffer.lines().filter_map(extract_command_output).collect();
            let filtered_output = actual_output.join("\n");
            serde_json::from_str::<serde_json::Value>(&filtered_output).is_ok()
        }
        Condition::JsonOutputHasPath { path } => {
            let actual_output: Vec<&str> =
                buffer.lines().filter_map(extract_command_output).collect();
            let filtered_output = actual_output.join("\n");
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&filtered_output) {
                !jsonpath_lib::select(&json, path)
                    .unwrap_or_default()
                    .is_empty()
            } else {
                false
            }
        }
        _ => false, // Other conditions not implemented yet
    }
}

// Helper to extract actual command output from a buffer line
fn extract_command_output(line: &str) -> Option<&str> {
    // Common prompt patterns: "$", "%", ">"
    let prompt_patterns = ["$", "%", ">"];
    let mut last_prompt_idx = None;

    for pat in &prompt_patterns {
        if let Some(idx) = line.rfind(pat) {
            // Check if it's a plausible prompt ending
            if (idx + pat.len() == line.len() || line[idx + pat.len()..].starts_with(' '))
                && last_prompt_idx.is_none_or(|(i, _)| idx > i)
            {
                last_prompt_idx = Some((idx, pat.len()));
            }
        }
    }

    if let Some((idx, pat_len)) = last_prompt_idx {
        // A prompt was found. Return only what comes after it.
        let after = &line[idx + pat_len..].trim();
        return if !after.is_empty() {
            Some(after)
        } else {
            // Prompt was found, but nothing followed it. Filter this line out.
            None
        };
    }

    // No prompt was found. Return the whole line if it's not empty.
    let trimmed_line = line.trim();
    if !trimmed_line.is_empty() {
        Some(trimmed_line)
    } else {
        None
    }
}

/// Creates a new Action with its string values substituted from the state map.
pub fn substitute_variables(action: &Action, state: &HashMap<String, String>) -> Action {
    match action {
        Action::Type { actor, content } => Action::Type {
            actor: actor.clone(),
            content: substitute_string(content, state),
        },
        Action::Run { actor, command } => {
            println!(
                "  [DEBUG] Substituting in Run action: command='{}'",
                command
            );
            Action::Run {
                actor: actor.clone(),
                command: substitute_string(command, state),
            }
        }
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

/// Determines if a test case contains only synchronous actions.
pub fn is_synchronous(test_case: &TestCase) -> bool {
    test_case.when.iter().all(|action| {
        matches!(
            action,
            Action::Run { .. }
                | Action::CreateFile { .. }
                | Action::DeleteFile { .. }
                | Action::CreateDir { .. }
                | Action::DeleteDir { .. }
        )
    })
}
