# **choreo**

**A declarative DSL for choreographing and testing command-line workflows.**

Choreo is a test runner and automation tool that uses a simple, readable, rule-based language to define and execute
complex interactions with command-line applications. It's designed for behavior-driven testing, integration testing, and
generating reproducible demos of CLI tools.

Think of it as **Cucumber for the command line**, with the reactive power of a rules engine.

### **Key Features**

* **Declarative BDD Syntax**: Write tests in a simple, human-readable `.chor` format using `feature`, `scenario`, and
  `test` blocks.
* **Reactive Test Execution**: Define `given` conditions that must be met before a test runs, and `then` conditions that
  validate the outcome. Choreo's engine continuously checks these conditions.
* **Multi-Actor System**: Choreograph interactions between different parts of your system.
    * `Terminal`: Spawns a pseudo-terminal (PTY) to run commands, type input, and read output just like a real user.
    * `FileSystem`: Create, delete, and verify files and directories to set up preconditions and assert outcomes.
* **Dynamic State Management**:
    * **Capture & Reuse**: Capture values from terminal output using regex and reuse them in subsequent steps.
    * **Variables**: Define static variables and pull in secrets or configuration from environment variables.
* **Rich Assertions**: A comprehensive set of conditions to validate system state:
    * Terminal output (`output_contains`, `output_matches`).
    * Command success and exit codes (`last_command succeeded`, `exit_code_is`).
    * File system state (`file_exists`, `file_contains`, `dir_exists`).
    * Test dependencies (`Test has_succeeded ...`).
* **Scenario Cleanup**: Use the `after` block to run cleanup commands, ensuring a clean state between scenario runs.
* **JSON Reporting**: Generates a detailed JSON report compatible with modern test reporting tools.

### **Example Usage**

Here is a simple `test_ls.chor` script that lists files and verifies the output:

```bash
# feature: "Tee Utility"
# This test verifies that tee correctly writes its input to both
# standard output and the specified file.

feature: "Tee Utility"
env: PWD
actors: Terminal, FileSystem

scenario "tee writes to both stdout and a file" {
    test TeeWritesToBothDests "tee writes to both destinations" {
        given:
            # Ensure a clean state before the test.
            FileSystem delete_file "tee_output.txt"
        when:
            # Use an env var to ensure the file is created in the correct place.
            Terminal runs "cd $PWD; echo 'hello tee' | tee tee_output.txt"
        then:
            # Check both the terminal output and the file system.
            Terminal output_contains "hello tee"
            FileSystem file_exists "tee_output.txt"
            FileSystem file_contains "tee_output.txt" "hello tee"
    }

    # This block runs after all tests in the scenario are complete.
    after {
        FileSystem delete_file "tee_output.txt"
    }
}
```

See [test_medi_env_workflow.chor](examples/test_medi_env_workflow.chor) for a more comprehensive example
using [medi](https://github.com/cladam/medi) as the tools to test

### How it works

Choreo is built on a simple but powerful pipeline:

1. **Parse:** A `pest`-based parser reads the `.chor` script and transforms it into an Abstract Syntax Tree (AST).
2. Run: A reactive runner loops through the tests in a scenario. It continuously checks `given` conditions against the
   current system state (terminal output, file system, etc.).
3. **Execute:** Once a test's `given` conditions are met, its `when` actions are dispatched to the appropriate backend (
   e.g.,
   `TerminalBackend`, `FileSystemBackend`).
4. **Assert:** The runner then checks the then conditions to determine if the `test` passed. This loop continues until
   all
   tests in the scenario are complete or a timeout is reached.
5. **Report:** After execution, a detailed JSON report is generated.

### Getting Started

#### Prerequisites

You need [Rust and Cargo](https://www.rust-lang.org/tools/install) installed.

#### Build

```bash
cargo build --release
```

#### Run a script

Use the `run` command to execute a `.chor` file. Use the `--verbose` flag for detailed debug output.

```bash
# Run a test script
cargo run -- run --file "examples/redirecting_output_tee.chor"

# Run with verbose logging
cargo run -- run --file "examples/redirecting_output_tee.chor" --verbose
```

### Status & Roadmap

Choreo is currently in the **alpha stage**. The core engine is functional, but it is not yet ready for production use.

The journey ahead includes:

* [ ] **More Actors**:
    * `WebActor` for making and asserting against HTTP API calls.
* [ ] **Richer Assertions**: Add more conditions, like regex matching without capturing (`output_matches_pattern`) and
  checking command exit codes.
* [ ] **Improved Reporting**: Generate JUnit XML reports for better CI/CD integration.
* [ ] **GIF Generation**: Revisit the vhs inspiration by adding an option to record the terminal session as a GIF.
* [ ] **Publish**: Publish the crate to crates.io.

### **Contributing**

Contributions are welcome! Please feel free to open an issue or submit a pull request.

