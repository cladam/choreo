// parser.rs
use crate::parser::ast::{
    Action, Condition, GivenStep, ReportFormat, Scenario, Statement, TestCase, TestSuite,
    TestSuiteSettings, Value,
};
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser, Debug)]
#[grammar = "parser/choreo.pest"]
pub struct ChoreoParser;

/// Parses a source string into an Abstract Syntax Tree (AST).
pub fn parse(source: &str) -> Result<TestSuite, pest::error::Error<Rule>> {
    let pairs = ChoreoParser::parse(Rule::grammar, source)?.next().unwrap();

    let mut statements = vec![];
    for statement_pair in pairs.into_inner() {
        if statement_pair.as_rule() == Rule::EOI {
            break;
        }
        //println!("Parsed statement: {:?}", statement_pair);

        // This opens the "statement" container to get the content.
        let pair = statement_pair.into_inner().next().unwrap();
        //println!("Parsed pair: {:?}", pair);

        statements.push(match pair.as_rule() {
            // Now this will correctly match on the inner rule.
            Rule::actors_def => build_actors_def(pair),
            Rule::settings_def => build_settings_def(pair),
            Rule::env_def => build_env_def(pair),
            Rule::vars_def => build_vars_def(pair),
            Rule::feature_def => build_feature_def(pair),
            Rule::scenario_def => build_scenario(pair),
            _ => unimplemented!("Parser rule not handled: {:?}", pair.as_rule()),
        });
    }

    Ok(TestSuite { statements })
}

// Helper function for a simple definition.
fn build_actors_def(pair: Pair<Rule>) -> Statement {
    let identifiers: Vec<String> = pair
        .into_inner()
        .filter(|p| p.as_rule() == Rule::identifier)
        .map(|p| p.as_str().to_string())
        .collect();
    Statement::ActorDef(identifiers)
}

// Helper function for settings
fn build_settings_def(pair: Pair<Rule>) -> Statement {
    let mut settings = TestSuiteSettings::default();
    for setting_pair in pair.into_inner() {
        let mut inner = setting_pair.into_inner();
        let key = inner.next().unwrap().as_str();
        let value_pair = inner.next().unwrap();
        let value = build_value(value_pair);

        match key {
            "timeout_seconds" => {
                if let Value::Number(n) = value {
                    settings.timeout_seconds = n as u64;
                } else {
                    // You might want to return a proper parse error here
                    panic!("'timeout_seconds' setting must be a number");
                }
            }
            "report_path" => {
                if let Value::String(s) = value {
                    settings.report_path = s;
                } else {
                    panic!("'report_path' setting must be a string");
                }
            }
            "report_format" => {
                if let Value::String(s) = value {
                    settings.report_format = match s.as_str() {
                        "json" => ReportFormat::Json,
                        "junit" => ReportFormat::Junit,
                        _ => panic!("Invalid 'report_format': must be 'json' or 'junit'"),
                    };
                } else {
                    panic!("'report_format' setting must be a string");
                }
            }
            _ => { /* Ignore unknown settings */ }
        }
    }
    Statement::SettingsDef(settings)
}

fn build_vars_def(pair: Pair<Rule>) -> Statement {
    let vars = pair
        .into_inner()
        .map(|var_pair| {
            let mut inner = var_pair.into_inner();
            let key = inner.next().unwrap().as_str().to_string();
            let value_pair = inner.next().unwrap(); // This is a `string` rule
            let value_str = value_pair.into_inner().next().unwrap().as_str();
            (key, Value::String(unescape_string(value_str)))
        })
        .collect();
    Statement::VarsDef(vars)
}

// Helper function for a feature definition.
fn build_feature_def(pair: Pair<Rule>) -> Statement {
    let name = pair
        .into_inner()
        .next()
        .unwrap()
        .into_inner()
        .next()
        .unwrap()
        .as_str()
        .to_string();
    Statement::FeatureDef(name)
}

fn build_env_def(pair: Pair<Rule>) -> Statement {
    let identifiers: Vec<String> = pair.into_inner().map(|p| p.as_str().to_string()).collect();
    Statement::EnvDef(identifiers)
}

// This is the other key function to implement.
fn build_actions(pairs: Pairs<Rule>) -> Vec<Action> {
    pairs
        .map(|pair| {
            // An 'action' rule from the grammar contains one of the specific action types.
            let inner_action = pair.into_inner().next().unwrap();
            build_action(inner_action)
        })
        .collect()
}

