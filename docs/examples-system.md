---
layout: default
title: System Examples
---

# The System actor

The `System` actor provides access to built-in functionalities that don't fall into other categories like `Terminal` or
`Web`. These are useful for debugging, controlling test flow, and generating dynamic data.

## Declaring the Actor

To use system actions, you must declare the `System` actor in your `actors` block.

```chor
actors {
    System
    Terminal
}
```

## Actions

### `log`

Prints a message to the console during the test run. This is extremely useful for debugging test logic and inspecting
variable values.
**Syntax:** `System log "<message>"`

#### Example:

```choreo
when:
    System log "Starting the user creation flow."
    System log "The user ID is ${userId}"
```

### `pause`

Pauses the test execution for a specified duration. This can be helpful when waiting for an asynchronous process that
has no other clear signal for completion. Durations can be specified in seconds (s) or milliseconds (ms).
**Syntax:** `System pause <duration>`

#### Example:

```choreo
when:
    Terminal run "start-background-process.sh"
    # Give the process a moment to initialise
    System pause 2s
```

### `uuid`

Generates a version 4 UUID and stores it in a variable. This is perfect for creating unique identifiers for resources,
flow IDs, or any other data that needs to be unique for each test run.
**Syntax:** `System uuid as <variable_name>`

#### Example:

```choreo
when:
    System uuid as NEW_USER_ID
    Terminal run "create-user --id ${NEW_USER_ID"
```

### `timestamp`

Gets the current Unix timestamp (as a string) and stores it in a variable.
**Syntax:** `System timestamp as <variable_name>`

#### Example:

```choreo
when:
    System timestamp as REQUEST_TIME
    Web set_header "X-Request-Time" "${REQUEST_TIME}"
```

This is a very common test case where you run a command and check if its standard output (`stdout`) contains the text
you expect.

```choreo
scenario "Program greets a user" {
    test VerifyPrintf "Verify the program prints a welcome message" {
        given:
            Test can_start
        when:
            # Run a command that should print "Hello, Choreo!"
            Terminal run "my-cli-app greet choreo"
        then:
            # Check that stdout (output) contains the expected greeting
            Terminal output_contains "Hello, choreo!"
            Terminal last_command succeeded
            Terminal stderr_is_empty
    }
}
```

## Full Example

See the example
file [examples/system_actions.chor](https://github.com/cladam/choreo/blob/main/examples/system_actions.chor) for a
runnable demonstration of these actions.