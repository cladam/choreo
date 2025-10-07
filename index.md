---
layout: default
title: choreo DSL Documentation
---

# What is `choreo`

`choreo` is a test runner and executable Domain-Specific Language (DSL) designed for behavior-driven testing of
command-line applications. It brings the power and expressiveness of a BDD framework like Cucumber to the shell,
allowing you to write automated, human-readable tests for any command-line tool or system interaction.

`choreo` is an executable Domain-Specific Language (DSL) for writing powerful, readable, and maintainable tests for
command-line applications and APIs. It's a modern BDD framework designed for engineers who need to
verify the behaviour of their systems from the outside in.

## The Challenge: Testing from the outside

Testing applications from an external perspective, whether it's a CLI tool, a REST API, or a script that interacts with
the file system is often complex. Traditional tools can lead to brittle shell scripts or require complex "glue code" to
connect plain-text specifications to an executable test runner. This creates friction and slows down development.

## The `choreo` solution: An executable specification

`choreo`'s design is heavily influenced by the original intent of Behaviour-Driven Development (BDD), as described by
its creator. The core goal is to improve communication to get work done, not to get bogged down in layers of
abstraction.

Traditional BDD frameworks often separate the plain-text specification from the implementation, creating a multi-layered
system:

`Plain-Text .feature file` -> `Regex-based Step Definitions (Glue Code)` -> `Actual Application Code`

This indirection is complex and brittle. `choreo` solves this by being an executable specification. A `.chor` file is a
complete, self-contained program that combines the readable BDD steps with the implementation (`Terminal run`, `FileSystem
create_file`, etc.). This creates a direct and simple architectural flow, entirely removing the need for a separate "
glue code" layer.

This approach aligns perfectly with the vision of a "model client"; a way for engineers to describe a system's
behaviour in a readable format that is directly executable.

## Key Features:

* **Readable BDD Syntax:** Uses an explicit `Feature -> Scenario -> Test` hierarchy with `given`, `when`, and `then`
  blocks to tell a clear story.
* **Multi-Actor System:** Natively understands how to interact with different parts of your system, including the
  `Terminal`, the `FileSystem`, and the `Web`.
* **Stateful Scenarios:** Chain tests together with `Test has_succeeded` and capture dynamic values from output into
  variables (`... as myVar`), allowing you to build complex, end-to-end scenarios.
* **Powerful Assertions:** A rich vocabulary of built-in matchers for checking exit codes, `stdout` vs. `stderr`, file
  content, JSON responses, and more, inspired by industry-standard tools like Chai.js and ShellSpec.
* **CI-Friendly Reporting:** Generates standard JSON reports for easy integration with CI/CD pipelines.
* **Extensible Architecture:** Designed to allow for future expansion with additional actors and custom commands.
* **Open Source:** Fully open-source and available at [GitHub]("https://github.com/cladam/choreo")
  and [Crates.io](https://crates.io/crates/choreo).

## ATDD in Practice

In modern software engineering, different types of tests serve different purposes. `choreo` is a specialised tool
designed for the upper levels of the testing pyramid, focusing on Acceptance Test-Driven Development (ATDD).

### Acceptance Tests (Level 3)

`choreo` is perfect for writing acceptance tests for your application's features. A `.chor` file can serve as the
executable version of a user story's acceptance criteria.

#### Example: Testing a CLI tool

```choreo
feature "Note Creation"
actors {
    Terminal
    Web
}

scenario "User can create a new note with content" {
    test NoteIsCreated "it creates the note file on disk" {
        given:
            FileSystem delete_file "my-note.md"
        when:
            Terminal run "my-cli new my-note --content 'Hello'"
        then:
            Terminal last_command succeeded
            FileSystem file_exists "my-note.md"
    }
}
```

### External System Contract Tests (Level 4)

The `Web` actor makes `choreo` a powerful tool for writing contract tests. These tests verify that the external APIs
your application depends on are still working the way you expect.

#### Example: Verifying an API contract

```choreo
feature "httpbin.org API Contract"
actor Web
scenario "Verify the /post endpoint contract" {
    test PostEndpointEchoesJson "it correctly echoes a JSON body" {
        when:
            Web POST "https://httpbin.org/post" with_body '{"id": 123}'
        then:
            Web response_status_is 200
            Web json_path at "/json/id" equals 123
    }
}
```

By providing a single, powerful language for both acceptance and contract testing, `choreo` helps you build confidence
in your entire system, from its internal features to its external dependencies.
