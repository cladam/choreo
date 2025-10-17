<div align="center">

<p align="center">
  <img src="assets/choreo-logo.png" alt="choreo logo" width="200"/>
</p>

<p align="center">
  <b>BDD testing that runs in your shell</b><br/>
</p>


![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/cladam/choreo/rust-ci.yml)
[![Crates.io](https://img.shields.io/crates/v/choreo.svg)](https://crates.io/crates/choreo)
[![Downloads](https://img.shields.io/crates/d/choreo.svg)](https://crates.io/crates/choreo)
[![License](https://img.shields.io/crates/l/choreo.svg)](https://crates.io/crates/choreo)

</div>

## About

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
    - **Terminal**: Control an interactive pseudo-terminal, check `stdout`, `stderr`, and command exit codes.
    - **FileSystem**: Create, delete, and verify files and directories as part of your test setup and assertions.
- **Configurable Test Runner**: Control test behavior with a settings block for features like timeouts and custom shell
  paths.
- **CI-Friendly Reporting**: Generates standard JSON reports for easy integration with CI/CD pipelines.

### Example Usage

<p align="center">
  <img src="assets/chor_example.png" alt="choreo logo" width="800"/>
</p>

See [web_health_check.chor](examples/web_health_check.chor) for the above example in text format. There are many more
examples in the [examples](examples) directory.

### Full Documentation

For a complete guide to all keywords, actions, conditions, and features, please see the
official [Choreo DSL Reference](https://cladam.github.io/choreo/docs/choreo-reference/).

### Getting Started

#### Prerequisites

You need [Rust and Cargo](https://www.rust-lang.org/tools/install) installed.

#### Installing from crates.io

The easiest way to install `choreo` is to download it from [crates.io](https://crates.io/crates/choreo). You can do it
using the following command:

```bash
cargo install choreo
```

If you want to update `choreo` to the latest version, execute the following command:

```bash
choreo update
```

#### Building from source

Alternatively you can build `medi` from source using Cargo:

```bash
git clone https://github.com/cladam/choreo.git
cd choreo
cargo build --release

# The binary will be located at target/release/choreo
# Optionally, you can add it to your PATH
export PATH=$PATH:$(pwd)/target/release

# Or, install it system-wide
cargo install --path .
```

### Command-Line Usage

#### Create a new test file

Use the `init` command to generate a new example `.chor` file. This is a great starting point for a new test suite.

```bash
# Create a new test file with default name "test.chor"
choreo init

# Create a new test file
choreo init --file "my_test.chor"
```

#### Validate a test file

Use the `validate` command to check the syntax and structure of a `.chor` file without executing it.

```bash
# Validate the default test.chor file
choreo validate

# Validate a specific file
choreo validate --file "examples/advanced_matchers.chor"
```

#### Lint a test file

Use the `lint` command to both check syntax and semantics of a `.chor` file without executing it.

```bash
# Validate and run linting on the default test.chor file
choreo lint

# Lint a specific file
choreo lint --file "examples/advanced_matchers.chor"
```

#### Run a script

Use the `run` command to execute a `.chor` file. Use the `--verbose` flag for detailed debug output.

```bash
# Run a test script
choreo run --file "examples/redirecting_output_tee.chor"

# Run with verbose logging
choreo run --file "examples/redirecting_output_tee.chor" --verbose
```

### Status & Roadmap

`choreo` is currently in the **beta stage**. The core engine is functional, but more to add before production-ready.

The journey ahead includes:

* [ ] A dedicated editor with syntax highlighting and linting.

### **Contributing**

Contributions are welcome! Please feel free to open an issue or submit a pull request.

