---
layout: default
title: choreo DSL reference
---

# Choreo DSL Reference

**choreo** is an executable Domain-Specific Language (DSL) for writing "model clients" for your command-line
applications and APIs. It uses a structured, human-readable format to define test scenarios that are easy to write,
read, and maintain.

This document serves as the official reference guide for the `.chor` file format and its syntax.

## File Structure: The BDD Hierarchy

A `.chor` file is structured hierarchically to tell a clear story, following the standard BDD pattern of
`Feature -> Scenario -> Test`.

```choreo
feature "A high-level description of the capability being tested"

# ... (settings, vars, actors)

scenario "A concrete example of the feature's behaviour" {

    test TestNameOne "The first step in the scenario" {  
        given:  
            # Pre-conditions and setup actions  
        when:  
            # The single action being tested  
        then:  
            # The expected outcomes and assertions  
    }

    test TestNameTwo "The second step, depending on the first" {  
        given:  
            Test has_succeeded TestNameOne  
        # ... and so on  
    }

    after {  
        # Cleanup actions that run after all tests in this scenario  
    }  
}
```

## Keywords

`choreo` uses a set of keywords to define the structure and logic of a test suite.

#### `feature`

Provides a high-level description of the software capability being tested and groups related scenarios. A `.chor` file
should contain exactly one `feature`.

**Example:**

```choreo
feature "User account management via the CLI"
```

#### `settings`

A block for configuring the behavior of the choreo test runner for the current file.

| Setting Key         | Value Type | Default    | Purpose                                                                                          |
|:--------------------|:-----------|:-----------|:-------------------------------------------------------------------------------------------------|
| `timeout_seconds`   | Number     | 30         | The maximum time in seconds a scenario can run before failing.                                   |  
| `stop_on_failure`   | Boolean    | false      | If true, the entire test suite will stop immediately after the first test fails.                 |  
| `shell_path`        | String     | "zsh"      | The absolute path to the shell to use for the Terminal actor.                                    |  
| `report_path`       | String     | "reports/" | The directory where the report file will be saved.                                               |  
| `expected_failures` | Number     | 0          | Declares the number of tests that are expected to fail for the suite to be considered a success. |

**Example:**

```choreo
settings {
  timeout_seconds = 10  
  stop_on_failure = true  
  expected_failures = 1
}
```

#### `background`

A block that provides a common set of readable `given` steps that apply to all scenarios in a feature.

**Example:**

```choreo
background {
  FileSystem create_dir "temp_data"
  FileSystem create_file "temp_data/initial.txt" with_content "setup data"
}
```

#### `var`

A keyword for defining key-value variables that can be used throughout the test file. This is useful for making tests
more readable and maintainable by avoiding "magic strings."

**Example:**

```choreo
var FILENAME = "my_output.txt"  
var GREETING = "Hello, Choreo!"
```

#### `env`

Declares a list of environment variables that the test suite requires. The test runner will read these from the shell
environment where `choreo` is executed and make them available for substitution.

**Example:**

```choreo
env API_TOKEN, GITHUB_USER
```

#### `actors`

Declares the different systems or components that the test will interact with. The three currently supported actors are
`Terminal`, `FileSystem` and `Web`. You must declare at least one actor per file.

**Example:**

```choreo
actors {
    Terminal
    FileSystem
    Web
}
```

Or use a singular actor like `Web`

```choreo
actor Web
```

#### `scenario`

Describes a single, concrete example of the feature's behaviour. It acts as a container for a sequence of related `test`
blocks that form a user story or workflow.

**Example:**

```choreo
scenario "A user can successfully create and then delete a file" {  
    # ... test blocks go here ...  
}
```

#### `test`

The core unit of testing in `choreo`. Each `test` block has a unique name (for dependencies) and a human-readable
description. It is composed of `given`, `when`, and `then` blocks.

**Example:**

```choreo
test FileIsCreated "it creates a new file with content" {  
    given: # ...  
    when:  # ...  
    then:  # ...  
}
```

#### `after`

An optional block inside a `scenario` that contains a list of cleanup actions. These actions are executed after all
`test` blocks within that scenario have completed, regardless of whether they passed or failed.

**Example:**

