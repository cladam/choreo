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
}

#[derive(Debug, Clone)]
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
    OutcomeDef(Vec<String>),
    Rule(Rule),
    TestCase(TestCase),
}

// The core logic block.
pub struct Rule {
    pub name: String,
    pub when: Vec<Condition>,
    pub then: Vec<Action>,
}

// All possible conditions that can trigger a rule.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
