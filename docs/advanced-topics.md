---
layout: default
title: Advanced Topics
---

# Advanced Topics

While the core of `choreo` is writing tests, the ecosystem includes powerful tools to help you maintain quality, work
with
variables, and integrate `choreo` into your workflow. This section covers those advanced features.

## Validating and Linting Tests

`choreo` comes with a built-in validate command that acts as a linter for your `.chor` files. It's an essential tool for
catching errors before you run your tests, especially in an automated CI pipeline.

The validator checks for several common issues:

- **Syntax Errors**: Ensures your file conforms to the `choreo` grammar.
- **Invalid Commands**: Verifies that the commands you're using are valid for the declared actors (e.g. you can't use
  `http_get` with the `Terminal` actor).
- **Undeclared Variables**: Checks that any `${variable}` you use has been defined in a `var` block or is expected to be
  passed in
  from the environment.

You can run it from your terminal like this:

```bash
# Validate the default test.chor file
choreo validate /path/to/your/test.chor
```

If there are any issues, the validator will print detailed error messages to help you fix them.

## Working with Variables and Environment Secrets

Variables in `choreo` allow you to store and reuse values throughout your tests. This is particularly useful for
sensitive information like API keys or tokens, which you don't want to hardcode in your test files.

#### Local Variables

You can define local variables within your `.chor` file using the `var` keyword. These variables can then be referenced
using `${variable_name}` syntax.

```choreo
var base_url = "https://api.staging.myapp.com"
var user_id = "user-123"
test "Fetch user profile" {
    given:
        wait >= 0s
    when:
        Web http_get "base_url/users/{user_id}"
    then:
        Web response_status is_success  
        Web json_path at "/myapp/user" equals "${user_id}"
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
env: AUTH_TOKEN
test "Authenticated request" {
    given:
        wait >= 0s
    when:
        Web set_header "Authorization" "Bearer ${AUTH_TOKEN}"
        Web http_get "https://api.myapp.com/protected"
    then:
        Web response_status is_success
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