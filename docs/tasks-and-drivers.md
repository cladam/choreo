---
layout: default
title: Tasks and Drivers
---

# Tasks: Separating Intent from Implementation

One of the most important principles in Behaviour-Driven Development is the separation of **business intent** (the
"what") from **implementation details** (the "how"). Traditional BDD frameworks achieve this through "glue code" — a
separate layer that translates human-readable steps into executable code. However, this creates indirection, complexity,
and maintenance burden.

`choreo` solves this differently. With **tasks**, you can create reusable, parameterised "driver" functions directly
within your `.chor` file. Tasks encapsulate the implementation details while your scenarios remain clean, readable, and
focused on business outcomes.

## The Problem: Leaky Abstractions

Consider a typical API health check test without tasks:

```choreo
scenario "API Health Check" {
    test HealthSLA "Verify service responds within SLA" {
        given:
            Web set_header "User-Agent" "choreo-test-runner/1.0"
        when:
            Web set_header "Authorization" "Bearer my-secret-token"
            Web http_get "https://api.myapp.com/health"
        then:
            Web response_status is_success
            Web response_time is_below 2s
    }
}
```

While this works, it exposes implementation details directly in the scenario:

- The specific header format (`Bearer my-secret-token`)
- The exact endpoint URL (`/health`)
- The HTTP method (`http_get`)

A Product Owner reading this test sees *how* we're testing, not *what* we're verifying. If the API changes from REST to
GraphQL, or the authentication mechanism changes, you must update every test that uses it.

## The Solution: The Four-Layer Model

Tasks introduce a clean separation using what we call the **Four-Layer Model**:

```
┌─────────────────────────────────────────────────────────┐
│  Business Specification Layer (Scenarios & Tests)       │
│  "What" - Derived from Acceptance Criteria              │
├─────────────────────────────────────────────────────────┤
│  Implementation Layer (Tasks/Drivers)                   │
│  "How" - Hidden from stakeholders, executable by Choreo │
├─────────────────────────────────────────────────────────┤
│  Actor Layer (Web, Terminal, FileSystem, System)        │
│  Built-in backends that execute commands                │
├─────────────────────────────────────────────────────────┤
│  System Under Test                                      │
│  Your application, API, or CLI tool                     │
└─────────────────────────────────────────────────────────┘
```

Tasks sit between your business-readable scenarios and the low-level actor commands, acting as **drivers** that
translate intent into action.

## Defining Tasks

A task is defined at the top level of your `.chor` file, alongside features, actors, and variables. Tasks have a name,
optional parameters, and a body containing actions and/or conditions.

### Syntax

```choreo
task task_name(param1, param2, ...) {
    # Actions and conditions go here
    Actor action "with ${param1}"
    Actor condition_check
}
```

### Example: Authentication Driver

```choreo
task authenticate_with_token(token, endpoint) {
    Web set_header "Authorization" "Bearer ${token}"
    Web http_get "${endpoint}"
}
```

### Example: SLA Verification Driver

```choreo
task verify_sla_compliance() {
    Web response_status is_success
    Web response_time is_below 2s
}
```

### Example: Content Assertion Driver

```choreo
task verify_response_contains(expected_text) {
    Web response_body_contains "${expected_text}"
}
```

## Calling Tasks

Tasks can be called from any `given`, `when`, or `then` block. The syntax mirrors a function call:

```choreo
task_name(arg1, arg2, ...)
```

Arguments can be:

- **Strings**: `"my-value"` or `"https://api.example.com/endpoint"`
- **Numbers**: `42` or `200`
- **Durations**: `2s`, `500ms`
- **Variable references**: `${MY_VAR}` or just the variable name

## Complete Example: Authentication Service SLA

Here's a full example demonstrating the power of tasks to separate business intent from implementation:

```choreo
feature "Authentication Service SLA"
actors: Web

# ═══════════════════════════════════════════════════════════════════
# IMPLEMENTATION LAYER (Drivers)
# This is the "How" — hidden from stakeholders, executable by Choreo
# ═══════════════════════════════════════════════════════════════════

task check_service_health(token, endpoint) {
    Web set_header "Authorization" "Bearer ${token}"
    Web http_get "${endpoint}"
}

task verify_sla_compliance() {
    Web response_status is_success
    Web response_time is_below 2s
}

task verify_response_contains(expected_text) {
    Web response_body_contains "${expected_text}"
}

# ═══════════════════════════════════════════════════════════════════
# BUSINESS SPECIFICATION LAYER
# This is the "What" — derived directly from Acceptance Criteria
# ═══════════════════════════════════════════════════════════════════

scenario "Authentication Service Reliability" {
    
    # ───────────────────────────────────────────────────────────────
    # User Story: As a Platform Owner, I want the Authentication
    # Service to be highly responsive so that our customers never
    # experience delays during login.
    #
    # AC1: The service must authorize requests using a secure token.
    # AC2: The service must respond with a success status.
    # AC3: The response time must stay under a 2-second SLA.
    # ───────────────────────────────────────────────────────────────
    
    test HealthSLA "Verify service responds within SLA" {
        given:
            Web set_header "User-Agent" "choreo-test-runner/1.0"
        when:
            check_service_health("secure-token-xyz", "https://api.myapp.com/health")
        then:
            verify_sla_compliance()
    }

    test ResponseContent "Verify response contains expected data" {
        given:
            Test has_succeeded HealthSLA
        when:
            Web set_header "User-Agent" "choreo-test-runner/1.0"
            Web http_get "https://api.myapp.com/status"
        then:
            Web response_status is_success
            verify_response_contains("operational")
    }
}
```

