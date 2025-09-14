// Represents the entire parsed test file
#[derive(Debug, Clone)]
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
pub enum ReportFormat {
    Json,
    Junit,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestSuiteSettings {
    pub timeout_seconds: u64,
    pub report_format: ReportFormat,
    pub report_path: String,
    pub stop_on_failure: bool,
    pub shell_path: Option<String>,
    pub expected_failures: usize,
}

impl Default for TestSuiteSettings {
    fn default() -> Self {
        Self {
            timeout_seconds: 60,
            report_format: ReportFormat::Json,
            report_path: "reports/".to_string(),
            stop_on_failure: false,
            shell_path: Option::from("/bin/bash".to_string()),
            expected_failures: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub given: Vec<GivenStep>,
    pub when: Vec<Action>,
    pub then: Vec<Condition>,
}

// All possible top-level statements in a .chor file.
#[derive(Debug, Clone)]
pub enum Statement {
    SettingsDef(TestSuiteSettings),
    BackgroundDef(Vec<GivenStep>),
    EnvDef(Vec<String>),
    VarDef(String, Value),
    ActorDef(Vec<String>),
    FeatureDef(String),
    Scenario(Scenario),
    TestCase(TestCase),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub name: String,
    pub tests: Vec<TestCase>,
    pub after: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GivenStep {
    Action(Action),
    Condition(Condition),
}

// All possible conditions that can trigger a rule.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Wait {
        op: String,
        wait: f32,
    },
    StateSucceeded {
        outcome: String,
    },
    // --- Terminal Conditions ---
    OutputContains {
        actor: String,
        text: String,
    },
    OutputMatches {
        actor: String,
        regex: String,
        capture_as: Option<String>,
    },
    LastCommandSucceeded,
    LastCommandFailed,
    LastCommandExitCodeIs(i32),
    StdoutIsEmpty,
    StderrIsEmpty,
    StderrContains(String),
    OutputStartsWith(String),
    OutputEndsWith(String),
    OutputEquals(String),
    // --- Filesystem Conditions ---
    FileExists {
        path: String,
    },
    DirExists {
        path: String,
    },
    DirDoesNotExist {
        path: String,
    },
    FileContains {
        path: String,
        content: String,
    },
    FileDoesNotExist {
        path: String,
    },
    FileIsEmpty {
        path: String,
    },
    FileIsNotEmpty {
        path: String,
    },
}

// All possible actions that can be executed.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // --- Terminal Actions ---
    Type { actor: String, content: String },
    Press { actor: String, key: String },
    Run { actor: String, command: String },
    // --- Filesystem Actions ---
    CreateFile { path: String, content: String },
    CreateDir { path: String },
    DeleteFile { path: String },
    DeleteDir { path: String },
}

// Primitive values.
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(i32),
}

impl Value {
    pub fn as_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
        }
    }
}