fn build_scenario(pair: Pair<Rule>) -> Statement {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .unwrap()
        .into_inner()
        .next()
        .unwrap()
        .as_str()
        .to_string();
    let mut tests = Vec::new();
    let mut after = Vec::new();
    for item in inner {
        match item.as_rule() {
            Rule::test => tests.push(build_test_case(item)),
            Rule::after_block => {
                after = build_actions(item.into_inner());
            }
            _ => {}
        }
    }
    Statement::Scenario(Scenario { name, tests, after })
}

/// Builds a TestCase from a parsed Pair.
pub fn build_test_case(pair: Pair<Rule>) -> TestCase {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let description = inner
        .next()
        .unwrap()
        .into_inner()
        .next()
        .unwrap()
        .as_str()
        .to_string();

    let given_block = inner.next().expect("Missing given block in test case");
    let when_block = inner.next().expect("Missing when block in test case");
    let then_block = inner.next().expect("Missing then block in test case");

    TestCase {
        name,
        description,
        given: build_given_steps(given_block.into_inner()),
        when: build_actions(when_block.into_inner()),
        then: build_conditions(then_block.into_inner()),
    }
}

// Builds a vector of GivenSteps, which can be either an Action or a Condition.
pub fn build_given_steps(pairs: Pairs<Rule>) -> Vec<GivenStep> {
    pairs
        .map(|pair| {
            // The pair will be either an 'action' or a 'condition'.
            match pair.as_rule() {
                Rule::action => {
                    // The 'action' rule contains one of the specific action types.
                    let specific_action = pair.into_inner().next().unwrap();
                    GivenStep::Action(build_action(specific_action))
                }
                Rule::condition => GivenStep::Condition(build_condition(pair)),
                _ => unreachable!("Unexpected rule in given block: {:?}", pair.as_rule()),
            }
        })
        .collect()
}

/// Builds a single Condition from a specific, inner condition rule Pair.
/// This is a helper to avoid unwrapping the 'condition' rule multiple times.
pub fn build_condition_from_specific(inner_cond: Pair<Rule>) -> Condition {
    //println!("Building condition from specific: {:?}", inner_cond);
    match inner_cond.as_rule() {
        Rule::wait_condition => {
            let mut inner = inner_cond.into_inner();
            let op = inner.next().unwrap().as_str().to_string();
            let wait_marker_str = inner.next().unwrap().as_str();

            let wait = if wait_marker_str.ends_with("ms") {
                let value_str = &wait_marker_str[..wait_marker_str.len() - 2];
                value_str.parse::<f32>().unwrap() / 1000.0
            } else if wait_marker_str.ends_with('s') {
                let value_str = &wait_marker_str[..wait_marker_str.len() - 1];
                value_str.parse::<f32>().unwrap()
            } else {
                // This case should not be reached if the grammar is correct
                0.0
            };
            Condition::Wait { op, wait }
        }
        Rule::terminal_condition => {
            let mut inner = inner_cond.into_inner();
            let terminal_cond = inner.next().unwrap();
            build_condition_from_specific(terminal_cond)
        }
        Rule::output_contains_condition => {
            let mut inner = inner_cond.into_inner();
            //let actor = inner.next().unwrap().as_str().to_string();
            let actor = "Terminal".to_string(); // Default actor for terminal conditions
            let text = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            Condition::OutputContains { actor, text }
        }
        Rule::state_condition => {
            let outcome = inner_cond.into_inner().next().unwrap().as_str().to_string();
            Condition::StateSucceeded { outcome }
        }
        Rule::output_matches_condition => {
            let mut inner = inner_cond.into_inner();
            //let actor = inner.next().unwrap().as_str().to_string();
            let regex = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            let capture_as = inner.next().map(|p| p.as_str().to_string());
            Condition::OutputMatches {
                actor: "Terminal".to_string(),
                regex,
                capture_as,
            }
        }
        Rule::last_command_succeeded_cond => Condition::LastCommandSucceeded,
        Rule::last_command_failed_cond => Condition::LastCommandFailed,
        Rule::last_command_exit_code_is_cond => {
            let mut inner = inner_cond.into_inner();
            let code_str = inner.next().unwrap().as_str();
            let code: i32 = code_str.parse().unwrap();
            Condition::LastCommandExitCodeIs(code)
        }
        Rule::filesystem_condition => {
            let mut inner = inner_cond.into_inner();
            //let _actor = inner.next().unwrap().as_str(); // Consume the actor identifier
            let keyword = inner.next().unwrap().as_str();
            let path = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .map_or(String::new(), |p| p.as_str().to_string());

            match keyword {
                "file_exists" => Condition::FileExists { path },
                "file_does_not_exist" => Condition::FileDoesNotExist { path },
                "dir_exists" => Condition::DirExists { path },
                "file_contains" => {
                    let content = inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .map_or(String::new(), |p| p.as_str().to_string());
                    Condition::FileContains { path, content }
                }
                _ => unreachable!(),
            }
        }
        Rule::stdout_is_empty_condition => Condition::StdoutIsEmpty,
        Rule::stderr_contains_condition => {
            let text = unescape_string(
                inner_cond
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str(),
            );
            Condition::StderrContains(text)
        }
        Rule::output_starts_with_condition => {
            let text = unescape_string(
                inner_cond
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str(),
            );
            println!("Building output_starts_with_condition '{}'", text);
            Condition::OutputStartsWith(text)
        }
        Rule::output_ends_with_condition => {
            let text = unescape_string(
                inner_cond
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str(),
            );
            Condition::OutputEndsWith(text)
        }
        Rule::output_equals_condition => {
            let text = unescape_string(
                inner_cond
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str(),
            );
            Condition::OutputEquals(text)
        }
        _ => unreachable!("Unhandled condition: {:?}", inner_cond.as_rule()),
    }
}

