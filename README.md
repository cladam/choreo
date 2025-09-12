# **choreo**

`choreo` is an executable Domain-Specific Language (DSL) for writing automated, behaviour-driven tests for command-line
applications and system interactions. It uses a structured, human-readable format inspired by Gherkin to define test
scenarios that are easy to write, read, and maintain.

The goal of choreo is to provide the power and expressiveness of a BDD framework like Cucumber, but in a self-contained,
executable format specifically designed for testing the shell.

### **Key Features**

- **Human-Readable BDD Syntax**: Uses a `given-when-then` structure within `test` blocks to create clear,
  self-documenting
  tests.
- **Executable Scripts**: `.chor` files are complete, runnable tests. No separate "step definition" or "glue code" files
  are required.
- **Stateful Scenarios**: Capture variables from command output and reuse them in subsequent steps to test complex
  workflows.
- **Multi-Actor System**: Interact with and assert against multiple parts of your system in a single test.
    - **Terminal**: Control an interactive pseudo-terminal, check `stdout`, `stderr, and command exit codes.
    - **FileSystem**: Create, delete, and verify files and directories as part of your test setup and assertions.
- **Configurable Test Runner**: Control test behavior with a settings block for features like timeouts and custom shell
  paths.
- **CI-Friendly Reporting**: Generates standard JSON reports for easy integration with CI/CD pipelines.

### Example Usage

Here is a simple `test_ls.chor` script that tests that `tee` correctly writes its input to both standard output and a
file.

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

### Full Documentation

For a complete guide to all keywords, actions, conditions, and features, please see the
official [Choreo DSL Reference](docs/REFERENCE.md).

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

* [ ] A `background` keyword for shared setup steps across scenarios.
* [ ] A `default_actor` setting to reduce verbosity in tests.
* [ ] **More Actors**
    * `WebActor` for making and asserting against HTTP API calls.
* [ ] Generate JUnit XML reports.
* [ ] Revisit the vhs inspiration by adding an option to record the terminal session as a GIF.
* [ ] Publish the crate to crates.io.
* [ ] An even richer vocabulary of built-in matchers and assertions.

### **Contributing**

Contributions are welcome! Please feel free to open an issue or submit a pull request.

