---
layout: default
title: FileSystem Examples
---

# The FileSystem Actor

The `FileSystem` actor is your tool for interacting with the file system during a test. It allows you to create, modify,
and delete files and directories, and then make assertions about their state. This is essential for testing file
generation, log output, or any process that manipulates files.

## Setup

To use the `FileSystem` actor in your tests, you must first declare it in the actors list at the top of your `.chor`
file.

```choreo
actor FileSystem
```

### Example 1: Creating a file and verifying its existence

This example shows a common scenario where a program is expected to generate a file. The test first ensures the file
doesn't exist, runs the program, and then verifies that the file has been created.

```choreo
feature "File Creation"
actors {
    FileSystem
    Terminal
}

scenario "Program generates a log file" {
    test VerifyLogfile "Verify log.txt is created after running the program" {
        given:
            # Ensure the file does not exist before the test
            FileSystem delete_file "log.txt"
        when:
            # Assume 'my-program --log' creates a log.txt file
            Terminal runs "my-program --log"
        then:
            # Verify that the file now exists
            FileSystem file_exists "log.txt"
    }
}
```

### Example 2: Writing to a file and asserting its content

This test demonstrates how to check the contents of a file. The test creates a configuration file with specific content,
runs an application that reads it, and then asserts that both the application's output and the file's content are
correct.

```choreo
feature "Configuration File Handling"

actors {
    FileSystem
    Terminal
}

scenario "Program reads a custom configuration" {
    test CommandSuccessAndFileChecks "Handles successful commands and file conditions" {
        given:
            # Create a config file with specific content
            FileSystem create_file "config.toml" with_content "verbose = true"
        when:
            # Run the program which should read config.toml
            Terminal runs "my-app --config config.toml"
        then:
            # The output should reflect the setting from the file
            Terminal output_contains "Running in verbose mode"
            FileSystem file_exists "config.toml"
            FileSystem file_contains "config.toml" with_content "verbose = true"
    }

    after {
        FileSystem delete_file "config.toml"
    }
}
```
