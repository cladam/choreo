---
layout: default
title: Terminal Examples
---

# The Terminal Actor

The `Terminal` actor is the core of `choreo` for testing command-line applications. It allows you to run commands, send
input, and make powerful assertions against `stdout`, `stderr`, and the exit code.

### Example 1: Basic command and exitcode check

This example shows the simplest terminal test: running a command and verifying that it completes successfully. An exit
code of `0` is the standard indicator for success in shell commands.

```choreo
feature "Basic Commands"

actor Terminal

scenario "Listing files in a directory" {
    test VerifyListing "Verify 'ls' command runs successfully" {
        given:
            # Execute immediately
            Test can_start
        when:
            # Run the standard 'ls -l' command
            Terminal run "ls -l"
        then:
            # Assert that the command exited without errors
            Terminal last_command succeeded
        }
}
```

### Example 2: Checking `stdout` for specific output

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