use crate::parser::ast::{
    Action, Condition, GivenStep, Scenario, Statement, TestCase, TestSuite, TestSuiteSettings,
    Value,
};
use std::collections::HashSet;

// A simplified example of the structure
struct Linter {
    diagnostics: Vec<Diagnostic>,
    defined_vars: HashSet<String>,
    used_vars: HashSet<String>,
    seen_scenario_names: HashSet<String>,
}

// The E, W and I codes are inspired by ESLint's conventions.
// E: Error - A serious issue that likely prevents correct execution.
// W: Warning - A potential issue that may lead to unexpected behavior.
// I: Info - Informational messages that do not indicate a problem.
pub struct DiagnosticRule {
    pub code: &'static str,
    pub message: &'static str,
}

pub struct DiagnosticCodes;

impl DiagnosticCodes {
    // Error codes (E) - Critical issues that prevent execution
    pub const TIMEOUT_ZERO: DiagnosticRule = DiagnosticRule {
        code: "E001",
        message: "Timeout cannot be zero",
    };
    pub const RUN_COMMAND_EMPTY: DiagnosticRule = DiagnosticRule {
        code: "E002",
        message: "Run command cannot be empty",
    };
    pub const FILE_PATH_EMPTY: DiagnosticRule = DiagnosticRule {
        code: "E003",
        message: "File path cannot be empty",
    };
    pub const WAIT_TIME_NON_POSITIVE: DiagnosticRule = DiagnosticRule {
        code: "E004",
        message: "Wait time must be positive",
    };
    pub const INVALID_HTTP_STATUS: DiagnosticRule = DiagnosticRule {
        code: "E005",
        message: "Invalid HTTP status code",
    };
    pub const JSON_PATH_EMPTY: DiagnosticRule = DiagnosticRule {
        code: "E006",
        message: "JSON path cannot be empty",
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
    pub const STOP_ON_FAILURE_ENABLED: DiagnosticRule = DiagnosticRule {
        code: "W007",
        message: "Stop on failure is enabled - tests will halt on first failure",
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

    // Info codes (I) - Informational
    pub const BEST_PRACTICE_SUGGESTION: DiagnosticRule = DiagnosticRule {
        code: "I001",
        message: "Consider using best practices",
    };
}

pub struct Diagnostic {
    pub rule_id: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub severity: Severity,
}

#[derive(Debug, PartialEq)]
pub enum Severity {
    Warning,
    Error,
    Info,
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
    fn visit_feature_def(&mut self, name: &String);
}

impl Visitor for Linter {
    fn visit_test_suite(&mut self, suite: &TestSuite) {
        // First pass: collect all variable definitions
        for statement in &suite.statements {
            match statement {
                Statement::EnvDef(vars) => {
                    for var in vars {
                        println!("Env: {}", var);
                        self.defined_vars.insert(var.clone());
                    }
                }
                Statement::VarDef(name, _value) => {
                    println!("Var: {}", name);
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
                0, // line number - would need span info from AST
                0, // column number - would need span info from AST
                Severity::Warning,
            );
        }
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Scenario(scenario) => self.visit_scenario(scenario),
            Statement::TestCase(test) => self.visit_test_case(test),
            Statement::SettingsDef(settings) => self.visit_settings(settings),
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
            let (line, column) = settings
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
                column,
                Severity::Error,
            );
        }

        if settings.timeout_seconds > 300 {
            let (line, column) = settings
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
                column,
                Severity::Error,
            );
        }

        // No need to lint report_format or report_path, as the parser catches errors earlier

        // shell_path errors are catched in the parser as well

        // Warn if stop_on_failure is enabled
        if settings.stop_on_failure {
            let (line, column) = settings
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
                column,
                Severity::Warning,
            );
        }

        // Validate expected_failures
        if settings.expected_failures > 100 {
            let (line, column) = settings
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
                column,
                Severity::Warning,
            );
        }
    }

    fn visit_scenario(&mut self, scenario: &Scenario) {
        let (line, column) = scenario
            .span
            .as_ref()
            .map_or((0, 0), |s| (s.line, s.column));
        println!("Scenario: {}", scenario.name);

        // Rule W001: Check for empty scenarios.
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
                column,
                Severity::Warning,
            );
        }

        // Rule W008: Check for duplicate scenario names.
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
                column,
                Severity::Warning,
            );
        }

        // Rule W009: Check if setup actions exist without a corresponding cleanup.
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
                column,
                Severity::Warning,
            );
        }

        for test in &scenario.tests {
            self.visit_test_case(test);
        }
    }

    fn visit_test_case(&mut self, test: &TestCase) {
        let (line, column) = test.span.as_ref().map_or((0, 0), |s| (s.line, s.column));
        println!("Test: {}", test.name);
    }

    fn visit_given_step(&mut self, step: &GivenStep) {
        todo!()
    }

    fn visit_action(&mut self, action: &Action) {
        todo!()
    }

    fn visit_condition(&mut self, condition: &Condition) {
        todo!()
    }

    fn visit_background(&mut self, steps: &Vec<GivenStep>) {
        todo!()
    }

    fn visit_env_def(&mut self, vars: &Vec<String>) {
        todo!()
    }

    fn visit_var_def(&mut self, name: &String, value: &Value) {
        todo!()
    }

    fn visit_actor_def(&mut self, actors: &Vec<String>) {
        todo!()
    }

    fn visit_feature_def(&mut self, name: &String) {
        todo!()
    }
}

impl Linter {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            defined_vars: HashSet::new(),
            used_vars: HashSet::new(),
            seen_scenario_names: HashSet::new(),
        }
    }

    pub fn lint(&mut self, suite: &TestSuite) -> &Vec<Diagnostic> {
        self.diagnostics.clear();
        self.visit_test_suite(suite);
        &self.diagnostics
    }

    // Use a custom formatted message but keep the rule code
    fn add_diagnostic(
        &mut self,
        rule: &DiagnosticRule,
        message: &str,
        line: usize,
        column: usize,
        severity: Severity,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_id: rule.code.to_string(),
            message: message.to_string(),
            line,
            column,
            severity,
        });
    }

    pub fn get_diagnostics(&self) -> &Vec<Diagnostic> {
        &self.diagnostics
    }
}

///Helper function to check if a scenario contains file system creation actions.
/// is_filesystem_creation is a helper function on the Action struct itself
fn scenario_has_setup_actions(scenario: &Scenario) -> bool {
    for test in &scenario.tests {
        let steps_to_check: Vec<_> = test
            .given
            .iter()
            .filter_map(|s| match s {
                GivenStep::Action(a) => Some(a),
                _ => None,
            })
            .chain(test.when.iter())
            .collect();

        for action in steps_to_check {
            if action.is_filesystem_creation() {
                return true;
            }
        }
    }
    false
}
