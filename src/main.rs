mod ast;
mod parser;

use std::fs;

fn main() {
    println!("Starting Choreo Test Runner...");

    // Read the test file from disk.
    let file_path = "src/resources/test_medi_workflow.chor";
    let source = fs::read_to_string(file_path)
        .expect("Should have been able to read the file");

    println!("Loaded test file: {}", file_path);

    // 2. Parse the source code into an AST.
    match parser::parse(&source) {
        Ok(test_suite) => {
            println!("✅ AST generated successfully.");

            // 3. Hand off the AST to the Test Runner / Simulation Engine.
            // The TestRunner would contain the TerminalBackend and manage the
            // simulation state (time, test outcomes, etc.).
            // run_tests(test_suite);
        }
        Err(e) => {
            eprintln!("❌ Parsing failed!");
            eprintln!("{}", e);
        }
    }
}