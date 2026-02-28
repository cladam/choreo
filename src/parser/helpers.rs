use crate::backend::filesystem_backend::FileSystemBackend;
use crate::backend::system_backend::SystemBackend;
use crate::backend::terminal_backend::TerminalBackend;
use crate::backend::web_backend::WebBackend;
use crate::parser::ast::{
    Action, Condition, GivenStep, StateCondition, TaskArg, TaskCall, TestCase, TestState, ThenStep,
    Value, WhenStep,
};
use jsonpath_lib::selector;
use std::collections::HashMap;
use strip_ansi_escapes::strip;

/// Checks if all conditions in a list are met.
pub fn check_all_conditions_met(
    _block_name: &str,
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
    system_backend: &SystemBackend,
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
            system_backend,
            verbose,
        );
        // if verbose {
        //     println!(
        //         "  [DEBUG] Checking {} condition: {:?} -> {}",
        //         _block_name, substituted_c, result
        //     );
        // }
        result
    })
}

/// Extracts conditions from a list of ThenSteps (ignoring TaskCalls for now)
pub fn extract_conditions_from_then_steps(steps: &[ThenStep]) -> Vec<Condition> {
    steps
        .iter()
        .filter_map(|step| match step {
            ThenStep::Condition(c) => Some(c.clone()),
            ThenStep::TaskCall(_) => None, // Task calls should be expanded before checking
        })
        .collect()
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
    system_backend: &SystemBackend,
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
        Condition::OutputNotContains { text, .. } => {
            if verbose {
                println!(
                    "Checking if '{}' does NOT contain '{}'",
                    content_to_check, text
                );
            }
            !content_to_check.contains(text)
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
        | Condition::JsonPathCapture { .. }
        | Condition::JsonPathEquals { .. } => {
            web_backend.check_condition(condition, env_vars, verbose)
        }
        // --- System Conditions ---
        Condition::ServiceIsRunning { name } => {
            system_backend.check_service_is_running(name, verbose)
        }
        Condition::ServiceIsStopped { name } => {
            system_backend.check_service_is_stopped(name, verbose)
        }
        Condition::ServiceIsInstalled { name } => {
            system_backend.check_service_is_installed(name, verbose)
        }
        Condition::PortIsListening { port } => {
            system_backend.check_port_is_listening(*port, verbose)
        }
        Condition::PortIsClosed { port } => system_backend.check_port_is_closed(*port, verbose),
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

pub fn substitute_string(content: &str, state: &HashMap<String, String>) -> String {
    let mut result = content.to_string();

    // Handle array indexing like ${VAR[0]}
    let array_pattern = regex::Regex::new(r"\$\{([^}]+)\[(\d+)\]\}").unwrap();
    result = array_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            let index: usize = caps[2].parse().unwrap_or(0);

            if let Some(value) = state.get(var_name) {
                if let Ok(array) = serde_json::from_str::<Vec<serde_json::Value>>(value) {
                    array
                        .get(index)
                        .map(|v| v.as_str().unwrap_or_default().to_string())
                        .unwrap_or_default()
                } else {
                    caps[0].to_string()
                }
            } else {
                caps[0].to_string()
            }
        })
        .to_string();

    // Helper to resolve dotted/indexed paths like `user.name` or `products[0].id`
    fn resolve_path(expr: &str, state: &HashMap<String, String>) -> Option<String> {
        use serde_json::Value as JsonValue;

        // Find base var name (up to first '.' or '[')
        let mut i = 0usize;
        for (idx, ch) in expr.char_indices() {
            if ch == '.' || ch == '[' {
                i = idx;
                break;
            }
            i = idx + ch.len_utf8();
        }
        let base = if i == 0 && !expr.is_empty() {
            expr
        } else {
            &expr[..i]
        };

        let mut rest = if i < expr.len() { &expr[i..] } else { "" };

        // Lookup base in state
        let base_val = state.get(base)?;
        // Try parse base as JSON; if not JSON, only allow exact base (no further path)
        let mut current: JsonValue = if let Ok(j) = serde_json::from_str::<JsonValue>(base_val) {
            j
        } else {
            // if there's no rest (no path), return the raw base_val (unquote if it is quoted JSON string)
            if rest.is_empty() {
                if let Ok(s) = serde_json::from_str::<String>(base_val) {
                    return Some(s);
                } else {
                    return Some(base_val.clone());
                }
            } else {
                return None;
            }
        };

        // Walk the rest of the path
        while !rest.is_empty() {
            if rest.starts_with('.') {
                // field access
                rest = &rest[1..];
                // take field until next '.' or '[' or end
                let mut j = 0usize;
                for (idx, ch) in rest.char_indices() {
                    if ch == '.' || ch == '[' {
                        j = idx;
                        break;
                    }
                    j = idx + ch.len_utf8();
                }
                let field = if j == 0 && !rest.is_empty() {
                    rest
                } else {
                    &rest[..j]
                };
                rest = if j < rest.len() { &rest[j..] } else { "" };
                match &current {
                    JsonValue::Object(map) => {
                        if let Some(next) = map.get(field) {
                            current = next.clone();
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                }
            } else if rest.starts_with('[') {
                // array index access
                // find closing bracket
                if let Some(end_idx) = rest.find(']') {
                    let idx_str = &rest[1..end_idx];
                    let idx: usize = idx_str.parse().ok()?;
                    rest = &rest[end_idx + 1..];
                    match &current {
                        JsonValue::Array(arr) => {
                            if let Some(next) = arr.get(idx) {
                                current = next.clone();
                            } else {
                                return None;
                            }
                        }
                        _ => return None,
                    }
                } else {
                    return None;
                }
            } else {
                // unexpected token
                return None;
            }
        }

        // Convert final JsonValue to a string suitable for substitution
        match current {
            JsonValue::String(s) => Some(s),
            JsonValue::Number(n) => Some(n.to_string()),
            JsonValue::Bool(b) => Some(b.to_string()),
            JsonValue::Null => Some("null".to_string()),
            other => serde_json::to_string(&other).ok(),
        }
    }

    // Handle simple variable substitution like ${VAR} and complex expressions like ${user.name}
    let simple_pattern = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
    result = simple_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            let expr = &caps[1];

            // Try dotted/indexed resolution first
            if expr.contains('.') || expr.contains('[') {
                if let Some(resolved) = resolve_path(expr, state) {
                    return resolved;
                }
            }

            // Fallback: exact lookup or unquote JSON string
            if let Some(val) = state.get(expr) {
                if let Ok(s) = serde_json::from_str::<String>(val) {
                    s
                } else {
                    val.clone()
                }
            } else {
                caps[0].to_string()
            }
        })
        .to_string();

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
        Condition::OutputNotContains { actor, text } => Condition::OutputNotContains {
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
        Condition::ResponseBodyContains { value } => Condition::ResponseBodyContains {
            value: substitute_string(value, state),
        },
        Condition::ResponseBodyMatches { regex, capture_as } => Condition::ResponseBodyMatches {
            regex: substitute_string(regex, state),
            capture_as: capture_as.clone(),
        },
        Condition::ResponseBodyEqualsJson { expected, ignored } => {
            Condition::ResponseBodyEqualsJson {
                expected: substitute_string(expected, state),
                ignored: ignored
                    .iter()
                    .map(|f| substitute_string(f, state))
                    .collect(),
            }
        }
        Condition::JsonBodyHasPath { path } => Condition::JsonBodyHasPath {
            path: substitute_string(path, state),
        },
        Condition::JsonPathEquals {
            path,
            expected_value,
        } => Condition::JsonPathEquals {
            path: substitute_string(path, state),
            expected_value: substitute_value(expected_value, state),
        },
        Condition::JsonPathCapture { path, capture_as } => Condition::JsonPathCapture {
            path: substitute_string(path, state),
            capture_as: capture_as.clone(),
        },
        // JSON value type checks (paths)
        Condition::JsonValueIsString { path } => Condition::JsonValueIsString {
            path: substitute_string(path, state),
        },
        Condition::JsonValueIsNumber { path } => Condition::JsonValueIsNumber {
            path: substitute_string(path, state),
        },
        Condition::JsonValueIsArray { path } => Condition::JsonValueIsArray {
            path: substitute_string(path, state),
        },
        Condition::JsonValueIsObject { path } => Condition::JsonValueIsObject {
            path: substitute_string(path, state),
        },
        Condition::JsonValueHasSize { path, size } => Condition::JsonValueHasSize {
            path: substitute_string(path, state),
            size: *size,
        },
        // --- System Conditions ---
        Condition::ServiceIsRunning { name } => Condition::ServiceIsRunning {
            name: substitute_string(name, state),
        },
        Condition::ServiceIsStopped { name } => Condition::ServiceIsStopped {
            name: substitute_string(name, state),
        },
        Condition::ServiceIsInstalled { name } => Condition::ServiceIsInstalled {
            name: substitute_string(name, state),
        },
        Condition::PortIsListening { port } => Condition::PortIsListening { port: *port },
        Condition::PortIsClosed { port } => Condition::PortIsClosed { port: *port },
        // fallback
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
        Action::SetCwd { path } => Action::SetCwd {
            path: substitute_string(path, state),
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

/// Creates a new TaskArg with its string values substituted from the state map.
pub fn substitute_variables_in_task_arg(arg: &TaskArg, state: &HashMap<String, String>) -> TaskArg {
    match arg {
        TaskArg::String(s) => TaskArg::String(substitute_string(s, state)),
        TaskArg::VariableRef(v) => {
            // Resolve the variable reference
            let resolved = substitute_string(v, state);
            TaskArg::String(resolved)
        }
        TaskArg::Number(n) => TaskArg::Number(*n),
        TaskArg::Duration(d) => TaskArg::Duration(*d),
    }
}

/// Creates a new TaskCall with its arguments substituted from the state map.
pub fn substitute_variables_in_task_call(
    task_call: &TaskCall,
    state: &HashMap<String, String>,
) -> TaskCall {
    TaskCall {
        name: task_call.name.clone(),
        arguments: task_call
            .arguments
            .iter()
            .map(|arg| substitute_variables_in_task_arg(arg, state))
            .collect(),
    }
}

/// Creates a new GivenStep with its values substituted from the state map.
pub fn substitute_variables_in_given_step(
    step: &GivenStep,
    state: &HashMap<String, String>,
) -> GivenStep {
    match step {
        GivenStep::Action(a) => GivenStep::Action(substitute_variables_in_action(a, state)),
        GivenStep::Condition(c) => {
            GivenStep::Condition(substitute_variables_in_condition(c, state))
        }
        GivenStep::TaskCall(tc) => {
            GivenStep::TaskCall(substitute_variables_in_task_call(tc, state))
        }
    }
}

/// Creates a new WhenStep with its values substituted from the state map.
pub fn substitute_variables_in_when_step(
    step: &WhenStep,
    state: &HashMap<String, String>,
) -> WhenStep {
    match step {
        WhenStep::Action(a) => WhenStep::Action(substitute_variables_in_action(a, state)),
        WhenStep::TaskCall(tc) => WhenStep::TaskCall(substitute_variables_in_task_call(tc, state)),
    }
}

/// Creates a new ThenStep with its values substituted from the state map.
pub fn substitute_variables_in_then_step(
    step: &ThenStep,
    state: &HashMap<String, String>,
) -> ThenStep {
    match step {
        ThenStep::Condition(c) => ThenStep::Condition(substitute_variables_in_condition(c, state)),
        ThenStep::TaskCall(tc) => ThenStep::TaskCall(substitute_variables_in_task_call(tc, state)),
    }
}

/// Creates a new TestCase with its string values substituted from the state map.
pub fn substitute_variables_in_test_case(
    test_case: &TestCase,
    state: &HashMap<String, String>,
) -> TestCase {
    TestCase {
        name: substitute_string(&test_case.name, state),
        description: substitute_string(&test_case.description, state),
        given: test_case
            .given
            .iter()
            .map(|step| substitute_variables_in_given_step(step, state))
            .collect(),
        when: test_case
            .when
            .iter()
            .map(|step| substitute_variables_in_when_step(step, state))
            .collect(),
        then: test_case
            .then
            .iter()
            .map(|step| substitute_variables_in_then_step(step, state))
            .collect(),
        span: test_case.span.clone(),
        testcase_spans: test_case.testcase_spans.clone(),
    }
}

fn substitute_value(v: &Value, state: &HashMap<String, String>) -> Value {
    match v {
        Value::String(s) => Value::String(substitute_string(s, state)),
        Value::Array(arr) => Value::Array(arr.iter().map(|x| substitute_value(x, state)).collect()),
        _ => v.clone(),
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
    // Check `given` (which contains GivenStep) and `when` (which contains WhenStep).
    let given_has_async = test_case.given.iter().any(|gs| match gs {
        GivenStep::Action(a) => action_is_async(a),
        GivenStep::Condition(_) => false,
        GivenStep::TaskCall(_) => false, // Task calls are expanded before execution
    });

    let when_has_async = test_case.when.iter().any(|ws| match ws {
        WhenStep::Action(a) => action_is_async(a),
        WhenStep::TaskCall(_) => false, // Task calls are expanded before execution
    });

    // If any async action is present, the test is asynchronous.
    !(given_has_async || when_has_async)
}

/// Determines if a test case contains only synchronous actions.
pub fn is_synchronous(test_case: &TestCase) -> bool {
    test_case.when.iter().all(|step| match step {
        WhenStep::Action(action) => matches!(
            action,
            Action::Run { .. }
                | Action::SetCwd { .. }
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
        ),
        WhenStep::TaskCall(_) => true, // Task calls are expanded before execution
    })
}
