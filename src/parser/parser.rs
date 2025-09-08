use crate::parser::ast::{
    Action, Condition, GivenStep, Scenario, Statement, TestCase, TestSuite, Value,
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

        statements.push(match pair.as_rule() {
            // Now this will correctly match on the inner rule.
            Rule::actors_def => build_actors_def(pair),
            Rule::settings_def => build_setting(pair),
            Rule::env_def => build_env_def(pair),
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

pub fn build_conditions(pairs: Pairs<Rule>) -> Vec<Condition> {
    pairs
        .map(|pair| build_condition(pair.into_inner().next().unwrap()))
        .collect()
}

// This is the other key function to implement.
fn build_actions(pairs: Pairs<Rule>) -> Vec<Action> {
    pairs
        .map(|pair| {
            let inner_action = pair.into_inner().next().unwrap();
            match inner_action.as_rule() {
                Rule::type_action => {
                    let mut inner = inner_action.into_inner();
                    let actor = inner.next().unwrap().as_str().to_string();
                    let content = inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
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
                        .unwrap()
                        .as_str()
                        .to_string();
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
                        .unwrap()
                        .as_str()
                        .to_string();
                    Action::Run { actor, command }
                }
                _ => unreachable!(),
            }
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

    let given_block = inner.next().unwrap();
    let when_block = inner.next().unwrap();
    let then_block = inner.next().unwrap();

    TestCase {
        name,
        description,
        // Use the new helper function to correctly build the `given` block.
        given: build_given_steps(given_block.into_inner()),
        when: build_actions(when_block.into_inner()),
        then: build_conditions(then_block.into_inner()),
    }
}

/// Builds a vector of GivenSteps, which can be either an Action or a Condition.
pub fn build_given_steps(pairs: Pairs<Rule>) -> Vec<GivenStep> {
    pairs
        .map(|pair| match pair.as_rule() {
            Rule::action => GivenStep::Action(build_action(pair.into_inner().next().unwrap())),
            Rule::condition => {
                GivenStep::Condition(build_condition(pair.into_inner().next().unwrap()))
            }
            _ => unreachable!("Invalid rule inside given block"),
        })
        .collect()
}

// --- Single Item Build Functions ---

/// Builds a single Action from a parsed Pair.
pub fn build_action(inner_action: Pair<Rule>) -> Action {
    match inner_action.as_rule() {
        Rule::type_action => {
            let mut inner = inner_action.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
            let content = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
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
                .unwrap()
                .as_str()
                .to_string();
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
                .unwrap()
                .as_str()
                .to_string();
            Action::Run { actor, command }
        }
        Rule::filesystem_action => {
            let mut inner = inner_action.into_inner();
            // The first pair is now the captured keyword.
            let keyword = inner.next().unwrap().as_str();
            println!("Filesystem action keyword: {}", keyword);
            // The next element is the path, which is a string.
            // need to debug here to see the structure
            println!("Filesystem action inner: {:?}", inner);
            println!("Filesystem action next element: {:?}", inner.clone().next());
            // We can safely unwrap here because the grammar guarantees these elements exist.
            println!("Filesystem action next element: {:?}", inner);
            // The second pair is the path string.
            let path = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();

            match keyword {
                "create_dir" => Action::CreateDir { path },
                "delete_file" => Action::DeleteFile { path },
                "delete_dir" => Action::DeleteDir { path },
                "create_file" => {
                    // The 'with_content' part is optional.
                    let content = if let Some(content_pair) = inner.next() {
                        // If it exists, parse it.
                        content_pair
                            .into_inner()
                            .next()
                            .unwrap()
                            .as_str()
                            .to_string()
                    } else {
                        // Otherwise, default to an empty string.
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

/// Builds a single Condition from a parsed Pair.
pub fn build_condition(inner_cond: Pair<Rule>) -> Condition {
    match inner_cond.as_rule() {
        Rule::time_condition => {
            let mut inner = inner_cond.into_inner();
            let op = inner.next().unwrap().as_str().to_string();
            let time_str = inner.next().unwrap().as_str();
            let time: f32 = time_str[..time_str.len() - 1].parse().unwrap();
            Condition::Time { op, time }
        }
        Rule::output_contains_condition => {
            let mut inner = inner_cond.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
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
            let mut inner = inner_cond.into_inner();
            let outcome = inner.next().unwrap().as_str().to_string();
            Condition::StateSucceeded { outcome }
        }
        Rule::output_matches_condition => {
            let mut inner = inner_cond.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
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
                actor,
                regex,
                capture_as,
            }
        }
        Rule::last_command_cond => {
            // This is the corrected logic.
            // We look at the inner content of the rule to determine the variant.
            let mut inner = inner_cond.into_inner();
            let choice_pair = inner.next().unwrap();

            // Check the simple keyword cases first by matching their string content.
            if choice_pair.as_str() == "succeeded" {
                return Condition::LastCommandSucceeded;
            }
            if choice_pair.as_str() == "failed" {
                return Condition::LastCommandFailed;
            }

            // If it's neither, it must be the `exit_code_is` pair.
            // This pair will contain the `number` as its inner pair.
            let mut exit_code_inner = choice_pair.into_inner();
            let code_str = exit_code_inner.next().unwrap().as_str();
            let code: i32 = code_str.parse().unwrap();
            Condition::LastCommandExitCodeIs(code)
        }
        Rule::filesystem_condition => {
            let mut inner = inner_cond.into_inner();
            let keyword = inner.next().unwrap().as_str();
            let path = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();

            match keyword {
                "file_exists" => Condition::FileExists { path },
                "dir_exists" => Condition::DirExists { path },
                "file_contains" => {
                    let content = inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    Condition::FileContains { path, content }
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!("Unhandled condition: {:?}", inner_cond.as_rule()),
    }
}
