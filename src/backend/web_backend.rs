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
        match action {
            Action::HttpSetHeader { key, value } => {
                let substituted_key = substitute_string(key, env_vars);
                let substituted_value = substitute_string(value, env_vars);
                if verbose {
                    println!(
                        "[WEB_BACKEND] Setting HTTP header: {}: {}",
                        substituted_key, substituted_value
                    );
                }
                self.headers.insert(substituted_key, substituted_value);
                true
            }
            Action::HttpClearHeader { key } => {
                let substituted_key = substitute_string(key, env_vars);
                if verbose {
                    println!("[WEB_BACKEND] Clearing HTTP header: {}", substituted_key);
                }
                self.headers.remove(&substituted_key);
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
                let substituted_key = substitute_string(key, env_vars);
                let substituted_value = substitute_string(value, env_vars);

                // Handle multiple cookies by appending to existing Cookie header
                let new_cookie = format!("{}={}", substituted_key, substituted_value);
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
                    println!(
                        "[WEB_BACKEND] Added cookie: {}={}",
                        substituted_key, substituted_value
                    );
                    println!(
                        "[WEB_BACKEND] Current Cookie header: {}",
                        self.headers.get("Cookie").unwrap_or(&"".to_string())
                    );
                }
                true
            }
            Action::HttpClearCookie { key } => {
                let substituted_key = substitute_string(key, env_vars);

                if let Some(cookie_header) = self.headers.get("Cookie") {
                    // Parse and filter out the specific cookie
                    let cookies: Vec<&str> = cookie_header.split(';').collect();
                    let filtered_cookies: Vec<&str> = cookies
                        .into_iter()
                        .filter(|cookie| {
                            let cookie_trimmed = cookie.trim();
                            !cookie_trimmed.starts_with(&format!("{}=", substituted_key))
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
                    println!("[WEB_BACKEND] Cleared cookie: {}", substituted_key);
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
                let substituted_url = substitute_string(url, env_vars);
                if verbose {
                    println!("[WEB_BACKEND] Performing HTTP GET to: {}", substituted_url);
                }

                let start_time = std::time::Instant::now();
                let mut request = self.agent.get(&substituted_url);
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
                let substituted_url = substitute_string(url, env_vars);
                let substituted_body = substitute_string(body, env_vars);

                if verbose {
                    println!("[WEB_BACKEND] POST URL: {}", substituted_url);
                    println!("[WEB_BACKEND] POST body: {}", substituted_body);
                }

                let start_time = std::time::Instant::now();
                let mut request = self.agent.post(&substituted_url);

                // Apply headers
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                // Send request and handle response
                match request.send(&substituted_body) {
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
                let substituted_url = substitute_string(url, env_vars);
                let substituted_body = substitute_string(body, env_vars);

                if verbose {
                    println!("  [WEB] PUT {}", substituted_url);
                }

                let start_time = std::time::Instant::now();
                let mut request = self.agent.put(&substituted_url);

                // Add headers and cookies (same as POST)
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                match request.send(substituted_body) {
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
                let substituted_url = substitute_string(url, env_vars);
                let substituted_body = substitute_string(body, env_vars);

                if verbose {
                    println!(
                        "[WEB_BACKEND] Performing HTTP PATCH to: {}",
                        substituted_url
                    );
                }

                let start_time = std::time::Instant::now();
                let mut request = self.agent.patch(&substituted_url);

                // Add headers
                for (key, value) in &self.headers {
                    request = request.header(key, value);
                }

                match request.send(substituted_body) {
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
                let substituted_url = substitute_string(url, env_vars);

                if verbose {
                    println!(
                        "[WEB_BACKEND] Performing HTTP DELETE to: {}",
                        substituted_url
                    );
                }

                let start_time = std::time::Instant::now();
                let mut request = self.agent.delete(&substituted_url);

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
            Condition::ResponseBodyEqualsJson { expected } => {
                // Substitute variables in the expected JSON string
                let substituted_expected = substitute_string(expected, variables);
                // Parse both the response body and expected JSON for comparison
                match (
                    serde_json::from_str::<JsonValue>(&last_response.body),
                    serde_json::from_str::<JsonValue>(&substituted_expected),
                ) {
                    (Ok(actual), Ok(expected_json)) => {
                        if verbose {
                            println!(
                                "[WEB_BACKEND] Comparing JSON response body with expected JSON"
                            );
                            //println!("[WEB_BACKEND] Actual: {}", actual);
                            //println!("[WEB_BACKEND] Expected: {}", expected_json);
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
                        match value {
                            JsonValue::Array(arr) => return arr.len() == *size,
                            JsonValue::String(s) => return s.len() == *size,
                            JsonValue::Object(obj) => return obj.len() == *size,
                            _ => return false,
                        }
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
                            // Add other type conversions as needed.
                            _ => Value::String(actual_value.to_string()),
                        };
                        return &our_value == expected_value;
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
