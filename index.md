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

## Philosophy and Guiding Principles

`choreo` is designed to be a modern, developer-centric tool for **Acceptance Test-Driven Development (ATDD)**. Its
architecture is heavily influenced by the principles of Continuous Delivery.

### An Executable Specification

The biggest pitfall of traditional BDD frameworks is the separation between the plain-text specification (the `.feature`
file) and the "glue code" that implements it. This creates unnecessary layers of indirection that are complex and
brittle.

`choreo` solves this by being an **executable specification**. A `.chor` file is a complete, self-contained program that
combines the readable BDD steps with the implementation (`Terminal run`, `Web http_get`, etc.). This creates a direct
and simple architectural flow from a single `.chor` file to the application being tested, entirely removing the need for
a separate "glue code" layer.

### Core Principles

`choreo` is built around a set of principles that lead to robust, maintainable, and valuable acceptance tests:

- **Developers Own the Tests:** Acceptance tests are a core part of the development process, not a separate QA activity.
  choreo is designed to be a powerful tool in the hands of developers.
- **Focus on "What," not "How":** Tests should describe the behaviour of your system, not the implementation details of
  its UI. By interacting directly with your application's shell interface or API, `choreo` tests are more stable and
  less coupled to the presentation layer.
- **Test Isolation is Crucial:** Tests must be repeatable and independent. `choreo`'s background and after blocks
  provide a robust mechanism for ensuring that each scenario runs in a clean, known state.
- **Tests Should Appear Synchronous:** A test should read like a simple, sequential story, even if the system under test
  is asynchronous. `choreo`'s reactive engine waits for a "concluding event" (the `then` conditions becoming true)
  rather than relying on fragile `sleep()` commands, which is a key principle for writing reliable tests.

Ready to learn the syntax? Dive into the [Choreo DSL Reference](/choreo-reference).
