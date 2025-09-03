use pest::Parser;
use pest_derive::Parser;
use crate::ast::TestSuite;

#[derive(Parser)]
#[grammar = "choreo.pest"]
pub struct ChoreoParser;

/// Parses a source string into an Abstract Syntax Tree (AST).
pub fn parse(source: &str) -> Result<TestSuite, pest::error::Error<Rule>> {
    println!("Parsing was successful! (AST construction logic is a TODO)");
    // Return a dummy AST for now.
    Ok(TestSuite { statements: vec![] })
}