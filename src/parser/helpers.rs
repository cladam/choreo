use crate::backend::filesystem_backend::FileSystemBackend;
use crate::backend::terminal_backend::TerminalBackend;
use crate::backend::web_backend::WebBackend;
use crate::parser::ast::{Action, Condition, GivenStep, StateCondition, TestCase, TestState};
use jsonpath_lib::selector;
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
    terminal_backend: &mut TerminalBackend,
    web_backend: &WebBackend,
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
            terminal_backend,
            web_backend,
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
    terminal_backend: &mut TerminalBackend,
    web_backend: &WebBackend,
    verbose: bool,
) -> bool {
    let cleaned_buffer = strip(output_buffer);
    let buffer = String::from_utf8_lossy(&cleaned_buffer);

    // For synchronous commands, the output is in `last_stdout`.
    // For asynchronous commands, it's in the PTY `output_buffer`.
    let content_to_check = if !terminal_backend.last_stdout.is_empty() {
        terminal_backend.last_stdout.as_str()
    } else {
        buffer.as_ref()
    };

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
                println!("Checking if '{}' contains '{}'", content_to_check, text);
            }
            content_to_check.contains(text)
        }
        Condition::OutputMatches {
            regex, capture_as, ..
        } => {
            if let Ok(re) = regex::Regex::new(regex) {
                if let Some(captures) = re.captures(content_to_check) {
                    if let Some(var_name) = capture_as {
                        if let Some(capture_group) = captures.get(1) {
                            let value = capture_group.as_str().to_string();
                            if verbose {
                                println!(
                                    "  [DEBUG] Captured value '{}' into variable '{}'",
                                    value, var_name
                                );
                            }
                            env_vars.insert(var_name.clone(), value);
                            // Clear last_stdout after successful capture to prevent reuse
                            terminal_backend.last_stdout.clear();
                        }
                    }
                    return true;
                }
            }
            false
        }
        Condition::State(StateCondition::HasSucceeded(outcome)) => test_states
            .get(outcome)
            .is_some_and(|s| *s == TestState::Passed),
        Condition::State(StateCondition::CanStart) => true,
        Condition::LastCommandSucceeded => {
            if verbose {
                println!("Checking if last command succeeded: {:?}", last_exit_code);
            }
            *last_exit_code == Some(0)
        }
        Condition::LastCommandFailed => last_exit_code.is_some_and(|code| code != 0),
        Condition::LastCommandExitCodeIs(expected_code) => *last_exit_code == Some(*expected_code),
        Condition::FileExists { path } => fs_backend.file_exists(
            &substitute_string(path, env_vars),
            terminal_backend.get_cwd(),
            verbose,
        ),
        Condition::FileDoesNotExist { path } => fs_backend.file_does_not_exist(
            &substitute_string(path, env_vars),
            terminal_backend.get_cwd(),
            verbose,
        ),
        Condition::FileIsEmpty { path } => {
            let resolved_path = fs_backend.resolve_path(
                &substitute_string(path, env_vars),
                terminal_backend.get_cwd(),
            );
            if verbose {
                println!("Checking if file is empty: {:?}", resolved_path);
            }
            resolved_path.is_file()
                && std::fs::metadata(resolved_path)
                    .map(|m| m.len() == 0)
                    .unwrap_or(false)
        }
        Condition::FileIsNotEmpty { path } => {
            let resolved_path = fs_backend.resolve_path(
                &substitute_string(path, env_vars),
                terminal_backend.get_cwd(),
            );
            if verbose {
                println!("Checking if file is not empty: {:?}", resolved_path);
            }
            resolved_path.is_file()
                && std::fs::metadata(resolved_path)
                    .map(|m| m.len() > 0)
                    .unwrap_or(false)
        }
        Condition::DirExists { path } => fs_backend.dir_exists(
            &substitute_string(path, env_vars),
            terminal_backend.get_cwd(),
            verbose,
        ),
        Condition::DirDoesNotExist { path } => fs_backend.dir_does_not_exist(
            &substitute_string(path, env_vars),
            terminal_backend.get_cwd(),
            verbose,
        ),
        Condition::FileContains { path, content } => fs_backend.file_contains(
            &substitute_string(path, env_vars),
            &substitute_string(content, env_vars),
            terminal_backend.get_cwd(),
            verbose,
        ),
        Condition::StdoutIsEmpty => content_to_check.trim().is_empty(),
        Condition::StderrIsEmpty => {
            let stderr_cleaned = strip(stderr_buffer);
            let stderr_buffer = String::from_utf8_lossy(&stderr_cleaned);
            //println!("stderr: {}", stderr_buffer);
            stderr_buffer.trim().is_empty()
        }
        Condition::StderrContains(text) => stderr_buffer.contains(text),
        Condition::OutputStartsWith(text) => content_to_check.trim().starts_with(text),
        Condition::OutputEndsWith(text) => content_to_check.trim().ends_with(text),
        Condition::OutputEquals(text) => content_to_check.trim() == text.trim(),
        Condition::OutputIsValidJson => {
            serde_json::from_str::<serde_json::Value>(content_to_check.trim()).is_ok()
        }
        Condition::JsonOutputHasPath { path } => {
            let json_obj = match serde_json::from_str::<serde_json::Value>(content_to_check.trim())
            {
                Ok(obj) => obj,
                Err(_) => return false,
            };

            if verbose {
                println!("Checking if JSON has path: {}", path);
            }

            let mut selector = selector(&json_obj);
            match selector(path) {
                Ok(nodes) => !nodes.is_empty(),
                Err(_) => false,
            }
        }
        Condition::ResponseStatusIs(_)
        | Condition::ResponseStatusIsSuccess
        | Condition::ResponseStatusIsError
        | Condition::ResponseStatusIsIn(_)
        | Condition::ResponseTimeIsBelow { .. }
        | Condition::ResponseBodyContains { .. }
        | Condition::ResponseBodyMatches { .. }
        | Condition::ResponseBodyEqualsJson { .. }
        | Condition::JsonValueIsString { .. }
        | Condition::JsonValueIsNumber { .. }
        | Condition::JsonValueIsArray { .. }
        | Condition::JsonValueIsObject { .. }
        | Condition::JsonValueHasSize { .. }
        | Condition::JsonBodyHasPath { .. }
        | Condition::JsonPathEquals { .. } => {
            web_backend.check_condition(condition, env_vars, verbose)
        }
        _ => false, // Other conditions not implemented yet
    }
}

