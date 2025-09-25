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

    // Info codes (I) - Informational
    pub const BEST_PRACTICE_SUGGESTION: DiagnosticRule = DiagnosticRule {
        code: "I001",
        message: "Consider using best practices",
    };
}

struct Diagnostic {
    rule_id: String,
    message: String,
    line: usize,
    column: usize,
    severity: Severity,
}

#[derive(Debug, PartialEq)]
pub enum Severity {
    Warning,
    Error,
    Info,
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
                if settings.timeout_seconds == 0 {
                    self.add_diagnostic(
                        &DiagnosticCodes::TIMEOUT_ZERO,
                        &format!(
                            "{}: {}",
                            DiagnosticCodes::TIMEOUT_ZERO.code,
                            DiagnosticCodes::TIMEOUT_ZERO.message
                        ),
                        0,
                        0,
                        Severity::Error,
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