```choreo
scenario "..." {  
    # ... tests ...

    after {  
        FileSystem delete_file "${FILENAME}"  
    }  
}
```

## Test Blocks: Given, When, Then

Each `test` block is structured using the standard BDD keywords to create a clear narrative.

#### `given`:

The `given` block sets up the context for a test. It can contain a mix of **actions** (to set up the environment) and
**conditions** (to check pre-requisites, including dependencies on other tests).

**Example:**

```choreo
given:  
    # Action: Ensure a clean state  
    FileSystem delete_file "data.txt"  
    # Condition: This test can only run after the setup test has passed  
    test has_succeeded InitialSetup
```

#### `when`:

The `when` block contains the single, specific action that is being tested. A `when` block should contain only actions,
not conditions.

**Example:**

```choreo
when:  
    Terminal runs "data-processor --input data.txt"
```

#### `then`:

The `then` block contains the assertions that verify the outcome of the `when` action. A `then` block should contain
only conditions. The test passes if all `then` conditions are met.

**Example:**

```choreo
then:  
    Terminal last_command succeeded  
    FileSystem file_exists "output.txt"
```

## Vocabulary: Actions & Conditions

This is the reference for all available commands that can be used within the `test` blocks.

### Wait Conditions

| Syntax          | Description                                                            |
|:----------------|:-----------------------------------------------------------------------|
| `wait >= 1.5s`  | Passes if the test has been running for at least 1.5 seconds.          |
| `wait <= 100ms` | Passes if the test has been running for no more than 100 milliseconds. |

### State Conditions

| Syntax                          | Description                                                                                                         |
|:--------------------------------|:--------------------------------------------------------------------------------------------------------------------|
| `test has_succeeded <TestName>` | Passes if the test with the given name has already passed. This is the primary mechanism for creating dependencies. |

### Terminal Commands

#### Actions

| Syntax                     | Description                                                                             |
|:---------------------------|:----------------------------------------------------------------------------------------|
| `Terminal runs "..."`      | Executes a shell command non-interactively. The command and a newline are sent at once. |
| `Terminal types "..."`     | Simulates a user typing a string into the terminal.                                     |
| `Terminal presses "Enter"` | Simulates a user pressing the Enter key.                                                |

#### Conditions

| Syntax                                     | Description                                                                        |
|:-------------------------------------------|:-----------------------------------------------------------------------------------|
| `Terminal last_command succeeded`          | Passes if the last `Terminal runs` command exited with code 0.                     |
| `Terminal last_command failed`             | Passes if the last `Terminal runs` command exited with a non-zero code.            |
| `Terminal last_command exit_code_is <num>` | Passes if the last `Terminal runs` command exited with the specified code.         |
| `Terminal output_contains "..."`           | Passes if the combined stdout/stderr stream from the PTY contains the substring.   |
| `Terminal stdout_is_empty`                 | Passes if the stdout from the last `Terminal runs` command was empty.              |
| `Terminal stderr_is_empty`                 | Passes if the stderr from the last `Terminal runs` command was empty.              |
| `Terminal stderr_contains "..."`           | Passes if the stderr from the last `Terminal runs` command contains the substring. |
| `Terminal output_starts_with "..."`        | Passes if the trimmed stdout of the last `runs` command starts with the string.    |
| `Terminal output_ends_with "..."`          | Passes if the trimmed stdout of the last `runs` command ends with the string.      |
| `Terminal output_equals "..."`             | Passes if the trimmed stdout of the last `runs` command is an exact match.         |
| `Terminal output_matches "..."`            | Passes if the combined stdout/stderr stream from the PTY matches the regex.        |
| `Terminal output_is_valid_json`            | Passes if the combined stdout/stderr stream from the PTY is valid JSON.            |
| `Terminal json_output has_path "..."`      | Passes if the JSON output has the specified JSON path.                             |

### FileSystem Commands

#### Actions

| Syntax                                            | Description                                                      |
|:--------------------------------------------------|:-----------------------------------------------------------------|
| `FileSystem create_dir "..."`                     | Creates a directory, including any necessary parent directories. |
| `FileSystem create_file "..."`                    | Creates an empty file.                                           |
| `FileSystem create_file "..." with_content "..."` | Creates a file and writes the specified content to it.           |
| `FileSystem delete_dir "..."`                     | Deletes a directory and all its contents.                        |
| `FileSystem delete_file "..."`                    | Deletes a file.                                                  |

