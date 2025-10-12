use crate::parser::ast::{Action, Condition, Value};
use crate::parser::helpers::{substitute_string, substitute_variables_in_action};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use ureq::http::StatusCode;
use ureq::Agent;

/// State of the last web request made.
#[derive(Debug, Clone, Default)]
pub struct LastResponse {
    pub status: StatusCode,
    pub body: String,
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

    // Return a cloned map of current headers so caller can capture them
    pub fn get_headers(&self) -> HashMap<String, String> {
        // Adjust to match the actual internal representation (this assumes a HashMap field named headers)
        self.headers.clone()
    }

    // Ensure you have a `set_header` method (used above). If it doesn't exist, implement it:
    pub fn set_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }
    /// Creates a new WebBackend with a persistent HTTP client.
    pub fn new() -> Self {
        Self {
            agent: Agent::new_with_defaults(),
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
        let substituted_action = substitute_variables_in_action(action, env_vars);
        match &substituted_action {
            Action::HttpSetHeader { key, value } => {
                if verbose {
                    println!("[WEB_BACKEND] Setting HTTP header: {}: {}", key, value);
                }
                self.headers.insert(key.clone(), value.clone());
                true
            }
            Action::HttpClearHeader { key } => {
                if verbose {
                    println!("[WEB_BACKEND] Clearing HTTP header: {}", key);
                }
                self.headers.remove(&*key);
                true
            }
            Action::HttpClearHeaders => {
                if verbose {
                    println!("[WEB_BACKEND] Clearing all HTTP headers");
                }
                self.headers.clear();
                true
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
                true
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
                true
            }
            Action::HttpClearCookies => {
                if verbose {
                    println!("[WEB_BACKEND] Clearing all HTTP cookies");
                }
                self.headers.remove("Cookie");
                true
            }
            Action::HttpGet { url, .. } => {
                if verbose {
                    println!("[WEB_BACKEND] Performing HTTP GET to: {}", url);
                }

                let start_time = std::time::Instant::now();
                let mut request = self.agent.get(url);
                // Add any headers that have been set.
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }
                let response_result = request.call();
                let response_time_ms = start_time.elapsed().as_millis();

                match response_result {
                    Ok(response) => {
                        let status = response.status();
                        {
                            let mut body_reader = response.into_body();
                            match body_reader.read_to_string() {
                                Ok(res) => {
                                    if verbose {
                                        println!("[WEB_BACKEND] Received response body: {}", res);
                                    }
                                    self.last_response = Some(LastResponse {
                                        status,
                                        body: res.clone(),
                                        response_time_ms,
                                    });
                                    res
                                }
                                Err(e) => {
                                    let error_message = format!(
                                        "[WEB_BACKEND] Failed to read response body: {}",
                                        e
                                    );
                                    self.last_response = Some(LastResponse {
                                        status,
                                        body: error_message.clone(),
                                        response_time_ms,
                                    });
                                    println!("{}", error_message);
                                    error_message
                                }
                            }
                        };
                        if verbose {
                            println!("[WEB_BACKEND] Received response status: {}", status);
                        }

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
                                response_time_ms,
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
                                response_time_ms,
                            });
                            false
                        }
                    },
                }
            }
            Action::HttpPost { url, body } => {
                if verbose {
                    println!("[WEB_BACKEND] Performing HTTP POST to: {}", url);
                }

                if verbose {
                    println!("[WEB_BACKEND] POST URL: {}", url);
                    println!("[WEB_BACKEND] POST body: {}", body);
                }

                let start_time = std::time::Instant::now();
                let mut request = self.agent.post(url);

                // Apply headers
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                // Send request and handle response
                match request.send(body) {
                    Ok(response) => {
                        let duration = start_time.elapsed();
                        let status = response.status();
                        self.last_response =
                            Some(response.into_body().read_to_string().map_or_else(
                                |e| LastResponse {
                                    status: StatusCode::INTERNAL_SERVER_ERROR,
                                    body: format!("Failed to read response body: {}", e),
                                    response_time_ms: duration.as_millis(),
                                },
                                |body| LastResponse {
                                    status,
                                    body,
                                    response_time_ms: duration.as_millis(),
                                },
                            ));

                        if verbose {
                            if let Some(ref resp) = self.last_response {
                                println!(
                                    "[WEB_BACKEND] POST completed with {} in {:.2}ms",
                                    resp.status,
                                    duration.as_millis()
                                );
                            }
                        }
                        true // Successfully handled
                    }
                    Err(ureq::Error::StatusCode(code)) => {
                        // HTTP error status codes should still be treated as valid responses
                        let duration = start_time.elapsed();
                        let status =
                            StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                        self.last_response = Some(LastResponse {
                            status,
                            body: format!("HTTP error with status code: {}", code),
                            response_time_ms: duration.as_millis(),
                        });

                        if verbose {
                            println!(
                                "[WEB_BACKEND] POST completed with error status {} in {}ms",
                                code,
                                duration.as_millis()
                            );
                        }
                        true // Still return true as the request was made successfully
                    }
                    Err(e) => {
                        let error_message = format!("[WEB_BACKEND] HTTP request failed: {}", e);
                        if verbose {
                            println!("{}", error_message);
                        }
                        self.last_response = Some(LastResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: error_message,
                            response_time_ms: 0,
                        });
                        false
                    }
                }
            }
            Action::HttpPut { url, body } => {
                if verbose {
                    println!("  [WEB] PUT {}", url);
                }

                let start_time = std::time::Instant::now();
                let mut request = self.agent.put(url);

                // Add headers and cookies (same as POST)
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                match request.send(body) {
                    Ok(response) => {
                        let duration = start_time.elapsed();
                        let status = response.status();
                        let body = response
                            .into_body()
                            .read_to_string()
                            .unwrap_or_else(|e| format!("Failed to read response body: {}", e));
                        if verbose {
                            println!(
                                "  [WEB] PUT completed with status {} in {}ms",
                                status,
                                duration.as_millis()
                            );
                            println!("  [WEB] Response body: {}", body);
                        }
                        self.last_response = Some(LastResponse {
                            status,
                            body,
                            response_time_ms: duration.as_millis(),
                        });
                        true
                    }
                    Err(ureq::Error::StatusCode(code)) => {
                        // HTTP error status codes should still be treated as valid responses
                        let duration = start_time.elapsed();
                        let status =
                            StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                        self.last_response = Some(LastResponse {
                            status,
                            body: format!("HTTP error with status code: {}", code),
                            response_time_ms: duration.as_millis(),
                        });

                        if verbose {
                            println!(
                                "[WEB_BACKEND] POST completed with error status {} in {}ms",
                                code,
                                duration.as_millis()
                            );
                        }
                        true // Still return true as the request was made successfully
                    }
                    Err(e) => {
                        let error_message = format!("[WEB_BACKEND] HTTP request failed: {}", e);
                        if verbose {
                            println!("{}", error_message);
                        }
                        self.last_response = Some(LastResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: error_message,
                            response_time_ms: 0,
                        });
                        false
                    }
                }
            }
            Action::HttpPatch { url, body } => {
                if verbose {
                    println!("[WEB_BACKEND] Performing HTTP PATCH to: {}", url);
                }

                let start_time = std::time::Instant::now();
                let mut request = self.agent.patch(url);

                // Add headers
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                match request.send(body) {
                    Ok(response) => {
                        let duration = start_time.elapsed();
                        let status = response.status();
                        let body = response
                            .into_body()
                            .read_to_string()
                            .unwrap_or_else(|e| format!("Failed to read response body: {}", e));

                        if verbose {
                            println!(
                                "[WEB_BACKEND] PATCH completed with status {} in {}ms",
                                status,
                                duration.as_millis()
                            );
                        }

                        self.last_response = Some(LastResponse {
                            status,
                            body,
                            response_time_ms: duration.as_millis(),
                        });
                        true
                    }
                    Err(ureq::Error::StatusCode(code)) => {
                        // HTTP error status codes should still be treated as valid responses
                        let duration = start_time.elapsed();
                        let status =
                            StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                        self.last_response = Some(LastResponse {
                            status,
                            body: format!("HTTP error with status code: {}", code),
                            response_time_ms: duration.as_millis(),
                        });

                        if verbose {
                            println!(
                                "[WEB_BACKEND] POST completed with error status {} in {}ms",
                                code,
                                duration.as_millis()
                            );
                        }
                        true // Still return true as the request was made successfully
                    }
                    Err(e) => {
                        let error_message = format!("[WEB_BACKEND] HTTP request failed: {}", e);
                        if verbose {
                            println!("{}", error_message);
                        }
                        self.last_response = Some(LastResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: error_message,
                            response_time_ms: 0,
                        });
                        false
                    }
                }
            }
            Action::HttpDelete { url } => {
                if verbose {
                    println!("[WEB_BACKEND] Performing HTTP DELETE to: {}", url);
                }

                let start_time = std::time::Instant::now();
                let mut request = self.agent.delete(url);

                // Add headers
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                match request.call() {
                    Ok(response) => {
                        let duration = start_time.elapsed();
                        let status = response.status();
                        let body = response
                            .into_body()
                            .read_to_string()
                            .unwrap_or_else(|e| format!("Failed to read response body: {}", e));

                        if verbose {
                            println!(
                                "[WEB_BACKEND] DELETE completed with status {} in {}ms",
                                status,
                                duration.as_millis()
                            );
                        }

                        self.last_response = Some(LastResponse {
                            status,
                            body,
                            response_time_ms: duration.as_millis(),
                        });
                        true
                    }
                    Err(ureq::Error::StatusCode(code)) => {
                        // HTTP error status codes should still be treated as valid responses
                        let duration = start_time.elapsed();
                        let status =
                            StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                        self.last_response = Some(LastResponse {
                            status,
                            body: format!("HTTP error with status code: {}", code),
                            response_time_ms: duration.as_millis(),
                        });

                        if verbose {
                            println!(
                                "[WEB_BACKEND] POST completed with error status {} in {}ms",
                                code,
                                duration.as_millis()
                            );
                        }
                        true // Still return true as the request was made successfully
                    }
                    Err(e) => {
                        let error_message = format!("[WEB_BACKEND] HTTP request failed: {}", e);
                        if verbose {
                            println!("{}", error_message);
                        }
                        self.last_response = Some(LastResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: error_message,
                            response_time_ms: 0,
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
                // Substitute variables in the expected JSON string
                let substituted_expected = substitute_string(expected, variables);
                // Parse both the response body and expected JSON for comparison
                match (
                    serde_json::from_str::<JsonValue>(&last_response.body),
                    serde_json::from_str::<JsonValue>(&substituted_expected),
                ) {
                    (Ok(mut actual), Ok(expected_json)) => {
                        if verbose {
                            println!(
                                "[WEB_BACKEND] Comparing JSON response body with expected JSON"
                            );
                            //println!("[WEB_BACKEND] Actual: {}", actual);
                            //println!("[WEB_BACKEND] Expected: {}", expected_json);
                        }
                        // Remove ignored fields from both actual and expected JSON values
                        for field in ignored {
                            remove_json_field_recursive(&mut actual, field);
                            //remove_json_field_recursive(&mut expected_json, field);
                        }
                        actual == expected_json
                    }
                    (Err(e), _) => {
                        if verbose {
                            println!("[WEB_BACKEND] Failed to parse response body as JSON: {}", e);
                        }
                        false
                    }
                    (_, Err(e)) => {
                        if verbose {
                            println!("[WEB_BACKEND] Failed to parse expected JSON: {}", e);
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