## Benefits of Using Tasks

### 1. Readable by Stakeholders

A Product Owner or Business Analyst can read the scenario and understand *what* is being verified without needing to
know the HTTP details:

```choreo
when:
    check_service_health("secure-token", "/health")
then:
    verify_sla_compliance()
```

This reads almost like English: "When we check the service health, then verify SLA compliance."

### 2. Reusable Drivers

If your API migrates from REST to GraphQL, or changes its authentication mechanism, you only update the task definition.
Every test that uses `check_service_health` automatically gets the fix:

```choreo
# Before: REST
task check_service_health(token, endpoint) {
    Web set_header "Authorization" "Bearer ${token}"
    Web http_get "${endpoint}"
}

# After: GraphQL
task check_service_health(token, query) {
    Web set_header "Authorization" "Bearer ${token}"
    Web set_header "Content-Type" "application/json"
    Web http_post "https://api.myapp.com/graphql" "${query}"
}
```

### 3. Direct Mapping to Acceptance Criteria

With tasks, your test names and steps can map 1:1 to your User Stories and Acceptance Criteria. This creates
**traceability** from requirements to executable tests — a core principle of Acceptance Test-Driven Development (ATDD).

### 4. Reduced Duplication

Common patterns like "authenticate and make request" or "verify standard success response" can be extracted into tasks
and reused across dozens of tests, reducing duplication and maintenance burden.

## Task Expansion

When `choreo` runs a test, tasks are **expanded** at runtime. This means:

1. The task call is replaced with its body
2. Parameters are substituted with the provided arguments
3. Actions are executed in order
4. Conditions are evaluated as part of the block they appear in

For example, calling `check_service_health("my-token", "/api/v1/health")` in a `when` block expands to:

```choreo
when:
    Web set_header "Authorization" "Bearer my-token"
    Web http_get "/api/v1/health"
```

## Best Practices

### Name Tasks by Intent, Not Implementation

```choreo
# ✅ Good: Describes what it does
task verify_user_is_authenticated() { ... }

# ❌ Bad: Describes how it does it
task check_200_status_and_token_header() { ... }
```

### Keep Tasks Focused

Each task should do one thing well. If a task is doing too much, split it:

```choreo
# ✅ Good: Single responsibility
task authenticate(token) {
    Web set_header "Authorization" "Bearer ${token}"
}

task fetch_user_profile(user_id) {
    Web http_get "/users/${user_id}"
}

# ❌ Bad: Doing too much
task authenticate_and_fetch_user_and_verify(token, user_id) {
    Web set_header "Authorization" "Bearer ${token}"
    Web http_get "/users/${user_id}"
    Web response_status is_success
    Web json_path at "$.id" equals "${user_id}"
}
```

### Document Your Drivers

Add comments to explain what each task does, especially if the implementation is complex:

```choreo
# Authenticates with the OAuth2 token endpoint and sets the bearer token.
# This task handles the full OAuth2 client credentials flow.
task oauth2_authenticate(client_id, client_secret) {
    Web set_header "Content-Type" "application/x-www-form-urlencoded"
    Web http_post "/oauth/token" "grant_type=client_credentials&client_id=${client_id}&client_secret=${client_secret}"
}
```

### Use Tasks in `then` Blocks for Complex Assertions

Tasks aren't just for actions. Use them to group related assertions:

```choreo
task verify_valid_user_response() {
    Web response_status is_success
    Web json_path at "$.id" is_string
    Web json_path at "$.email" is_string
    Web json_path at "$.created_at" is_string
}

scenario "User API" {
    test FetchUser "Fetch user returns valid structure" {
        given:
            Test can_start
        when:
            Web http_get "/users/123"
        then:
            verify_valid_user_response()
    }
}
```

## Summary

Tasks bring the best of both worlds to `choreo`:

- **Executable specifications** — no separate glue code layer
- **Clean separation of concerns** — business intent vs implementation
- **Reusability** — define once, use everywhere
- **Maintainability** — change implementation in one place

By using tasks effectively, your `.chor` files become living documentation that stakeholders can read and developers can
execute — the true promise of Behaviour-Driven Development.