/// Creates a new Action with its string values substituted from the state map.
pub fn _substitute_variables(action: &Action, state: &HashMap<String, String>) -> Action {
    match action {
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
        Action::ReadFile { path, variable } => Action::ReadFile {
            path: substitute_string(path, state),
            variable: variable.clone(),
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
        Action::Run { command, actor } => Action::Run {
            actor: actor.clone(),
            command: substitute_string(command, state),
        },
        Action::Log { message } => Action::Log {
            message: substitute_string(message, state),
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
        Action::ReadFile { path, variable } => Action::ReadFile {
            path: substitute_string(path, state),
            variable: variable.clone(),
        },
        Action::HttpGet { url } => Action::HttpGet {
            url: substitute_string(url, state),
        },
        Action::HttpPost { url, body } => Action::HttpPost {
            url: substitute_string(url, state),
            body: substitute_string(body, state),
        },
        Action::HttpPut { url, body } => Action::HttpPut {
            url: substitute_string(url, state),
            body: substitute_string(body, state),
        },
        Action::HttpPatch { url, body } => Action::HttpPatch {
            url: substitute_string(url, state),
            body: substitute_string(body, state),
        },
        Action::HttpDelete { url } => Action::HttpDelete {
            url: substitute_string(url, state),
        },
        Action::HttpSetHeader { key, value } => Action::HttpSetHeader {
            key: substitute_string(key, state),
            value: substitute_string(value, state),
        },
        Action::HttpClearHeader { key } => Action::HttpClearHeader {
            key: substitute_string(key, state),
        },
        Action::HttpSetCookie { key, value } => Action::HttpSetCookie {
            key: substitute_string(key, state),
            value: substitute_string(value, state),
        },
        Action::HttpClearCookie { key } => Action::HttpClearCookie {
            key: substitute_string(key, state),
        },
        _ => action.clone(),
    }
}

/// Returns true if the given action should be treated as *asynchronous*
fn action_is_async(action: &Action) -> bool {
    match action {
        // Treat HTTP request actions as non-blocking for test orchestration
        Action::HttpGet { .. }
        | Action::HttpPost { .. }
        | Action::HttpPut { .. }
        | Action::HttpPatch { .. }
        | Action::HttpDelete { .. } => true,

        // Treat shell Run commands that end with '&' as async (background jobs).
        Action::Run { command, .. } => {
            let trimmed = command.trim();
            // If the command ends with '&' (allowing whitespace before it) treat as async.
            trimmed.ends_with('&')
        }

        // Other actions considered synchronous by default.
        _ => false,
    }
}

/// Determines whether an entire test case should be executed synchronously.
/// Returns `true` when the test contains no async actions; `false` otherwise.
pub fn is_synchronous_new(test_case: &TestCase) -> bool {
    // Check `given` (which contains GivenStep) and `when` (which contains Action).
    let given_has_async = test_case.given.iter().any(|gs| match gs {
        GivenStep::Action(a) => action_is_async(a),
        GivenStep::Condition(_) => false,
    });

    let when_has_async = test_case.when.iter().any(|a| action_is_async(a));

    // If any async action is present, the test is asynchronous.
    !(given_has_async || when_has_async)
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
                | Action::ReadFile { .. }
                | Action::HttpGet { .. }
                | Action::HttpPost { .. }
                | Action::HttpPut { .. }
                | Action::HttpPatch { .. }
                | Action::HttpDelete { .. }
                | Action::HttpSetHeader { .. }
                | Action::HttpClearHeader { .. }
                | Action::HttpClearHeaders
                | Action::HttpSetCookie { .. }
                | Action::HttpClearCookie { .. }
                | Action::HttpClearCookies
        )
    })
}
