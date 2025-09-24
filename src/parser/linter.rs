use crate::parser::ast::{Action, Condition, GivenStep, Scenario, Statement, TestCase, TestSuite};
// A simplified example of the structure
struct Linter {
    diagnostics: Vec<Diagnostic>,
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
        todo!()
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

    fn add_diagnostic(
        &mut self,
        rule_id: &str,
        message: &str,
        line: usize,
        column: usize,
        severity: Severity,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_id: rule_id.to_string(),
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
