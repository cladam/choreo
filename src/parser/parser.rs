// parser.rs
use crate::parser::ast::{
    Action, Condition, GivenStep, Scenario, Statement, TestCase, TestSuite, Value,
};
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;

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
        println!("Parsed statement: {:?}", statement_pair);

        // This opens the "statement" container to get the content.
        let pair = statement_pair.into_inner().next().unwrap();
        println!("Parsed pair: {:?}", pair);

        statements.push(match pair.as_rule() {
            // Now this will correctly match on the inner rule.
            Rule::actors_def => build_actors_def(pair),
            Rule::settings_def => build_setting(pair),
            Rule::env_def => build_env_def(pair),
            Rule::vars_def => {
                let mut vars = HashMap::new();
                for assignment in pair.into_inner() {
                    let mut inner = assignment.into_inner();
                    let key = inner.next().unwrap().as_str().to_string();
                    let value = build_value(inner.next().unwrap());
                    vars.insert(key, value);
                }
                Statement::VarsDef(vars)
            }
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
fn build_setting(pair: Pair<Rule>) -> Statement {
    // Extract setting name and value
    let mut inner_rules = pair.into_inner();
    let name = inner_rules.next().unwrap().as_str().to_string();
    let value_pair = inner_rules.next().unwrap();
    let value = match value_pair.as_rule() {
        Rule::string => Value::String(value_pair.as_str().to_string()),
        Rule::number => Value::Number(value_pair.as_str().parse().unwrap()),
        _ => unreachable!(),
    };
    Statement::Setting(name, value)
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

fn build_vars_def(pair: Pair<Rule>) -> Statement {
    let mut vars = HashMap::new();
    for assignment in pair.into_inner() {
        if assignment.as_rule() == Rule::var_assignment {
            let mut inner = assignment.into_inner();
            let key = inner.next().unwrap().as_str().to_string();
            let value = build_value(inner.next().unwrap());
            vars.insert(key, value);
        }
    }
    Statement::VarsDef(vars)
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
    for test_pair in inner {
        // Corrected to check for the 'test' rule name from your grammar
        if test_pair.as_rule() == Rule::test {
            tests.push(build_test_case(test_pair));
        }
    }
    Statement::Scenario(Scenario { name, tests })
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
    println!("Building condition from specific: {:?}", inner_cond);
    match inner_cond.as_rule() {
        Rule::time_condition => {
            let mut inner = inner_cond.into_inner();
            let op = inner.next().unwrap().as_str().to_string();
            let time_str = inner.next().unwrap().as_str();
            let time: f32 = time_str[..time_str.len() - 1].parse().unwrap();
            Condition::Time { op, time }
        }
        Rule::terminal_condition => {
            // A terminal_condition contains one of the specific terminal condition types.
            let specific_terminal_cond = inner_cond.into_inner().next().unwrap();
            build_condition_from_specific(specific_terminal_cond)
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
    println!("Building action for inner_action: {:?}", inner_action);
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
    match pair.as_rule() {
        Rule::string => {
            let inner = pair.into_inner().next().unwrap();
            Value::String(unescape_string(inner.as_str()))
        }
        Rule::number => Value::Number(pair.as_str().parse().unwrap()),
        _ => unreachable!(),
    }
}

/// Unescapes a string captured by the parser.
fn unescape_string(s: &str) -> String {
    s.replace("\\\"", "\"").replace("\\'", "'")
}
