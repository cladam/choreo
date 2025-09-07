// Represents the entire parsed test file.
pub struct TestSuite {
    pub statements: Vec<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TestState {
    Pending,
    Running,
    Passed,
    Failed(String),
    Skipped,
}

impl TestState {
    pub fn is_done(&self) -> bool {
        matches!(self, TestState::Passed | TestState::Failed(_))
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, TestState::Failed(_))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub given: Vec<Condition>,
    pub when: Vec<Action>,
    pub then: Vec<Condition>,
}

// All possible top-level statements in a .chor file.
pub enum Statement {
    Setting(String, Value),
    EnvDef(Vec<String>),
    ActorDef(Vec<String>),
    FeatureDef(String),
    Scenario(Scenario),
    TestCase(TestCase),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub name: String,
    pub tests: Vec<TestCase>,
}

// All possible conditions that can trigger a rule.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Time {
        op: String,
        time: f32,
    },
    OutputContains {
        actor: String,
        text: String,
    },
    StateSucceeded {
        outcome: String,
    },
    OutputMatches {
        actor: String,
        regex: String,
        capture_as: Option<String>,
    },
}

// All possible actions that can be executed.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Type { actor: String, content: String },
    Press { actor: String, key: String },
    Run { actor: String, command: String },
}

// Primitive values.
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(i32),
}
