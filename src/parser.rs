use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;
use crate::ast;
use crate::ast::{Action, Condition, Statement, TestSuite, Value};

#[derive(Parser)]
#[grammar = "resources/choreo.pest"]
pub struct ChoreoParser;

/// Parses a source string into an Abstract Syntax Tree (AST).
pub fn parse(source: &str) -> Result<TestSuite, pest::error::Error<Rule>> {
    let pairs = ChoreoParser::parse(Rule::grammar, source)?.next().unwrap();

    let mut statements = vec![];
    for pair in pairs.into_inner() {
        // Each pair at this level is a top-level statement.
        if pair.as_rule() == Rule::EOI { break; } // End of input
        println!("Parsed statement: {:?}", pair);

        // All top-level statements are handled here.
        statements.push(match pair.as_rule() {
            Rule::actors_def => build_actors_def(pair),
            Rule::outcomes_def => build_outcomes_def(pair),
            Rule::rule => build_rule(pair),
            Rule::setting => build_setting(pair),
            _ => unimplemented!("Parser rule not handled: {:?}", pair.as_rule()),
        });
    }


    println!("Parsing was successful! (AST construction logic is a TODO)");
    // Return a dummy AST for now.
    Ok(TestSuite { statements: vec![] })
}

// Helper function for a simple definition.
fn build_actors_def(pair: Pair<Rule>) -> Statement {
    let identifiers: Vec<String> = pair.into_inner()
        .filter(|p| p.as_rule() == Rule::identifier)
        .map(|p| p.as_str().to_string())
        .collect();
    Statement::ActorDef(identifiers)
}

fn build_outcomes_def(pair: Pair<Rule>) -> Statement {
    let identifiers: Vec<String> = pair.into_inner()
        .filter(|p| p.as_rule() == Rule::identifier)
        .map(|p| p.as_str().to_string())
        .collect();
    Statement::OutcomeDef(identifiers)
}

// Helper function for a complex rule.
fn build_rule(pair: Pair<Rule>) -> Statement {
    let mut inner = pair.into_inner();

    // The first inner pair is the rule's name (a string).
    let name = inner.next().unwrap().into_inner().next().unwrap().as_str().to_string();

    // The next pairs are the when/then blocks.
    let when_block = inner.next().unwrap();
    let then_block = inner.next().unwrap();

    let rule = ast::Rule {
        name,
        when: build_conditions(when_block.into_inner()),
        then: build_actions(then_block.into_inner()),
    };
    Statement::Rule(rule)
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
    pairs.filter_map(|pair| {
        match pair.as_rule() {
            Rule::time_condition => {
                let mut inner = pair.into_inner();
                inner.next(); // Skip "time" keyword
                let op = inner.next().unwrap().as_str().to_string();
                let time_marker = inner.next().unwrap();
                let time_str = time_marker.into_inner().next().unwrap().as_str();
                let time = time_str.parse().unwrap();
                Some(Condition::Time { op, time })
            }
            Rule::output_condition => {
                let mut inner = pair.into_inner();
                let actor = inner.next().unwrap().as_str().to_string();
                inner.next(); // Skip "output_contains" keyword
                let text = inner.next().unwrap().as_str().to_string();
                Some(Condition::OutputContains { actor, text })
            }
            Rule::state_condition => {
                let mut inner = pair.into_inner();
                inner.next(); // Skip "state" keyword
                inner.next(); // Skip "succeeded" keyword
                let outcome = inner.next().unwrap().as_str().to_string();
                Some(Condition::StateSucceeded { outcome })
            }
            _ => None,
        }
    }).collect()
}

fn build_actions(pairs: Pairs<Rule>) -> Vec<Action> {
    pairs.filter_map(|pair| {
        match pair.as_rule() {
            Rule::type_action => {
                let mut inner = pair.into_inner();
                let actor = inner.next().unwrap().as_str().to_string();
                let content = inner.next().unwrap().as_str().to_string();
                Some(Action::Type { actor, content })
            }
            Rule::press_action => {
                let mut inner = pair.into_inner();
                let actor = inner.next().unwrap().as_str().to_string();
                let key = inner.next().unwrap().as_str().to_string();
                Some(Action::Press { actor, key })
            }
            Rule::run_action => {
                let mut inner = pair.into_inner();
                let actor = inner.next().unwrap().as_str().to_string();
                let command = inner.next().unwrap().as_str().to_string();
                Some(Action::Run { actor, command })
            }
            Rule::test_action => {
                let outcome = pair.into_inner().next().unwrap().as_str().to_string();
                Some(Action::Succeeds { outcome })
            }
            _ => None,
        }
    }).collect()
}