---
layout: default
title: Advanced Topics
---

# Advanced Topics

While the core of `choreo` is writing tests, the ecosystem includes powerful tools to help you maintain quality, work
with variables, and integrate `choreo` into your workflow. This section covers those advanced features.

## Validating and Linting Tests

`choreo` comes with a built-in validate command that acts as a comprehensive linter for your `.chor` files. It's a
great tool for catching errors, potential issues, and following best practices before you run your tests, especially
in an automated CI pipeline.

The validator performs three levels of checks:

### Error Checks (E codes)

Critical issues that must be fixed:

- **E001**: Timeout cannot be zero
- **E002**: Invalid HTTP status codes
- **E003**: Invalid HTTP header names (no spaces or special characters)
- **E004**: Invalid JSON in request body when Content-Type is `application/json`

### Warning Checks (W codes)

Potential issues that should be reviewed:

- **W001**: Empty scenarios with no test cases
- **W002**: Tests with no `given` steps (may depend on implicit state)
- **W003**: HTTP URLs missing protocol (http:// or https://)
- **W004**: Excessive wait times (over 5 minutes)
- **W005**: Excessive timeout settings (over 5 minutes)
- **W006**: Unusually high expected failures count
- **W008**: Duplicate scenario names within a feature
- **W009**: Missing cleanup in `after` blocks for file/directory creation
- **W010**: Unused variable definitions
- **W011**: URLs pointing to localhost (may not work in all environments)
- **W012**: Placeholder domains like example.com
- **W013**: Common header typos (e.g., "Acept" instead of "Accept")
- **W014**: Conflicting HTTP headers (e.g., multiple Content-Type headers)
- **W015**: Large request bodies that may cause timeouts
- **W016**: Hardcoded credentials in URLs or headers
- **W017**: Insecure HTTP instead of HTTPS
- **W018**: Missing User-Agent headers for HTTP requests

### Info Checks (I codes)

Informational suggestions:

- **I001**: General best practice suggestions
- **I002**: Notifications when stop_on_failure is enabled

You can run the linter from your terminal like this:

```bash
# Validate a test chor file
choreo lint /path/to/your/test.chor
```

The linter will output detailed diagnostic messages with specific line numbers, error codes, and suggestions to help you
fix issues and improve your test quality. It also tracks variable usage to ensure all defined variables are actually
used in your tests.

## Working with Variables and Environment Secrets

Variables in `choreo` allow you to store and reuse values throughout your tests. This is particularly useful for
sensitive information like API keys or tokens, which you don't want to hardcode in your test files.

#### Local Variables

You can define local variables within your `.chor` file using the `var` keyword. These variables can then be referenced
using `${VARIABLE_NAME}` syntax.

```choreo
feature "Fetch a user profile"
actor Web

var BASE_URL = "https://api.staging.myapp.com"
var USER_ID = "user-123"

scenario "User Profile Tests" {
    test FetchProfile "Fetch user profile" {
        given:
            Test can_start
        when:
            Web http_get "${BASE_URL}/users/${USER_ID}"
        then:
            Web response_status is_success
            Web json_path at "/myapp/user" equals "${USER_ID}"
    }
}
```

#### Environment Variables for Secrets

For sensitive data like API keys or passwords, it's best practice to pass them in from the environment.
`choreo` automatically makes environment variables available for substitution.

```bash
# In your terminal
export AUTH_TOKEN="super-secret-token"
choreo run /path/to/test.chor
```

In your `.chor` file, you can reference this environment variable like so:

```choreo
feature "Fetch a user profile"
actor Web

env AUTH_TOKEN
var BASE_URL = "https://api.staging.myapp.com"
var USER_ID = "user-123"

scenario "User Profile Tests" {
    test FetchProfile "Fetch user profile" {
        given:
            Test can_start
        when:
            Web set_header "Authorization" "Bearer ${AUTH_TOKEN}"
            Web http_get "${BASE_URL}/users/${USER_ID}"
        then:
            Web response_status is_success
            Web json_path at "/myapp/user" equals "${USER_ID}"
    }
}
```

## Editor Integration (Upcoming)

We are actively developing features to improve the `choreo` authoring experience directly in your favorite code editor.

- **Language Server Protocol (LSP)**: A language server that will offer advanced features like auto-completion for
  commands, real-time error checking (linting), and documentation on hover.
- **A dedicated `choreo` editor**: A standalone application with built-in support for writing, validating, and running
  `choreo` tests.
- **Syntax Highlighting**: Plugins for popular editors like VSCode, Sublime Text, and Vim to provide syntax highlighting
  and basic linting.

This will bring a much richer and more productive editing experience to `choreo`. Stay tuned for updates!