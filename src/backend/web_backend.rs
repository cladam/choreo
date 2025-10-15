use crate::parser::ast::{Action, Condition, Value};
use crate::parser::helpers::{substitute_string, substitute_variables_in_action};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Display};
use ureq::http::{Response, StatusCode};
use ureq::{Agent, Body};

#[derive(Debug)]
enum CompatResult {
    Success(Response<Body>),
    ClientError(Response<Body>),
    ServerError(Response<Body>),
    TransportError(ureq::Error),
}

impl Display for CompatResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompatResult::Success(response) => {
                write!(
                    f,
                    "HTTP request succeeded with status {}",
                    response.status()
                )
            }
            CompatResult::ClientError(response) => {
                write!(f, "HTTP client error: {}", response.status())
            }
            CompatResult::ServerError(response) => {
                write!(f, "HTTP server error: {}", response.status())
            }
            CompatResult::TransportError(error) => {
                write!(f, "HTTP transport error: {}", error)
            }
        }
    }
}

/// State of the last web request made.
#[derive(Debug, Clone, Default)]
pub struct LastResponse {
    pub status: StatusCode,
    pub body: String,
    pub message: Option<String>,
    pub response_time_ms: u128,
}

/// The backend responsible for handling web-based actions and conditions.
#[derive(Debug)]
pub struct WebBackend {
    agent: Agent,
    headers: HashMap<String, String>,
    pub last_response: Option<LastResponse>,
}

impl WebBackend {
    pub fn with_headers(headers: HashMap<String, String>) -> Self {
        let mut wb = WebBackend::new();
        for (k, v) in headers.into_iter() {
            wb.set_header(&k, &v);
        }
        wb
    }

    pub fn get_headers(&self) -> HashMap<String, String> {
        // Adjust to match the actual internal representation (this assumes a HashMap field named headers)
        self.headers.clone()
    }

