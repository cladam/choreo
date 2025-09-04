use crate::ast;
use crate::ast::{Action, Condition, Statement, TestSuite, Value};
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "resources/choreo.pest"]
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
            Rule::outcomes_def => build_outcomes_def(pair),
            Rule::rule => build_rule(pair),
            Rule::settings_def => build_setting(pair),
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

fn build_outcomes_def(pair: Pair<Rule>) -> Statement {
    let identifiers: Vec<String> = pair
        .into_inner()
        .filter(|p| p.as_rule() == Rule::identifier)
        .map(|p| p.as_str().to_string())
        .collect();
    Statement::OutcomeDef(identifiers)
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

fn build_conditions(pairs: Pairs<Rule>) -> Vec<Condition> {
    pairs
        .map(|pair| {
            let inner_cond = pair.into_inner().next().unwrap();
            match inner_cond.as_rule() {
                Rule::time_condition => {
                    let mut inner = inner_cond.into_inner();
                    let op = inner.next().unwrap().as_str().to_string();
                    let time_str = inner.next().unwrap().as_str();
                    let time: f32 = time_str[..time_str.len() - 1].parse().unwrap();
                    Condition::Time { op, time }
                }
                Rule::output_condition => {
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
                _ => unreachable!(),
            }
        })
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
                Rule::test_action => {
                    let mut inner = inner_action.into_inner();
                    let outcome = inner.next().unwrap().as_str().to_string();
                    Action::Succeeds { outcome }
                }
                _ => unreachable!(),
            }
        })
        .collect()
}

// Make sure your build_rule function calls these new helpers:
fn build_rule(pair: Pair<Rule>) -> Statement {
    let mut inner = pair.into_inner();

    let name = inner
        .next()
        .unwrap()
        .into_inner()
        .next()
        .unwrap()
        .as_str()
        .to_string();

    let when_block = inner.next().unwrap();
    let then_block = inner.next().unwrap();

    let rule = ast::Rule {
        name,
        when: build_conditions(when_block.into_inner()), // Use the new function
        then: build_actions(then_block.into_inner()),    // Use the new function
    };
    Statement::Rule(rule)
}
