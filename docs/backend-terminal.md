---
layout: default
title: Terminal Backend
---

## TerminalBackend

`src/terminal_backend.rs` provides the core functionality for interacting with a terminal process. It is responsible for
spawning a shell, sending commands to it, and reading its output. This allows the application to programmatically
control a terminal session to execute and verify the steps defined in `choreo` test scripts.

### High-Level Description

The `TerminalBackend` struct encapsulates a **pseudo-terminal (PTY)** session, acting as the bridge between the abstract
test `Action`s in the AST and concrete operations in a real terminal.

Its primary challenge is solving the "blocking I/O" problem: reading from a terminal will freeze an application until
there is new output. The `TerminalBackend` solves this by employing a **multi-threaded design**. Think of it like a
newsroom:

* The **main thread** is the **news anchor**, running the test and deciding what needs to happen next.
* A dedicated **reader thread** acts as a **field reporter**, who is sent out to watch the terminal. The reporter waits
  patiently for any news (output) and immediately sends it back to the newsroom via a **channel** (a secure message
  queue).

This ensures the news anchor (the main thread) is never stuck waiting and can remain responsive, checking for new
messages from the reporter whenever it needs to.
