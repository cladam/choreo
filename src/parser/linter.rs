use crate::parser::ast::{
    Action, Condition, GivenStep, Scenario, Statement, TestCase, TestSuite, TestSuiteSettings,
    ThenStep, Value, WhenStep,
};
use std::collections::{HashMap, HashSet};

// The E, W and I codes are inspired by ESLint's conventions.
pub struct DiagnosticRule {
    pub code: &'static str,
    pub message: &'static str,
}

pub struct DiagnosticCodes;

impl DiagnosticCodes {
    // Error codes (E) - Critical issues in the chor file
    pub const TIMEOUT_ZERO: DiagnosticRule = DiagnosticRule {
        code: "E001",
        message: "Timeout cannot be zero",
    };
    pub const INVALID_HTTP_STATUS: DiagnosticRule = DiagnosticRule {
        code: "E002",
        message: "Invalid HTTP status code",
    };
    pub const INVALID_HEADER_NAME: DiagnosticRule = DiagnosticRule {
        code: "E003",
        message:
            "Invalid HTTP header name. Header names cannot contain spaces or special characters.",
    };
    pub const INVALID_JSON_BODY: DiagnosticRule = DiagnosticRule {
        code: "E004",
        message: "Request body is not valid JSON, but Content-Type is 'application/json'.",
    };

    // Warning codes (W) - Potential issues
    pub const SCENARIO_NO_TESTS: DiagnosticRule = DiagnosticRule {
        code: "W001",
        message: "Scenario has no test cases and will be skipped.",
    };
    pub const TEST_NO_GIVEN_STEPS: DiagnosticRule = DiagnosticRule {
        code: "W002",
        message: "Test has no `given` steps; its execution may depend on implicit state.",
    };
    pub const HTTP_URL_NO_PROTOCOL: DiagnosticRule = DiagnosticRule {
        code: "W003",
        message: "HTTP URL should start with http:// or https://",
    };
    pub const WAIT_TIME_EXCESSIVE: DiagnosticRule = DiagnosticRule {
        code: "W004",
        message: "Wait time exceeds 5 minutes",
    };
    pub const TIMEOUT_EXCESSIVE: DiagnosticRule = DiagnosticRule {
        code: "W005",
        message: "timeout exceeds 5 minutes",
    };
    pub const EXPECTED_FAILURES_HIGH: DiagnosticRule = DiagnosticRule {
        code: "W006",
        message: "Expected failures count seems unusually high",
    };

    pub const DUPLICATE_SCENARIO_NAME: DiagnosticRule = DiagnosticRule {
        code: "W008",
        message: "Scenario name is duplicated. All scenario names in a feature should be unique.",
    };
    pub const MISSING_CLEANUP: DiagnosticRule = DiagnosticRule {
        code: "W009",
        message: "Scenario creates files or directories but has no `after` block for cleanup.",
    };
    pub const UNUSED_VARIABLE: DiagnosticRule = DiagnosticRule {
        code: "W010",
        message: "Variable is defined but never used.",
    };
    pub const HTTP_URL_IS_LOCALHOST: DiagnosticRule = DiagnosticRule {
        code: "W011",
        message: "URL points to localhost. This may not be accessible in all environments.",
    };
    pub const HTTP_URL_IS_PLACEHOLDER: DiagnosticRule = DiagnosticRule {
        code: "W012",
        message: "URL uses a placeholder domain (e.g. example.com). Is this intentional?",
    };
    pub const SUSPICIOUS_HEADER_TYPO: DiagnosticRule = DiagnosticRule {
        code: "W013",
        message: "HTTP header contains a common typo.",
    };
    pub const CONFLICTING_HEADER: DiagnosticRule = DiagnosticRule { code: "W014", message: "Conflicting HTTP header. A header like 'Content-Type' should only be set once per request." };
    pub const LARGE_REQUEST_BODY: DiagnosticRule = DiagnosticRule {
        code: "W015",
        message: "HTTP request body is very large and may cause performance issues or timeouts.",
    };
    pub const HARDCODED_CREDENTIALS: DiagnosticRule = DiagnosticRule {
        code: "W016",
        message: "Potential hardcoded credentials found in URL or header. Use variables instead.",
    };
    pub const INSECURE_HTTP_URL: DiagnosticRule = DiagnosticRule {
        code: "W017",
        message: "URL uses insecure HTTP protocol instead of HTTPS.",
    };
    pub const MISSING_USER_AGENT: DiagnosticRule = DiagnosticRule {
        code: "W018",
        message: "No User-Agent header was set for the HTTP request.",
    };

