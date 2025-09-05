// Represents the entire parsed test file.
pub struct TestSuite {
    pub statements: Vec<Statement>,
}

// All possible top-level statements in a .chor file.
pub enum Statement {
    Setting(String, Value),
    EnvDef(Vec<String>),
    ActorDef(Vec<String>),
    OutcomeDef(Vec<String>),
    Rule(Rule),
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
    Time { op: String, time: f32 },
    OutputContains { actor: String, text: String },
    StateSucceeded { outcome: String },
    OutputMatches { actor: String, regex: String, capture_as: Option<String> },
}

// All possible actions that can be executed.
#[derive(Debug, Clone)]
pub enum Action {
    Type { actor: String, content: String },
    Press { actor: String, key: String },
    Run { actor: String, command: String },
    Succeeds { outcome: String },
}

// Primitive values.
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(i32),
}
