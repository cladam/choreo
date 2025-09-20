## WebBackend

The `WebBackend` class is a specialised backend for handling web requests and responses. It extends the functionality of
a generic backend to cater specifically to web applications.

### High-Level Description

The `WebBackend` class is designed to manage web interactions, including sending HTTP requests and processing responses.
It provides methods for common web operations, such as GET and POST requests, and handles various aspects of web
communication, such as headers, cookies, and sessions.

### Key Features

- **HTTP Methods**: Supports standard HTTP methods like GET, POST, PUT, DELETE, etc.
- **Session Management**: Maintains session state across multiple requests.
- **Header and Cookie Handling**: Allows setting and retrieving HTTP headers and cookies.
- **Error Handling**: Provides mechanisms to handle HTTP errors and exceptions gracefully.