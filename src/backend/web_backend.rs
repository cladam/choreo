use crate::parser::ast::{Action, Condition, Value};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// State of the last web request made.
#[derive(Debug, Clone, Default)]
pub struct LastResponse {
    pub status: StatusCode,
    pub body: String,
}

/// The backend responsible for handling web-based actions and conditions.
#[derive(Debug)]
pub struct WebBackend {
    client: Client,
    pub last_response: Option<LastResponse>,
}

impl WebBackend {
    /// Creates a new WebBackend with a persistent HTTP client.
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            last_response: None,
        }
    }

    /// Executes a single web-related action from the AST.
    pub fn execute_action(
        &mut self,
        action: &Action,
        variables: &HashMap<String, String>,
        verbose: bool,
    ) {
        // Clear the previous response before making a new request.
        self.last_response = None;

        let response = match action {
            Action::HttpGet { url, .. } => {
                let substituted_url = substitute_string(url, variables);
                if verbose {
                    println!("  [DEBUG] Performing HTTP GET: {}", substituted_url);
                }
                self.client.get(&substituted_url).send()
            }
            // Action::HttpPost { url, body, .. } => {
            //     let substituted_url = substitute_string(url, variables);
            //     let substituted_body = body
            //         .as_ref()
            //         .map_or(String::new(), |b| substitute_string(b, variables));
            //     if verbose {
            //         println!("  [DEBUG] Performing HTTP POST: {}", substituted_url);
            //         println!("  [DEBUG] Post body: {}", substituted_body);
            //     }
            //     self.client
            //         .post(&substituted_url)
            //         .body(substituted_body)
            //         .send()
            // }
            _ => return, // Not a web action
        };

        match response {
            Ok(res) => {
                let status = res.status();
                let body = res.text().unwrap_or_default();
                if verbose {
                    println!("  [DEBUG] Response status: {}", status);
                    println!("  [DEBUG] Response body: {}", body);
                }
                self.last_response = Some(LastResponse { status, body });
            }
            Err(e) => {
                let error_message = format!("[WEB_BACKEND] HTTP request failed: {}", e);
                println!("{}", error_message);
                // Store a failure state in the last response
                self.last_response = Some(LastResponse {
                    status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                    body: error_message,
                });
            }
        }
    }

    pub fn execute_action2(
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

                let response = self.client.get(&substituted_url).send();

                match response {
                    Ok(res) => {
                        let status = res.status();
                        let body = res.text().unwrap_or_default();
                        if verbose {
                            println!("[WEB_BACKEND] Response: Status={}, Body={}", status, body);
                        }
                        self.last_response = Some(LastResponse { status, body });
                        true
                    }
                    Err(e) => {
                        let error_message = format!("[WEB_BACKEND] HTTP request failed: {}", e);
                        println!("{}", error_message);
                        self.last_response = Some(LastResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: error_message,
                        });
                        false
                    }
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
                last_response.status.as_u16() == *expected_status
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