    // Info codes (I) - Informational
    pub const BEST_PRACTICE_SUGGESTION: DiagnosticRule = DiagnosticRule {
        code: "I001",
        message: "Consider using best practices",
    };
    pub const STOP_ON_FAILURE_ENABLED: DiagnosticRule = DiagnosticRule {
        code: "I002",
        message: "Stop on failure is enabled - tests will halt on first failure",
    };
}

pub struct Diagnostic {
    pub rule_id: String,
    pub message: String,
    pub line: usize,
    pub severity: Severity,
}

#[derive(Debug, PartialEq)]
pub enum Severity {
    Warning,
    Error,
    Info,
}

struct Linter {
    diagnostics: Vec<Diagnostic>,
    defined_vars: HashSet<String>,
    used_vars: HashSet<String>,
    seen_scenario_names: HashSet<String>,
    current_headers: HashMap<String, String>,
}

impl Linter {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            defined_vars: HashSet::new(),
            used_vars: HashSet::new(),
            seen_scenario_names: HashSet::new(),
            current_headers: HashMap::new(),
        }
    }

    pub fn lint(&mut self, suite: &TestSuite) -> &Vec<Diagnostic> {
        self.diagnostics.clear();
        self.visit_test_suite(suite);
        &self.diagnostics
    }

    fn lint_url(&mut self, url: &str) {
        // Skip linting if the URL contains variable substitution like ${URL}
        if url.contains("${") {
            return;
        }

        if url.contains("localhost") || url.contains("127.0.0.1") {
            self.add_diagnostic(
                &DiagnosticCodes::HTTP_URL_IS_LOCALHOST,
                &format!(
                    "{} {}",
                    DiagnosticCodes::HTTP_URL_IS_LOCALHOST.code,
                    DiagnosticCodes::HTTP_URL_IS_LOCALHOST.message
                ),
                0,
                Severity::Warning,
            );
        }
        if url.contains("example.com") || url.contains("example.org") {
            self.add_diagnostic(
                &DiagnosticCodes::HTTP_URL_IS_PLACEHOLDER,
                &format!(
                    "{} {}",
                    DiagnosticCodes::HTTP_URL_IS_PLACEHOLDER.code,
                    DiagnosticCodes::HTTP_URL_IS_PLACEHOLDER.message
                ),
                0,
                Severity::Warning,
            );
        }
        if url.starts_with("http://") {
            self.add_diagnostic(
                &DiagnosticCodes::INSECURE_HTTP_URL,
                &format!(
                    "{} {}",
                    DiagnosticCodes::INSECURE_HTTP_URL.code,
                    DiagnosticCodes::INSECURE_HTTP_URL.message
                ),
                0,
                Severity::Warning,
            );
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            self.add_diagnostic(
                &DiagnosticCodes::HTTP_URL_NO_PROTOCOL,
                &format!(
                    "{}: {}",
                    DiagnosticCodes::HTTP_URL_NO_PROTOCOL.code,
                    DiagnosticCodes::HTTP_URL_NO_PROTOCOL.message,
                ),
                0,
                Severity::Warning,
            );
        }

        if url.contains('@') && !url.contains("${") {
            self.add_diagnostic(
                &DiagnosticCodes::HARDCODED_CREDENTIALS,
                &format!(
                    "{} {}",
                    DiagnosticCodes::HARDCODED_CREDENTIALS.code,
                    DiagnosticCodes::HARDCODED_CREDENTIALS.message
                ),
                0,
                Severity::Warning,
            );
        }
    }

    fn lint_header(&mut self, key: &str, value: &str) {
        let lower_key = key.to_lowercase();
        // This regex ensures HTTP header names contain only valid "token" characters as defined by RFC 7230.
        // Valid: Content-Type, User-Agent, X-Custom-Header
        // Invalid: Content Type (space), User@Agent (@ symbol), Header:Value (colon)
        let re = regex::Regex::new(r"^[!#$%&'*+\-.^_`|~0-9a-zA-Z]+$").unwrap();
        //println!("key: {}", key);
        if !re.is_match(key) {
            //println!("key matches regexp {}", key);
            self.add_diagnostic(
                &DiagnosticCodes::INVALID_HEADER_NAME,
                &format!(
                    "{} {}",
                    DiagnosticCodes::INVALID_HEADER_NAME.code,
                    DiagnosticCodes::INVALID_HEADER_NAME.message
                ),
                0,
                Severity::Warning,
            );
        }

        if lower_key == "contet-type" || lower_key == "acept" {
            self.add_diagnostic(
                &DiagnosticCodes::SUSPICIOUS_HEADER_TYPO,
                &format!(
                    "{} {}",
                    DiagnosticCodes::SUSPICIOUS_HEADER_TYPO.code,
                    DiagnosticCodes::SUSPICIOUS_HEADER_TYPO.message
                ),
                0,
                Severity::Warning,
            );
        }

        if lower_key == "authorization" && !value.contains("${") {
            self.add_diagnostic(
                &DiagnosticCodes::HARDCODED_CREDENTIALS,
                &format!(
                    "{} {}",
                    DiagnosticCodes::HARDCODED_CREDENTIALS.code,
                    DiagnosticCodes::HARDCODED_CREDENTIALS.message
                ),
                0,
                Severity::Warning,
            );
        }

        if self.current_headers.contains_key(&lower_key) && lower_key == "content-type" {
            self.add_diagnostic(
                &DiagnosticCodes::CONFLICTING_HEADER,
                &format!(
                    "{} {}",
                    DiagnosticCodes::CONFLICTING_HEADER.code,
                    DiagnosticCodes::CONFLICTING_HEADER.message,
                ),
                0,
                Severity::Warning,
            );
        }

        self.current_headers
            .insert(lower_key.to_owned(), value.to_owned());
    }

    fn lint_http_body(&mut self, body: &str) {
        // Skip linting if the body contains variable substitution
        if body.contains("${") {
            return;
        }

        //println!("body: {}", body);

        if body.len() > 10 * 1024 {
            // 10 KB
            self.add_diagnostic(
                &DiagnosticCodes::LARGE_REQUEST_BODY,
                &format!(
                    "{} {}",
                    DiagnosticCodes::LARGE_REQUEST_BODY.code,
                    DiagnosticCodes::LARGE_REQUEST_BODY.message,
                ),
                0,
                Severity::Warning,
            );
        }
        if let Some(content_type) = self.current_headers.get("content-type") {
            if content_type.contains("application/json")
                && serde_json::from_str::<serde_json::Value>(body).is_err()
            {
                self.add_diagnostic(
                    &DiagnosticCodes::INVALID_JSON_BODY,
                    &format!(
                        "{} {}",
                        DiagnosticCodes::INVALID_JSON_BODY.code,
                        DiagnosticCodes::INVALID_JSON_BODY.message
                    ),
                    0,
                    Severity::Warning,
                );
            }
        }
    }

    // Use a custom formatted message
    fn add_diagnostic(
        &mut self,
        rule: &DiagnosticRule,
        message: &str,
        line: usize,
        severity: Severity,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_id: rule.code.to_string(),
            message: message.to_string(),
            line,
            severity,
        });
    }
}

