use crate::parser::ast::{Action, Condition, Value};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use ureq::http::StatusCode;
use ureq::Agent;

/// State of the last web request made.
#[derive(Debug, Clone, Default)]
pub struct LastResponse {
    pub status: StatusCode,
    pub body: String,
}

/// The backend responsible for handling web-based actions and conditions.
#[derive(Debug)]
pub struct WebBackend {
    agent: Agent,
    pub last_response: Option<LastResponse>,
}

impl WebBackend {
    /// Creates a new WebBackend with a persistent HTTP client.
    pub fn new() -> Self {
        Self {
            agent: Agent::new_with_defaults(),
            last_response: None,
        }
    }

    /// Executes a single web-related action. Returns true if the action was handled.
    pub fn execute_action(
        &mut self,
        action: &Action,
        env_vars: &HashMap<String, String>,
        verbose: bool,
    ) -> bool {
        match action {
            Action::HttpGet { url, .. } => {
                let substituted_url = substitute_string(url, env_vars);
                if verbose {
                    println!("[WEB_BACKEND] Performing HTTP GET to: {}", substituted_url);
                }

                let response_result = self.agent.get(&substituted_url).call();

                match response_result {
                    Ok(response) => {
                        let status = response.status();
                        let body = {
                            let mut body_reader = response.into_body();
                            let body = String::new();
                            match body_reader.read_to_string() {
                                Ok(_) => body,
                                Err(e) => {
                                    let error_message = format!(
                                        "[WEB_BACKEND] Failed to read response body: {}",
                                        e
                                    );
                                    println!("{}", error_message);
                                    error_message
                                }
                            }
                        };
                        if verbose {
                            println!("[WEB_BACKEND] Received response status: {}", status);
                        }

                        self.last_response = Some(LastResponse { status, body });
                        true
                    }
                    Err(e) => match &e {
                        ureq::Error::StatusCode(code) => {
                            if verbose {
                                println!(
                                    "[WEB_BACKEND] HTTP request returned error status: {}",
                                    code
                                );
                            }
                            self.last_response = Some(LastResponse {
                                status: StatusCode::from_u16(*code)
                                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                                body: format!("HTTP error with status code: {}", code),
                            });
                            true
                        }
                        _ => {
                            let error_message = format!("[WEB_BACKEND] HTTP request failed: {}", e);
                            if verbose {
                                println!("{}", error_message);
                            }
                            self.last_response = Some(LastResponse {
                                status: StatusCode::INTERNAL_SERVER_ERROR,
                                body: error_message,
                            });
                            false
                        }
                    },
                }
            }
            _ => false,
        }
    }

    /// Checks a single web-related condition against the last response.
    pub fn check_condition(
        &self,
        condition: &Condition,
        variables: &mut HashMap<String, String>,
    ) -> bool {
        // If no request has been made yet, all web conditions fail.
        let last_response = match &self.last_response {
            Some(res) => res,
            None => return false,
        };

        match condition {
            Condition::ResponseStatusIs(expected_status) => {
                last_response.status == *expected_status
            }
            Condition::ResponseBodyContains { value } => last_response.body.contains(value),
            Condition::ResponseBodyMatches { regex, capture_as } => {
                if let Ok(re) = regex::Regex::new(regex) {
                    if let Some(captures) = re.captures(&last_response.body) {
                        if let Some(var_name) = capture_as {
                            if let Some(capture_group) = captures.get(1) {
                                let value = capture_group.as_str().to_string();
                                variables.insert(var_name.clone(), value);
                            }
                        }
                        return true;
                    }
                }
                false
            }
            Condition::JsonBodyHasPath { path } => {
                // Try to parse the body as JSON. If it fails, the condition fails.
                if let Ok(json_body) = serde_json::from_str::<JsonValue>(&last_response.body) {
                    // Use `pointer` to navigate the JSON structure.
                    // The path must be in JSON Pointer format (e.g., "/user/id").
                    json_body.pointer(path).is_some()
                } else {
                    false
                }
            }
            Condition::JsonPathEquals {
                path,
                expected_value,
            } => {
                if let Ok(json_body) = serde_json::from_str::<JsonValue>(&last_response.body) {
                    // Find the value at the specified path.
                    if let Some(actual_value) = json_body.pointer(path) {
                        // Compare the actual value with the expected value.
                        // This is a simplified comparison; a real implementation
                        // would handle different types (numbers, booleans, etc.).
                        return &Value::String(actual_value.to_string()) == expected_value;
                    }
                }
                false
            }
            _ => false, // Not a web condition
        }
    }
}

/// Simple helper for variable substitution in URLs.
fn substitute_string(content: &str, state: &HashMap<String, String>) -> String {
    let mut result = content.to_string();
    for (key, value) in state {
        let placeholder = format!("${{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}
