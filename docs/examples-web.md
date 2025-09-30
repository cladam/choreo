---
layout: default
title: Web Examples
---

# The Web Actor

The `Web` actor is a powerful component in `choreo` designed for testing HTTP-based services, such as REST APIs. It
allows you to send HTTP requests, manage headers and cookies, and assert conditions on the responses you receive.

## Setup

To use the `Web` actor in your tests, you must first declare it in the actors list at the top of your `.chor` file.

```choreo
actor Web
```

### Example 1: API health check

This example shows a simple health check that sends an authenticated `GET` request to an API endpoint and verifies the
response.

```choreo
feature "API Health Check"

actor Web

settings {
    timeout_seconds = 5
    stop_on_failure = true
}

background {
    Web set_header "User-Agent" "choreo-test-runner/1.0"
    Web set_header "Accept" "application/json"
    Web set_cookie "session_id" "abc123"
}

var URL = "https://httpbin.org/bearer"
var BEARING_TOKEN = "choreo-token-xyz"
# BEARING_TOKEN should come from an environment variable or secret manager in real scenarios
# Example: env BEARING_TOKEN

scenario "Health check for a web API endpoint" {
    test HealthCheck "Verify the API endpoint is healthy" {
        given:
            wait >= 0s
        when:
            Web set_header "Authorization" "Bearer ${BEARING_TOKEN}"
            Web http_get "${URL}"
        then:
            Web response_status is_success
            Web response_time is_below 2s
    }

    after {
        Web clear_header "Authorization"
        Web clear_cookie "session_id"
    }
}
```

### Example 2: Comprehensive web conditions

This example demonstrates a variety of web conditions against the `httpbin.org` service, testing status codes, response
bodies, and JSON structures.

```choreo
feature "Web Conditions"
actor Web

scenario "Testing various web conditions" {
    test StatusCodeTest "Test different HTTP status codes" {
        given:
            wait >= 0s
        when:
            Web http_get "https://httpbin.org/status/200"
        then:
            Web response_status_is 200
            Web response_time is_below 2s
    }

    test SuccessStatusTest "Test response status is success" {
        given:
            wait >= 0s
        when:
            Web http_get "https://httpbin.org/status/201"
        then:
            Web response_status is_success
    }

    test BodyContainsTest "Test response body contains text and has correct JSON structure" {
        given:
            wait >= 0s
        when:
            Web http_get "https://httpbin.org/json"
        then:
            Web response_status_is 200
            Web response_body_contains "slideshow"
            Web json_response at "/slideshow/slides" is_an_array
            Web json_response at "/slideshow/title" is_a_string
            Web json_response at "/slideshow/slides" has_size 2
            Web json_response at "/slideshow" is_an_object
    }

    test JsonValueTest "Test JSON path value equality" {
        given:
            wait >= 0s
        when:
            Web http_get "https://httpbin.org/json"
        then:
            Web response_status_is 200
            Web json_path at "/slideshow/title" equals "Sample Slide Show"
    }
}
```