#### Conditions

| Syntax                                 | Description                                                      |
|:---------------------------------------|:-----------------------------------------------------------------|
| `FileSystem dir_exists "..."`          | Passes if a directory exists at the specified path.              |
| `FileSystem dir_does_not_exist "..."`  | Passes if no directory exists at the specified path.             |
| `FileSystem file_exists "..."`         | Passes if a file exists at the specified path.                   |
| `FileSystem file_does_not_exist "..."` | Passes if nothing exists at the specified path.                  |
| `FileSystem file_contains "..." "..."` | Passes if the file at the first path contains the second string. |
| `FileSystem file "..." is_empty`       | Passes if the file at the specified path is empty.               |
| `FileSystem file "..." is_not_empty`   | Passes if the file at the specified path is not empty.           |

### Web Commands

#### Actions

| Syntax                                   | Description                                                                |
|:-----------------------------------------|:---------------------------------------------------------------------------|
| `Web http_get "..."`                     | Sends a GET request to the specified URL.                                  |
| `Web set_header "<key>" "<value>"`       | Sets a custom HTTP header (e.g., `Authorization`) for subsequent requests. |
| `Web clear_header "..."`                 | Clears a custom header previously set with `Web set_header`.               |
| `Web set_cookie "<key>" "<value>"`       | Sets a cookie for subsequent requests.                                     |
| `Web clear_cookie "..."`                 | Clears a cookie previously set with `Web set_cookie`.                      |
| `Web http_get "<url>"`                   | Sends a GET request to the specified URL.                                  |
| `Web http_post "<url>" with_body "..."`  | Sends a POST request with the given body.                                  |
| `Web http_put "<url>" with_body "..."`   | Sends a PUT request with the given body.                                   |
| `Web http_patch "<url>" with_body "..."` | Sends a PATCH request with the given body.                                 |
| `Web http_delete "<url>"`                | Sends a DELETE request to the specified URL.                               |

#### Conditions

| Syntax                                         | Description                                                                          |
|:-----------------------------------------------|:-------------------------------------------------------------------------------------|
| `Web response_status_is <num>`                 | Passes if the last HTTP response had the specified status code.                      |
| `Web response_status is_success`               | Passes if the last HTTP response status code is in the 200-299 range.                |
| `Web response_status is_error`                 | Passes if the last HTTP response status code is in the 400-599 range.                |
| `Web response_status is_in [num, num, num]`    | Passes if the last HTTP response status code is in the specified list.               |
| `Web response_time is_below 1s/200ms`          | Passes if the last HTTP response was received in under the specified time.           |
| `Web response_body_contains "..."`             | Passes if the last HTTP response body contains the specified substring.              |
| `Web response_body_matches "..." [as JSON]`    | Passes if the last HTTP response body matches the specified regex.                   |
| `Web json_body has_path "..."`                 | Passes if the last HTTP response body (as JSON) has the specified JSON path.         |
| `Web json_path at "..." equals <value>`        | Passes if the value at the specified JSON path equals the given value.               |
| `Web json_response at "..." is_a_string`       | Passes if the value at the specified JSON path is a string.                          |
| `Web json_response at "..." is_a_number`       | Passes if the value at the specified JSON path is a number.                          |
| `Web json_response at "..." is_an_array`       | Passes if the value at the specified JSON path is an array.                          |
| `Web json_response at "..." is_an_object`      | Passes if the value at the specified JSON path is an object.                         |
| `Web json_response at "..." has_size <number>` | Passes if the value at the specified JSON path (array or string) has the given size. |

## Variables

`choreo` supports both environment variables and file-defined variables for making tests dynamic.

### Substitution

To use a variable, use the `${VAR_NAME}` syntax inside any string literal. The test runner will replace this placeholder
with the variable's value before executing the step.

**Example:**

```choreo
var FILENAME = "output.log"  
when:  
  Terminal runs "echo 'hello' > ${FILENAME}"  
```