/// Builds a single Condition from a parsed Pair.
pub fn build_condition(pair: Pair<Rule>) -> Condition {
    // A 'condition' pair from the grammar contains one of the specific condition types.
    let inner_cond = pair.into_inner().next().unwrap();
    build_condition_from_specific(inner_cond)
}

/// Builds a vector of Conditions from parsed Pairs.
fn build_conditions(pairs: Pairs<Rule>) -> Vec<Condition> {
    pairs.map(build_condition).collect()
}

// --- Single Item Build Functions ---

/// Builds a single Action from a parsed Pair.
/// Builds a single Action from a parsed Pair.
pub fn build_action(inner_action: Pair<Rule>) -> Action {
    //println!("Building action for inner_action: {:?}", inner_action);
    match inner_action.as_rule() {
        Rule::type_action => {
            let mut inner = inner_action.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
            let content = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .map_or(String::new(), |p| p.as_str().to_string());
            Action::Type { actor, content }
        }
        Rule::press_action => {
            let mut inner = inner_action.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
            let key = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .map_or(String::new(), |p| p.as_str().to_string());
            Action::Press { actor, key }
        }
        Rule::run_action => {
            let mut inner = inner_action.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
            let command = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .map_or(String::new(), |p| p.as_str().to_string());
            Action::Run { actor, command }
        }
        Rule::filesystem_action => {
            let mut inner = inner_action.into_inner();
            //let _actor = inner.next().unwrap().as_str(); // Consume the actor identifier
            let keyword = inner.next().unwrap().as_str();
            let path = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .map_or(String::new(), |p| p.as_str().to_string());

            match keyword {
                "create_dir" => Action::CreateDir { path },
                "delete_file" => Action::DeleteFile { path },
                "delete_dir" => Action::DeleteDir { path },
                "create_file" => {
                    let content = if let Some(content_pair) = inner.next() {
                        content_pair
                            .into_inner()
                            .next()
                            .map_or(String::new(), |p| p.as_str().to_string())
                    } else {
                        String::new()
                    };
                    Action::CreateFile { path, content }
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!("Unhandled action: {:?}", inner_action.as_rule()),
    }
}

fn build_value(pair: Pair<Rule>) -> Value {
    // The `value` rule is silent, so we need to inspect its inner pair.
    let inner_pair = pair.into_inner().next().unwrap();
    match inner_pair.as_rule() {
        Rule::string => {
            let inner = inner_pair.into_inner().next().unwrap();
            Value::String(unescape_string(inner.as_str()))
        }
        Rule::number => Value::Number(inner_pair.as_str().parse().unwrap()),
        _ => unreachable!("Unexpected value rule: {:?}", inner_pair.as_rule()),
    }
}

/// Unescapes a string captured by the parser.
fn unescape_string(s: &str) -> String {
    s.replace("\\\"", "\"")
        .replace("\\'", "'")
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
        .replace("\\\\", "\\")
}