    pub fn set_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }
    /// Creates a new WebBackend with a persistent HTTP client.
    pub fn new() -> Self {
        let config = Agent::config_builder().http_status_as_error(false).build();
        let agent: Agent = config.into();
        Self {
            agent,
            headers: HashMap::new(),
            last_response: None,
        }
    }

    /// Executes a single web-related action. Returns true if the action was handled.
    pub fn execute_action(
        &mut self,
        action: &Action,
        env_vars: &mut HashMap<String, String>,
        verbose: bool,
    ) -> bool {
        self.last_response = None;
        let substituted_action = substitute_variables_in_action(action, env_vars);
        let start_time = std::time::Instant::now();
        let result: Result<Response<Body>, ureq::Error> = match &substituted_action {
            Action::HttpSetHeader { key, value } => {
                if verbose {
                    println!("[WEB_BACKEND] Setting HTTP header: {}: {}", key, value);
                }
                self.headers.insert(key.clone(), value.clone());

                // This isn't a request but need to return a response
                let response = Response::builder()
                    .status(200)
                    .body(Body::builder().data("choreo"));
                Ok(response.expect("hmm"))
            }
            Action::HttpClearHeader { key } => {
                if verbose {
                    println!("[WEB_BACKEND] Clearing HTTP header: {}", key);
                }
                self.headers.remove(&*key);
                // This isn't a request but need to return a response
                let response = Response::builder()
                    .status(200)
                    .body(Body::builder().data("choreo"));
                Ok(response.expect("hmm"))
            }
            Action::HttpClearHeaders => {
                if verbose {
                    println!("[WEB_BACKEND] Clearing all HTTP headers");
                }
                self.headers.clear();
                // This isn't a request but need to return a response
                let response = Response::builder()
                    .status(200)
                    .body(Body::builder().data("choreo"));
                Ok(response.expect("hmm"))
            }
            Action::HttpSetCookie { key, value } => {
                // Handle multiple cookies by appending to existing Cookie header
                let new_cookie = format!("{}={}", key, value);
                match self.headers.get("Cookie") {
                    Some(existing) => {
                        let updated_cookies = format!("{}; {}", existing, new_cookie);
                        self.headers.insert("Cookie".to_string(), updated_cookies);
                    }
                    None => {
                        self.headers.insert("Cookie".to_string(), new_cookie);
                    }
                }

                if verbose {
                    println!("[WEB_BACKEND] Added cookie: {}={}", key, value);
                    println!(
                        "[WEB_BACKEND] Current Cookie header: {}",
                        self.headers.get("Cookie").unwrap_or(&"".to_string())
                    );
                }
                // This isn't a request but need to return a response
                let response = Response::builder()
                    .status(200)
                    .body(Body::builder().data("choreo"));
                Ok(response.expect("hmm"))
            }
            Action::HttpClearCookie { key } => {
                if let Some(cookie_header) = self.headers.get("Cookie") {
                    // Parse and filter out the specific cookie
                    let cookies: Vec<&str> = cookie_header.split(';').collect();
                    let filtered_cookies: Vec<&str> = cookies
                        .into_iter()
                        .filter(|cookie| {
                            let cookie_trimmed = cookie.trim();
                            !cookie_trimmed.starts_with(&format!("{}=", key))
                        })
                        .collect();

                    if filtered_cookies.is_empty() {
                        self.headers.remove("Cookie");
                    } else {
                        let new_cookie_header = filtered_cookies.join("; ");
                        self.headers.insert("Cookie".to_string(), new_cookie_header);
                    }
                }

                if verbose {
                    println!("[WEB_BACKEND] Cleared cookie: {}", key);
                }
                // This isn't a request but need to return a response
                let response = Response::builder()
                    .status(200)
                    .body(Body::builder().data("choreo"));
                Ok(response.expect("hmm"))
            }
            Action::HttpClearCookies => {
                if verbose {
                    println!("[WEB_BACKEND] Clearing all HTTP cookies");
                }
                self.headers.remove("Cookie");
                // This isn't a request but need to return a response
                let response = Response::builder()
                    .status(200)
                    .body(Body::builder().data("choreo"));
                Ok(response.expect("hmm"))
            }
            Action::HttpGet { url, .. } => {
                if verbose {
                    println!("[WEB_BACKEND] Performing HTTP GET to: {}", url);
                }

                let mut request = self.agent.get(url);

                // Add headers
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                request.call()
            }
            Action::HttpPost { url, body } => {
                if verbose {
                    println!("[WEB_BACKEND] Performing HTTP POST to: {}", url);
                }

                let mut request = self.agent.post(url);

                // Add headers
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                request.send(body)
            }
            Action::HttpPut { url, body } => {
                if verbose {
                    println!("[WEB_BACKEND] Performing HTTP PUT to: {}", url);
                }

                let mut request = self.agent.put(url);

                // Add headers
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                request.send(body)
            }
            Action::HttpPatch { url, body } => {
                if verbose {
                    println!("[WEB_BACKEND] Performing HTTP PATCH to: {}", url);
                }

                let mut request = self.agent.patch(url);

                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                request.send(body)
            }
            Action::HttpDelete { url } => {
                if verbose {
                    println!("[WEB_BACKEND] Performing HTTP DELETE to: {}", url);
                }

                let mut request = self.agent.delete(url);

                // Add headers
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                request.call()
            }
            _ => return false,
        };

        let compat_result = match result {
            Ok(response) => {
                let status = response.status();
                match status.as_u16() {
                    200..=299 => CompatResult::Success(response),
                    400..=499 => CompatResult::ClientError(response),
                    500..=599 => CompatResult::ServerError(response),
                    _ => CompatResult::Success(response), // Handle other cases like redirects if needed
                }
            }
            Err(e) => CompatResult::TransportError(e),
        };

        let mut process_response = |response: Response<Body>, message: String| {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();

            let body = response
                .into_body()
                .read_to_string()
                .unwrap_or_else(|e| format!("[choreo] Failed to read response body: {}", e));

            let body_json = if content_type.contains("application/json") {
                // Pretty print JSON for better readability
                serde_json::from_str::<serde_json::Value>(&body)
                    .map(|v| serde_json::to_string_pretty(&v).unwrap_or(body.clone()))
                    .unwrap_or(body.clone())
            } else {
                body
            };

            let response_time_ms = start_time.elapsed().as_millis();
            self.last_response = Some(LastResponse {
                status,
                body: body_json.clone(),
                message: Some(message.to_string()),
                response_time_ms,
            });
        };

        match compat_result {
            CompatResult::Success(response) => {
                // 200 success
                let status = response.status();
                let message = format!("HTTP request succeeded with status {}", status);
                process_response(response, message);
            }
            CompatResult::ClientError(response) => {
                // client error (4xx)
                let status = response.status();
                let message = format!("HTTP client error: {}", status);
                process_response(response, message);
            }
            CompatResult::ServerError(response) => {
                // server error (5xx)
                let status = response.status();
                let message = format!("HTTP server error: {}", status);
                process_response(response, message);
            }
            CompatResult::TransportError(e) => {
                // Transport-level errors.
                let error_message = format!("[WEB_BACKEND] HTTP request failed: {}", e);
                self.last_response = Some(LastResponse {
                    status: StatusCode::from_u16(599).unwrap(),
                    body: error_message.clone(),
                    response_time_ms: 0,
                    message: Some(error_message),
                });
            }
        }
        true
    }

    /// Checks a single web-related condition against the last response.
    pub fn check_condition(
        &self,
        condition: &Condition,
        variables: &mut HashMap<String, String>,
        verbose: bool,
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
            Condition::ResponseStatusIsSuccess => last_response.status.is_success(),
            Condition::ResponseStatusIsError => {
                last_response.status.is_client_error() || last_response.status.is_server_error()
            }
            Condition::ResponseStatusIsIn(statuses) => {
                statuses.contains(&last_response.status.as_u16())
            }
            Condition::ResponseTimeIsBelow { duration } => {
                if let Some(last_response) = &self.last_response {
                    let actual_time_seconds = last_response.response_time_ms as f32 / 1000.0;
                    let result = duration > &actual_time_seconds;
                    if verbose {
                        println!(
                            "[WEB_BACKEND] Response time: {}ms ({:.3}s), expected below: {:.3}s -> {}",
                            last_response.response_time_ms,
                            actual_time_seconds,
                            duration,
                            result
                        );
                    }
                    result
                } else {
                    false
                }
            }
            Condition::ResponseBodyContains { value } => {
                if verbose {
                    println!("[WEB_BACKEND] Received response body contains '{}'", value);
                    println!("[WEB_BACKEND] Full response body: {}", last_response.body);
                }
                last_response.body.contains(value)
            }
            Condition::ResponseBodyMatches { regex, capture_as } => {
                if let Ok(re) = regex::Regex::new(regex) {
                    if let Some(captures) = re.captures(&last_response.body) {
                        //println!("Regexp: {}", captures.get(0).unwrap().as_str());
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
            Condition::ResponseBodyEqualsJson { expected, ignored } => {
                if self.last_response.is_none() {
                    if verbose {
                        println!("[WEB_BACKEND] No response available for JSON comparison");
                    }
                    return false;
                }
                // Substitute variables in the expected JSON string
                let substituted_expected = substitute_string(expected, variables);
                // Parse both the response body and expected JSON for comparison
                match (
                    serde_json::from_str::<JsonValue>(&last_response.body),
                    serde_json::from_str::<JsonValue>(&expected),
                ) {
                    (Ok(mut actual), Ok(mut expected_json)) => {
                        if verbose {
                            println!(
                                "[WEB_BACKEND] Comparing JSON response body with expected JSON"
                            );
                        }
                        // Remove ignored fields from both actual and expected JSON values
                        for field in ignored {
                            remove_json_field_recursive(&mut actual, field);
                            remove_json_field_recursive(&mut expected_json, field);
                        }

                        // Normalise both JSON values by serializing and re-parsing
                        // for consistent field ordering and formatting. Apparently many JVM libraries do unordered json....
                        let actual_normalised = serde_json::to_string(&actual)
                            .and_then(|s| serde_json::from_str::<JsonValue>(&s));
                        let expected_normalised = serde_json::to_string(&expected_json)
                            .and_then(|s| serde_json::from_str::<JsonValue>(&s));

                        let result = match (actual_normalised, expected_normalised) {
                            (Ok(actual_norm), Ok(expected_norm)) => actual_norm == expected_norm,
                            _ => actual == expected_json, // Fallback to direct comparison
                        };

                        if !result && verbose {
                            println!("[WEB_BACKEND] JSON comparison failed");
                            println!(
                                "[WEB_BACKEND] Actual (after ignoring fields): {}",
                                serde_json::to_string_pretty(&actual).unwrap_or_default()
                            );
                            println!(
                                "[WEB_BACKEND] Expected (after ignoring fields): {}",
                                serde_json::to_string_pretty(&expected_json).unwrap_or_default()
                            );
                        }
                        result
                    }
                    (Err(e), _) => {
                        if verbose {
                            println!("[WEB_BACKEND] Failed to parse response body as JSON: {}", e);
                            println!("[WEB_BACKEND] Response body: {}", last_response.body);
                        }
                        false
                    }
                    (_, Err(e)) => {
                        if verbose {
                            println!("[WEB_BACKEND] Failed to parse expected JSON: {}", e);
                            println!("[WEB_BACKEND] Expected JSON: {}", substituted_expected);
                        }
                        false
                    }
                }
            }
            Condition::JsonValueIsString { path } => {
                if let Ok(json_body) = serde_json::from_str::<JsonValue>(&last_response.body) {
                    if let Some(value) = json_body.pointer(path) {
                        return value.is_string();
                    }
                }
                false
            }
            Condition::JsonValueIsNumber { path } => {
                if let Ok(json_body) = serde_json::from_str::<JsonValue>(&last_response.body) {
                    if let Some(value) = json_body.pointer(path) {
                        return value.is_number();
                    }
                }
                false
            }
            Condition::JsonValueIsArray { path } => {
                if let Ok(json_body) = serde_json::from_str::<JsonValue>(&last_response.body) {
                    if let Some(value) = json_body.pointer(path) {
                        return value.is_array();
                    }
                }
                false
            }
            Condition::JsonValueIsObject { path } => {
                if let Ok(json_body) = serde_json::from_str::<JsonValue>(&last_response.body) {
                    if let Some(value) = json_body.pointer(path) {
                        return value.is_object();
                    }
                }
                false
            }
            Condition::JsonValueHasSize { path, size } => {
                if let Ok(json_body) = serde_json::from_str::<JsonValue>(&last_response.body) {
                    if let Some(value) = json_body.pointer(path) {
                        return match value {
                            JsonValue::Array(arr) => arr.len() == *size,
                            JsonValue::String(s) => s.len() == *size,
                            JsonValue::Object(obj) => obj.len() == *size,
                            _ => false,
                        };
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
                    if let Some(actual_value) = json_body.pointer(path) {
                        // Convert the serde_json::Value to our AST Value for comparison.
                        let our_value = match actual_value {
                            JsonValue::String(s) => Value::String(s.clone()),
                            JsonValue::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0) as i32),
                            JsonValue::Bool(b) => Value::Bool(*b),
                            // Add other type conversions as needed.
                            // I'm lacking Object, Array abd null - TODO
                            _ => Value::String(actual_value.to_string()),
                        };
                        return &our_value == expected_value;
                    }
                }
                false
            }
            Condition::JsonPathCapture { path, capture_as } => {
                if let Ok(json_body) = serde_json::from_str::<JsonValue>(&last_response.body) {
                    if let Some(value) = json_body.pointer(path) {
                        // Convert the JSON value to a string and capture it
                        let captured_value = match value {
                            JsonValue::String(s) => s.clone(),
                            JsonValue::Number(n) => n.to_string(),
                            JsonValue::Bool(b) => b.to_string(),
                            JsonValue::Null => "null".to_string(),
                            _ => value.to_string(), // For arrays and objects
                        };

                        variables.insert(capture_as.clone(), captured_value);

                        if verbose {
                            println!(
                                "[WEB_BACKEND] Captured value from path '{}': {}",
                                path,
                                variables.get(capture_as).unwrap()
                            );
                        }

                        return true;
                    }
                }
                false
            }
            _ => false, // Not a web condition
        }
    }
}

/// Recursively removes a field from a serde_json::Value.
fn remove_json_field_recursive(value: &mut JsonValue, field_to_remove: &str) {
    match value {
        JsonValue::Object(map) => {
            map.remove(field_to_remove);
            for (_, v) in map.iter_mut() {
                remove_json_field_recursive(v, field_to_remove);
            }
        }
        JsonValue::Array(arr) => {
            for v in arr.iter_mut() {
                remove_json_field_recursive(v, field_to_remove);
            }
        }
        _ => {}
    }
}
