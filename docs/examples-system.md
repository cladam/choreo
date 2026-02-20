---
layout: default
title: System Examples
---

# The System actor

The `System` actor provides access to built-in functionalities that don't fall into other categories like `Terminal` or
`Web`. These are useful for debugging, controlling test flow, generating dynamic data, and verifying system state like
services and ports.

## Declaring the Actor

To use system actions, you must declare the `System` actor in your `actors` block.

```choreo
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
    Terminal run "create-user --id ${NEW_USER_ID}"
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

## Conditions

The `System` actor also provides conditions for checking system state. These are useful for verifying that required
services are running or that ports are available before running tests.

### Service Conditions

#### `service_is_running`

Checks if a service is currently running on the system. On macOS, this checks launchd services and running processes.
On Linux, this checks systemd and init.d services.
**Syntax:** `System service_is_running "<service_name>"`

#### `service_is_stopped`

Checks if a service is not currently running.
**Syntax:** `System service_is_stopped "<service_name>"`

#### `service_is_installed`

Checks if a service is installed on the system (regardless of whether it's running).
**Syntax:** `System service_is_installed "<service_name>"`

#### Example:

```choreo
test DatabaseReady "Ensure database is ready before tests" {
    given:
        Test can_start
    when:
        Terminal run "echo 'Checking database...'"
    then:
        Terminal last_command succeeded
        System service_is_running "postgresql"
}
```

### Port Conditions

#### `port_is_listening`

Checks if something is listening on the specified port. This is useful for verifying that a server has started.
**Syntax:** `System port_is_listening <port_number>`

#### `port_is_closed`

Checks if a port is available (nothing is listening on it). This is useful for ensuring a port is free before starting
a service.
**Syntax:** `System port_is_closed <port_number>`

#### Example:

```choreo
test ServerStarted "Verify the server is listening" {
    given:
        Test can_start
    when:
        Terminal run "start-server.sh &"
        System pause 2s
    then:
        System port_is_listening 8080
}

test PortAvailable "Ensure port is free before starting" {
    given:
        Test can_start
    when:
        Terminal run "echo 'Checking port availability'"
    then:
        Terminal last_command succeeded
        System port_is_closed 3000
}
```

## Full Example

This example demonstrates using System conditions to verify environment readiness:

```choreo
feature "System Conditions Demo"

actors: System, Terminal

scenario "Verify system environment" {
    test CheckPorts "Verify required ports are available" {
        given:
            Test can_start
        when:
            Terminal run "echo 'Checking port availability'"
        then:
            Terminal last_command succeeded
            System port_is_closed 8080
            System port_is_closed 5432
    }

    test CheckServices "Verify required services" {
        given:
            Test has_succeeded CheckPorts
        when:
            Terminal run "echo 'Checking services'"
        then:
            Terminal last_command succeeded
            System service_is_installed "docker"
    }
}
```

See the example file
[examples/system_conditions.chor](https://github.com/cladam/choreo/blob/main/examples/system_conditions.chor) for a
runnable demonstration of these conditions.
