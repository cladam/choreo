use crate::parser::ast::{Action, Condition, GivenStep, Scenario, Statement, TestCase, TestSuite};
// A simplified example of the structure
struct Linter {
    diagnostics: Vec<Diagnostic>,
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
    pub const TEST_NO_WHEN_ACTIONS: DiagnosticRule = DiagnosticRule {
        code: "E002",
        message: "Test has no when actions",
    };
    pub const TEST_NO_THEN_CONDITIONS: DiagnosticRule = DiagnosticRule {
        code: "E003",
        message: "Test has no then conditions",
    };
    pub const RUN_COMMAND_EMPTY: DiagnosticRule = DiagnosticRule {
        code: "E004",
        message: "Run command cannot be empty",
    };
    pub const FILE_PATH_EMPTY: DiagnosticRule = DiagnosticRule {
        code: "E005",
        message: "File path cannot be empty",
    };
    pub const WAIT_TIME_NON_POSITIVE: DiagnosticRule = DiagnosticRule {
        code: "E006",
        message: "Wait time must be positive",
    };
    pub const INVALID_HTTP_STATUS: DiagnosticRule = DiagnosticRule {
        code: "E007",
        message: "Invalid HTTP status code",
    };
    pub const JSON_PATH_EMPTY: DiagnosticRule = DiagnosticRule {
        code: "E008",
        message: "JSON path cannot be empty",
    };
    pub const REPORT_PATH_EMPTY: DiagnosticRule = DiagnosticRule {
        code: "E009",
        message: "Report path cannot be empty",
    };
    pub const SHELL_PATH_EMPTY: DiagnosticRule = DiagnosticRule {
        code: "E010",
        message: "Shell path cannot be empty",
    };

    // Warning codes (W) - Potential issues
    pub const SCENARIO_NO_TESTS: DiagnosticRule = DiagnosticRule {
        code: "W001",
        message: "Scenario has no test cases",
    };
    pub const TEST_NO_GIVEN_STEPS: DiagnosticRule = DiagnosticRule {
        code: "W002",
        message: "Test has no given steps",
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
    pub const SHELL_PATH_RELATIVE: DiagnosticRule = DiagnosticRule {
        code: "W006",
        message: "Shell path should be absolute",
    };
    pub const EXPECTED_FAILURES_HIGH: DiagnosticRule = DiagnosticRule {
        code: "W007",
        message: "Expected failures count seems unusually high",
    };
    pub const STOP_ON_FAILURE_ENABLED: DiagnosticRule = DiagnosticRule {
        code: "W008",
        message: "Stop on failure is enabled - tests will halt on first failure",
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
    fn visit_scenario(&mut self, scenario: &Scenario);
    fn visit_test_case(&mut self, test: &TestCase);
    fn visit_given_step(&mut self, step: &GivenStep);
    fn visit_action(&mut self, action: &Action);
    fn visit_condition(&mut self, condition: &Condition);
}

impl Visitor for Linter {
    fn visit_test_suite(&mut self, suite: &TestSuite) {
        for statement in &suite.statements {
            self.visit_statement(statement);
        }
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Scenario(scenario) => self.visit_scenario(scenario),
            Statement::TestCase(test) => self.visit_test_case(test),
            Statement::SettingsDef(settings) => {
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
                            "{}: {} (line: {}, column: {})",
                            DiagnosticCodes::TIMEOUT_ZERO.code,
                            DiagnosticCodes::TIMEOUT_ZERO.message,
                            line,
                            column
                        ),
                        line,
                        column,
                        Severity::Error,
                    );
                }

                if settings.timeout_seconds > 300 {
                    self.add_diagnostic(
                        &DiagnosticCodes::TIMEOUT_EXCESSIVE,
                        &format!(
                            "{}: {}",
                            DiagnosticCodes::TIMEOUT_EXCESSIVE.code,
                            DiagnosticCodes::TIMEOUT_EXCESSIVE.message
                        ),
                        0,
                        0,
                        Severity::Error,
                    );
                }
                // Validate report_path
                if settings.report_path.is_empty() {
                    self.add_diagnostic(
                        &DiagnosticCodes::REPORT_PATH_EMPTY,
                        &format!(
                            "{}: {}",
                            DiagnosticCodes::REPORT_PATH_EMPTY.code,
                            DiagnosticCodes::REPORT_PATH_EMPTY.message
                        ),
                        0,
                        0,
                        Severity::Error,
                    );
                }
                // Validate shell_path
                if let Some(shell_path) = &settings.shell_path {
                    if shell_path.is_empty() {
                        self.add_diagnostic(
                            &DiagnosticCodes::SHELL_PATH_EMPTY,
                            &format!(
                                "{}: {}",
                                DiagnosticCodes::SHELL_PATH_EMPTY.code,
                                DiagnosticCodes::SHELL_PATH_EMPTY.message
                            ),
                            0,
                            0,
                            Severity::Error,
                        );
                    }
                    if !shell_path.starts_with('/') {
                        self.add_diagnostic(
                            &DiagnosticCodes::SHELL_PATH_RELATIVE,
                            &format!(
                                "{}: {}",
                                DiagnosticCodes::SHELL_PATH_RELATIVE.code,
                                DiagnosticCodes::SHELL_PATH_RELATIVE.message
                            ),
                            0,
                            0,
                            Severity::Warning,
                        );
                    }
                }

                // Warn if stop_on_failure is enabled
                if settings.stop_on_failure {
                    self.add_diagnostic(
                        &DiagnosticCodes::STOP_ON_FAILURE_ENABLED,
                        &format!(
                            "{}: {}",
                            DiagnosticCodes::STOP_ON_FAILURE_ENABLED.code,
                            DiagnosticCodes::STOP_ON_FAILURE_ENABLED.message
                        ),
                        0,
                        0,
                        Severity::Warning,
                    );
                }

                // Validate expected_failures
                if settings.expected_failures > 100 {
                    self.add_diagnostic(
                        &DiagnosticCodes::EXPECTED_FAILURES_HIGH,
                        &format!(
                            "{}: {}",
                            DiagnosticCodes::EXPECTED_FAILURES_HIGH.code,
                            DiagnosticCodes::EXPECTED_FAILURES_HIGH.message
                        ),
                        0,
                        0,
                        Severity::Warning,
                    );
                }
            }
            _ => {}
        }
    }

    fn visit_scenario(&mut self, scenario: &Scenario) {
        todo!()
    }

    fn visit_test_case(&mut self, test: &TestCase) {
        todo!()
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
}

impl Linter {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
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
