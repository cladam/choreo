---
layout: default
title: Core Concepts
---

# Core Concepts

The `choreo` DSL is designed to be readable and structured.
Every test file follows a consistent hierarchy of concepts that describe a feature, its actors, and specific test
scenarios. This page breaks down those fundamental building blocks.

## The Anatomy of a `.chor` File

Think of a `.chor` file as a script for a play. It has a main theme (**feature**) and a cast of characters (**actors**).
The play is broken into scenes (**scenarios**), and each scene contains one or more specific sequences of action (*
*tests**).
Finally, the `given`, `when`, `then` **steps** act as the stage directions and dialogue that describe the plot of each
test.

```choreo
feature "User Authentication"
actor Terminal

scenario "Successful login attempt" {
    test VerifySuccessfulLogin "Verify login grants access" {
        given:
            # Assume a 'login' command exists
            Terminal run "login --user admin --pass secret123"
        when:
            # Check the output for a success message
            Terminal output_contains "Welcome, admin!"
        then:
            # The command should exit cleanly (0)
            Terminal last_command succeeded
    }
}
```

## 1. Features: The "What"

Every `.chor` file starts by declaring a `feature`. A `feature` is a high-level description of a single piece of your
application's functionality, like "User Authentication" or "File Uploads." It acts as a container for all related test
scenarios.

```choreo
feature "A short, descriptive name for the functionality"
```

## 2. Actors: The "Who"

`actors` are the components or systems that will perform actions in your tests. Choreo has three built-in actors that
act as specialised backends:

- **`Web`**: For making HTTP requests and testing APIs.
- **`Terminal`**: For running shell commands and inspecting their output.
- **`FileSystem`**: For creating, reading, and verifying files and directories.

You must declare which actors your feature will use at the top of the file.

```choreo
actors {
    Terminal
    FileSystem
}
```

## 3. Scenarios & Tests: The "How"

A `scenario` is a single, concrete example of a `feature`'s behaviour. A `feature` can have multiple scenarios. For
example, a "User Authentication" `feature` might have scenarios for both a successful login and a failed login.

Inside a scenario, you define one or more test blocks. Each test block is an individual, runnable test case with a clear
success condition.

```choreo
scenario "Handling invalid credentials" {
  test VerifyDeny "Verify login is denied" {
    ...
  }
}
```

## 4. Steps: The Action (Given, When, Then)

Each `test` is made up of steps that follow the popular **Given-When-Then** BDD structure. This format helps describe
the context, the action, and the expected outcome in a clear, logical flow.

- `given`: Sets up the initial state or preconditions. ("Given I have a user in the database...")
- `when`: Describes the key action that is being tested. ("When I send a POST request to the login endpoint...")
- `then`: Contains the assertions that verify the outcome. ("Then the API should respond with a 200 OK status...")

## How choreo Works

When you run a Choreo test, an internal **parser** reads your human-readable `.chor` file and translates each step into
a command.
These commands are then executed by the declared **actors**, which act as the bridge between your test and the system.
For a deeper look at the parser and backend implementations, see
our [Architecture documentation](../architecture-overview).

## Summary

By structuring your `.chor` files with `feature`, `actors`, `scenario`, and `test` blocks, you create clear,
readable, and maintainable tests that describe both the behavior of your application and the intent behind each test
case.
This organisation helps you and your team understand what is being tested and why, making it easier to maintain and
extend your test suite over time.
