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
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioSpan {
    pub name_span: Option<Span>,
    pub tests_span: Option<Span>,
    pub after_span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingSpan {
    pub timeout_seconds_span: Option<Span>,
    pub report_path_span: Option<Span>,
    pub report_format_span: Option<Span>,
    pub shell_path_span: Option<Span>,
    pub stop_on_failure_span: Option<Span>,
    pub expected_failures_span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestSuiteSettings {
    pub timeout_seconds: u64,
    pub report_format: ReportFormat,
    pub report_path: String,
    pub stop_on_failure: bool,
    pub shell_path: Option<String>,
    pub expected_failures: usize,
    pub span: Option<Span>,
    pub setting_spans: Option<SettingSpan>,
}

impl Default for TestSuiteSettings {
    fn default() -> Self {
        Self {
            timeout_seconds: 60,
            report_format: ReportFormat::Json,
            report_path: "reports/".to_string(),
            stop_on_failure: false,
            shell_path: Option::from("/bin/sh".to_string()),
            expected_failures: 0,
            span: None,
            setting_spans: None,
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
    pub span: Option<Span>,
    pub testcase_spans: Option<TestCaseSpan>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestCaseSpan {
    pub name_span: Option<Span>,
    pub description_span: Option<Span>,
    pub given_span: Option<Span>,
    pub when_span: Option<Span>,
    pub then_span: Option<Span>,
}

impl Default for TestCase {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            given: Vec::new(),
            when: Vec::new(),
            then: Vec::new(),
            span: None,
            testcase_spans: None,
        }
    }
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
    pub parallel: bool,
    pub span: Option<Span>,
    pub scenario_span: Option<ScenarioSpan>,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            name: String::new(),
            tests: Vec::new(),
            after: Vec::new(),
            parallel: false,
            span: None,
            scenario_span: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GivenStep {
    Action(Action),
    Condition(Condition),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StateCondition {
    HasSucceeded(String),
    CanStart,
}

// All possible conditions that can trigger a rule.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    // deprecated
    Wait {
        op: String,
        wait: f32,
    },
    // deprecated
    StateSucceeded {
        outcome: String,
    },
    State(StateCondition),
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
    // --- JSON Conditions ---
    OutputIsValidJson,
    JsonValueIsString {
        path: String,
    },
    JsonValueIsNumber {
        path: String,
    },
    JsonValueIsArray {
        path: String,
    },
    JsonValueIsObject {
        path: String,
    },
    JsonValueHasSize {
        path: String,
        size: usize,
    },
    JsonOutputHasPath {
        path: String,
    },
    JsonOutputAtEquals {
        path: String,
        value: Value,
    },
    JsonOutputAtIncludes {
        path: String,
        value: Value,
    },
    JsonOutputAtHasItemCount {
        path: String,
        count: i32,
    },
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
    // --- Web Conditions ---
    ResponseStatusIs(u16),
    ResponseStatusIsSuccess,
    ResponseStatusIsError,
    ResponseStatusIsIn(Vec<u16>),
    ResponseTimeIsBelow {
        duration: f32,
    },
    ResponseBodyContains {
        value: String,
    },
    ResponseBodyMatches {
        regex: String,
        capture_as: Option<String>,
    },
    ResponseBodyEqualsJson {
        expected: String,
        ignored: Vec<String>,
    },
    JsonBodyHasPath {
        path: String,
    },
    JsonPathEquals {
        path: String,
        expected_value: Value,
    },
    JsonPathCapture {
        path: String,
        capture_as: String,
    },
}

// All possible actions that can be executed.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // --- System Actions ---
    Pause {
        duration: f32,
    },
    Log {
        message: String,
    },
    Timestamp {
        variable: String,
    },
    Uuid {
        variable: String,
    },
    // --- Terminal Actions ---
    Run {
        actor: String,
        command: String,
    },
    // --- Filesystem Actions ---
    CreateFile {
        path: String,
        content: String,
    },
    CreateDir {
        path: String,
    },
    DeleteFile {
        path: String,
    },
    DeleteDir {
        path: String,
    },
    ReadFile {
        path: String,
        variable: Option<String>,
    },
    // --- Web Actions ---
    HttpSetHeader {
        key: String,
        value: String,
    },
    HttpClearHeader {
        key: String,
    },
    HttpClearHeaders,
    HttpSetCookie {
        key: String,
        value: String,
    },
    HttpClearCookie {
        key: String,
    },
    HttpClearCookies,
    HttpGet {
        url: String,
    },
    HttpPost {
        url: String,
        body: String,
    },
    HttpPut {
        url: String,
        body: String,
    },
    HttpPatch {
        url: String,
        body: String,
    },
    HttpDelete {
        url: String,
    },
}

impl Action {
    pub fn is_filesystem_creation(&self) -> bool {
        matches!(self, Self::CreateFile { .. } | Self::CreateDir { .. })
    }
}

// Primitive values.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(i32),
    Bool(bool),
}

impl Value {
    pub fn as_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
        }
    }
}
