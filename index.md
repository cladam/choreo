---
layout: default
title: choreo DSL Documentation
---

# Overview

`choreo` is a test runner and executable Domain-Specific Language (DSL) designed for behavior-driven testing of
command-line applications. It brings the power and expressiveness of a BDD framework like Cucumber to the shell,
allowing you to write automated, human-readable tests for any command-line tool or system interaction.

The tests are written in a structured, Gherkin-inspired format, making them easy to read and maintain. Each `.chor` file
is a self-contained, executable test, eliminating the need for separate "step definition" files.

## Key Features:

* **Human-Readable BDD Syntax:** Utilises a `given-when-then` structure within `test` blocks for clear and descriptive
  tests.
* **Executable Scripts:** `.chor` files are complete, runnable tests without needing separate "step definition" files.
* **Stateful Scenarios:** Capture variables from command output and reuse them in subsequent steps, allowing for
  complex, stateful test scenarios.
* **Multi-Actor System:** Interact with and assert against multiple parts of a system in a single test, including a "
  Terminal" for checking `stdout`, `stderr`, and exit codes, and a "FileSystem" for managing files and directories.
* **Configurable Test Runner:** Provides a settings block to control test behavior such as timeouts and custom shell
  paths.
* **CI-Friendly Reporting:** Generates standard JSON reports for easy integration with CI/CD pipelines.
* **Extensible Architecture:** Designed to allow for future expansion with additional actors and custom commands.
* **Open Source:** Fully open-source and available on [GitHub]("https://github.com/cladam/choreo")
  and [Crates.io](https://crates.io/crates/choreo).

```mermaid
graph TD;
    A[Plain Text (.chor file)] --> B[Parser];
    B -- Pest --> C{Abstract Syntax Tree};
    C --> D[Runner];
    D --> E{Backends};
    subgraph Backends
      F[Web];
      G[Terminal];
      H[FileSystem];
    end
    E --> F;
    E --> G;
    E --> H;
    F --> I([Test Result]);
    G --> I;
    H --> I;
    I -- JSON Report --> I;
  ```