// Add this convenience function at the module level
pub fn lint(suite: &TestSuite) -> Vec<String> {
    let mut linter = Linter::new();
    let diagnostics = linter.lint(suite);

    diagnostics
        .iter()
        .map(|d| format!("[{}] {}", d.rule_id, d.message))
        .collect()
}

pub trait Visitor {
    fn visit_test_suite(&mut self, suite: &TestSuite);
    fn visit_statement(&mut self, stmt: &Statement);
    fn visit_settings(&mut self, settings: &TestSuiteSettings);
    fn visit_scenario(&mut self, scenario: &Scenario);
    fn visit_test_case(&mut self, test: &TestCase);
    fn visit_given_step(&mut self, step: &GivenStep);
    fn visit_action(&mut self, action: &Action);
    fn visit_condition(&mut self, condition: &Condition);
    fn visit_background(&mut self, steps: &Vec<GivenStep>);
    fn visit_env_def(&mut self, vars: &Vec<String>);
    fn visit_var_def(&mut self, name: &String, value: &Value);
    fn visit_actor_def(&mut self, actors: &Vec<String>);
    //fn visit_feature_def(&mut self, name: &String);
}

impl Visitor for Linter {
    fn visit_test_suite(&mut self, suite: &TestSuite) {
        // First pass: collect all variable definitions
        for statement in &suite.statements {
            match statement {
                Statement::EnvDef(vars) => {
                    for var in vars {
                        //println!("Env: {}", var);
                        self.defined_vars.insert(var.clone());
                    }
                }
                Statement::VarDef(name, _value) => {
                    //println!("Var: {}", name);
                    self.defined_vars.insert(name.clone());
                }
                _ => {}
            }
        }

        // Second pass: visit all nodes to find variable usages and check rules
        self.seen_scenario_names.clear();
        for statement in &suite.statements {
            self.visit_statement(statement);
        }

        // Third pass: check for unused variables
        let unused_vars: Vec<String> = self
            .defined_vars
            .difference(&self.used_vars)
            .cloned()
            .collect();
        for var in unused_vars {
            self.add_diagnostic(
                &DiagnosticCodes::UNUSED_VARIABLE,
                &format!(
                    "{}: {} ({})",
                    DiagnosticCodes::UNUSED_VARIABLE.code,
                    DiagnosticCodes::UNUSED_VARIABLE.message,
                    var
                ),
                0, // line number - need to get span info
                Severity::Warning,
            );
        }
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Scenario(scenario) => self.visit_scenario(scenario),
            Statement::TestCase(test) => self.visit_test_case(test),
            Statement::SettingsDef(settings) => self.visit_settings(settings),
            Statement::BackgroundDef(steps) => self.visit_background(steps),
            // Nothing to lint really
            // Statement::FeatureDef(name) => self.visit_feature_def(name),
            Statement::ActorDef(actors) => self.visit_actor_def(actors),
            Statement::EnvDef(vars) => self.visit_env_def(vars),
            Statement::VarDef(name, value) => self.visit_var_def(name, value),
            _ => {}
        }
    }

    fn visit_settings(&mut self, settings: &TestSuiteSettings) {
        let default_span = settings
            .span
            .as_ref()
            .map(|s| (s.line, s.column))
            .unwrap_or((0, 0));

        if settings.timeout_seconds == 0 {
            let (line, _column) = settings
                .setting_spans
                .as_ref()
                .and_then(|spans| spans.timeout_seconds_span.as_ref())
                .map(|span| (span.line, span.column))
                .unwrap_or(default_span);

            self.add_diagnostic(
                &DiagnosticCodes::TIMEOUT_ZERO,
                &format!(
                    "{}: {} (line: {})",
                    DiagnosticCodes::TIMEOUT_ZERO.code,
                    DiagnosticCodes::TIMEOUT_ZERO.message,
                    line
                ),
                line,
                Severity::Error,
            );
        }

        if settings.timeout_seconds > 300 {
            let (line, _column) = settings
                .setting_spans
                .as_ref()
                .and_then(|spans| spans.timeout_seconds_span.as_ref())
                .map(|span| (span.line, span.column))
                .unwrap_or(default_span);

            self.add_diagnostic(
                &DiagnosticCodes::TIMEOUT_EXCESSIVE,
                &format!(
                    "{}: {} (line: {})",
                    DiagnosticCodes::TIMEOUT_EXCESSIVE.code,
                    DiagnosticCodes::TIMEOUT_EXCESSIVE.message,
                    line
                ),
                line,
                Severity::Error,
            );
        }

        // Warn if stop_on_failure is enabled
        if settings.stop_on_failure {
            let (line, _column) = settings
                .setting_spans
                .as_ref()
                .and_then(|spans| spans.stop_on_failure_span.as_ref())
                .map(|span| (span.line, span.column))
                .unwrap_or(default_span);
            self.add_diagnostic(
                &DiagnosticCodes::STOP_ON_FAILURE_ENABLED,
                &format!(
                    "{}: {} (line: {})",
                    DiagnosticCodes::STOP_ON_FAILURE_ENABLED.code,
                    DiagnosticCodes::STOP_ON_FAILURE_ENABLED.message,
                    line
                ),
                line,
                Severity::Info,
            );
        }

        // Validate expected_failures
        if settings.expected_failures > 100 {
            let (line, _column) = settings
                .setting_spans
                .as_ref()
                .and_then(|spans| spans.expected_failures_span.as_ref())
                .map(|span| (span.line, span.column))
                .unwrap_or(default_span);
            self.add_diagnostic(
                &DiagnosticCodes::EXPECTED_FAILURES_HIGH,
                &format!(
                    "{}: {} (line: {})",
                    DiagnosticCodes::EXPECTED_FAILURES_HIGH.code,
                    DiagnosticCodes::EXPECTED_FAILURES_HIGH.message,
                    line
                ),
                line,
                Severity::Warning,
            );
        }
    }

    fn visit_scenario(&mut self, scenario: &Scenario) {
        let (line, _column) = scenario
            .span
            .as_ref()
            .map_or((0, 0), |s| (s.line, s.column));
        //println!("Scenario: {}", scenario.name);

        // Check for empty scenarios.
        if scenario.tests.is_empty() {
            self.add_diagnostic(
                &DiagnosticCodes::SCENARIO_NO_TESTS,
                &format!(
                    "{}: {} (line: {})",
                    DiagnosticCodes::SCENARIO_NO_TESTS.code,
                    DiagnosticCodes::SCENARIO_NO_TESTS.message,
                    line
                ),
                line,
                Severity::Warning,
            );
        }

        // Check for duplicate scenario names.
        if !self.seen_scenario_names.insert(scenario.name.clone()) {
            self.add_diagnostic(
                &DiagnosticCodes::DUPLICATE_SCENARIO_NAME,
                &format!(
                    "{}: {} (line: {})",
                    DiagnosticCodes::DUPLICATE_SCENARIO_NAME.code,
                    DiagnosticCodes::DUPLICATE_SCENARIO_NAME.message,
                    line
                ),
                line,
                Severity::Warning,
            );
        }

        // Check if setup actions exist without a corresponding cleanup.
        if scenario_has_setup_actions(scenario) && scenario.after.is_empty() {
            self.add_diagnostic(
                &DiagnosticCodes::MISSING_CLEANUP,
                &format!(
                    "{}: {} (line: {})",
                    DiagnosticCodes::MISSING_CLEANUP.code,
                    DiagnosticCodes::MISSING_CLEANUP.message,
                    line
                ),
                line,
                Severity::Warning,
            );
        }

        for test in &scenario.tests {
            self.visit_test_case(test);
        }
    }

    fn visit_test_case(&mut self, test: &TestCase) {
        self.current_headers.clear();
        let (line, _column) = test.span.as_ref().map_or((0, 0), |s| (s.line, s.column));
        //println!("Test: {}", test.name);

        for step in &test.given {
            match step {
                GivenStep::Action(a) => self.visit_action(a),
                GivenStep::Condition(c) => self.visit_condition(c),
                GivenStep::TaskCall(_) => {} // Task calls will be expanded before execution
            }
        }
        for step in &test.when {
            match step {
                WhenStep::Action(action) => self.visit_action(action),
                WhenStep::TaskCall(_) => {} // Task calls will be expanded before execution
            }
        }
        for step in &test.then {
            match step {
                ThenStep::Condition(condition) => self.visit_condition(condition),
                ThenStep::TaskCall(_) => {} // Task calls will be expanded before execution
            }
        }

        // Check for missing User-Agent
        if self.current_headers.values().any(|v| v.starts_with("http"))
            && !self.current_headers.contains_key("user-agent")
        {
            self.add_diagnostic(
                &DiagnosticCodes::MISSING_USER_AGENT,
                &format!(
                    "{} {}",
                    DiagnosticCodes::MISSING_USER_AGENT.code,
                    DiagnosticCodes::MISSING_USER_AGENT.message
                ),
                line,
                Severity::Warning,
            );
        }
    }

    fn visit_given_step(&mut self, step: &GivenStep) {
        match step {
            GivenStep::Action(a) => self.visit_action(&a),
            GivenStep::Condition(c) => self.visit_condition(&c),
            GivenStep::TaskCall(_) => {} // Task calls will be expanded before execution
        }
    }

    fn visit_action(&mut self, action: &Action) {
        // A helper closure to find variables in a string.
        let find_vars = |s: &str, used: &mut HashSet<String>| {
            let re = regex::Regex::new(r"\$\{(\w+)}").unwrap();
            for cap in re.captures_iter(s) {
                used.insert(cap[1].to_string());
            }
        };

        match action {
            Action::Run { command, .. } => find_vars(command, &mut self.used_vars),
            Action::SetCwd { path } => find_vars(path, &mut self.used_vars),
            Action::CreateFile { path, content } => {
                find_vars(path, &mut self.used_vars);
                find_vars(content, &mut self.used_vars);
            }
            Action::DeleteFile { path }
            | Action::CreateDir { path }
            | Action::DeleteDir { path } => {
                find_vars(path, &mut self.used_vars);
            }
            Action::HttpSetHeader { key, value } | Action::HttpSetCookie { key, value } => {
                find_vars(key, &mut self.used_vars);
                find_vars(value, &mut self.used_vars);
                self.lint_header(key, value);
            }
            Action::HttpGet { url } | Action::HttpDelete { url } => {
                find_vars(url, &mut self.used_vars);
                self.lint_url(url);
            }
            Action::HttpPost { url, body }
            | Action::HttpPut { url, body }
            | Action::HttpPatch { url, body } => {
                find_vars(url, &mut self.used_vars);
                find_vars(body, &mut self.used_vars);
                self.lint_url(url);
                self.lint_http_body(body);
            }
            // Other actions...
            _ => {}
        }
    }

    fn visit_condition(&mut self, condition: &Condition) {
        // A helper closure to find variables in a string.
        let mut find_cond_vars = |s: &str| {
            let re = regex::Regex::new(r"\$\{(\w+)}").unwrap();
            for cap in re.captures_iter(s) {
                self.used_vars.insert(cap[1].to_string());
            }
        };

        const VALID_HTTP_STATUS_RANGE: std::ops::RangeInclusive<u16> = 100..=599;

        match condition {
            Condition::Wait { wait, .. } => {
                // Excessive wait time
                if *wait > 300.0 {
                    // 5 minutes
                    self.add_diagnostic(
                        &DiagnosticCodes::WAIT_TIME_EXCESSIVE,
                        &format!(
                            "{}: {} ({:.1}s)",
                            DiagnosticCodes::WAIT_TIME_EXCESSIVE.code,
                            DiagnosticCodes::WAIT_TIME_EXCESSIVE.message,
                            wait
                        ),
                        0,
                        Severity::Warning,
                    );
                }
            }

            Condition::OutputContains { text, .. }
            | Condition::OutputNotContains { text, .. }
            | Condition::StderrContains(text)
            | Condition::OutputStartsWith(text)
            | Condition::OutputEndsWith(text)
            | Condition::OutputEquals(text) => {
                find_cond_vars(text);
            }

            Condition::OutputMatches { regex, .. } => {
                find_cond_vars(regex);
            }

            Condition::JsonPathEquals {
                path,
                expected_value: value,
            } => {
                find_cond_vars(path);
                if let Value::String(s) = value {
                    find_cond_vars(s);
                }
            }

            Condition::JsonValueIsString { path }
            | Condition::JsonValueIsNumber { path }
            | Condition::JsonValueIsArray { path }
            | Condition::JsonValueIsObject { path }
            | Condition::JsonBodyHasPath { path } => {
                find_cond_vars(path);
            }
            Condition::JsonValueHasSize { path, .. }
            | Condition::JsonOutputAtEquals { path, .. }
            | Condition::JsonOutputAtIncludes { path, .. }
            | Condition::JsonOutputAtHasItemCount { path, .. } => {
                find_cond_vars(path);
            }

            Condition::FileExists { path }
            | Condition::FileDoesNotExist { path }
            | Condition::DirExists { path }
            | Condition::DirDoesNotExist { path }
            | Condition::FileIsEmpty { path }
            | Condition::FileIsNotEmpty { path } => {
                find_cond_vars(path);
            }

            Condition::ResponseStatusIs(status) => {
                // Invalid HTTP status code
                if !VALID_HTTP_STATUS_RANGE.contains(status) {
                    self.add_diagnostic(
                        &DiagnosticCodes::INVALID_HTTP_STATUS,
                        &format!(
                            "{}: {} ({})",
                            DiagnosticCodes::INVALID_HTTP_STATUS.code,
                            DiagnosticCodes::INVALID_HTTP_STATUS.message,
                            status
                        ),
                        0,
                        Severity::Error,
                    );
                }
            }
            Condition::ResponseStatusIsIn(statuses) => {
                for status in statuses {
                    if !VALID_HTTP_STATUS_RANGE.contains(status) {
                        self.add_diagnostic(
                            &DiagnosticCodes::INVALID_HTTP_STATUS,
                            &format!(
                                "{}: {} ({})",
                                DiagnosticCodes::INVALID_HTTP_STATUS.code,
                                DiagnosticCodes::INVALID_HTTP_STATUS.message,
                                status
                            ),
                            0,
                            Severity::Error,
                        );
                    }
                }
            }
            Condition::ResponseBodyContains { value } => {
                find_cond_vars(value);
            }
            Condition::ResponseBodyMatches { regex, .. } => {
                find_cond_vars(regex);
            }

            // Other conditions...
            _ => {}
        }
    }

    fn visit_background(&mut self, steps: &Vec<GivenStep>) {
        // A background block is essentially a given bloch that is parsed before everything else in a scenario
        for step in steps {
            self.visit_given_step(step);
        }
    }

    fn visit_env_def(&mut self, vars: &Vec<String>) {
        for var in vars {
            // Check for standard naming convention
            if !var
                .chars()
                .all(|c| c.is_uppercase() || c.is_numeric() || c == '_')
            {
                self.add_diagnostic(
                    &DiagnosticCodes::BEST_PRACTICE_SUGGESTION,
                    &format!(
                        "Environment variable '{}' should use SCREAMING_SNAKE_CASE",
                        var
                    ),
                    0,
                    Severity::Info,
                );
            }

            // Warn about potentially missing common environment variables
            if var == "PATH" || var == "HOME" {
                self.add_diagnostic(
                    &DiagnosticCodes::BEST_PRACTICE_SUGGESTION,
                    &format!(
                        "Environment variable '{}' is system-critical. Ensure it's available.",
                        var
                    ),
                    0,
                    Severity::Info,
                );
            }
        }
    }

    fn visit_var_def(&mut self, name: &String, _value: &Value) {
        // Check for naming convention (SCREAMING_SNAKE_CASE for variables)
        if !name
            .chars()
            .all(|c| c.is_uppercase() || c.is_numeric() || c == '_')
        {
            self.add_diagnostic(
                &DiagnosticCodes::BEST_PRACTICE_SUGGESTION,
                &format!(
                    "Variable '{}' should use SCREAMING_SNAKE_CASE naming convention",
                    name
                ),
                0,
                Severity::Info,
            );
        }

        // Check for potentially sensitive variable names
        let sensitive_keywords = ["PASSWORD", "SECRET", "TOKEN", "KEY", "API_KEY"];
        if sensitive_keywords
            .iter()
            .any(|&keyword| name.to_uppercase().contains(keyword))
        {
            self.add_diagnostic(
                &DiagnosticCodes::HARDCODED_CREDENTIALS,
                &format!("Variable '{}' appears to contain sensitive data", name),
                0,
                Severity::Warning,
            );
        }
    }

    fn visit_actor_def(&mut self, actors: &Vec<String>) {
        const VALID_ACTORS: &[&str] = &["Web", "Terminal", "System", "FileSystem"];

        let mut seen_actors = HashSet::new();

        for actor in actors {
            //println!("actor: {}", actor);
            // Check for duplicate actors
            if !seen_actors.insert(actor.clone()) {
                self.add_diagnostic(
                    &DiagnosticCodes::DUPLICATE_SCENARIO_NAME, // reuse
                    &format!("Duplicate actor '{}' found", actor),
                    0,
                    Severity::Warning,
                );
            }

            // Check if actor is valid
            if !VALID_ACTORS.contains(&actor.as_str()) {
                self.add_diagnostic(
                    &DiagnosticCodes::BEST_PRACTICE_SUGGESTION,
                    &format!(
                        "Unknown actor '{}'. Valid actors are: {}",
                        actor,
                        VALID_ACTORS.join(", ")
                    ),
                    0,
                    Severity::Error,
                );
            }

            // Check naming convention (PascalCase)
            if !actor.chars().next().unwrap_or(' ').is_uppercase() {
                self.add_diagnostic(
                    &DiagnosticCodes::BEST_PRACTICE_SUGGESTION,
                    &format!(
                        "Actor '{}' should follow PascalCase naming convention",
                        actor
                    ),
                    0,
                    Severity::Info,
                );
            }
        }
    }
}

///Helper function to check if a scenario contains file system creation actions.
fn scenario_has_setup_actions(scenario: &Scenario) -> bool {
    for test in &scenario.tests {
        // Collect actions from given steps
        let given_actions: Vec<_> = test
            .given
            .iter()
            .filter_map(|s| match s {
                GivenStep::Action(a) => Some(a),
                _ => None,
            })
            .collect();

        // Collect actions from when steps
        let when_actions: Vec<_> = test
            .when
            .iter()
            .filter_map(|s| match s {
                WhenStep::Action(a) => Some(a),
                _ => None,
            })
            .collect();

        // Check all actions for filesystem creation
        for action in given_actions.into_iter().chain(when_actions.into_iter()) {
            if action.is_filesystem_creation() {
                return true;
            }
        }
    }
    false
}
