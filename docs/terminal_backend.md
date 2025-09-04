## TerminalBackend

`src/terminal_backend.rs` provides the core functionality for interacting with a terminal process. It is responsible for spawning a shell, sending commands to it, and reading its output. This allows the application to programmatically control a terminal session to execute and verify the steps defined in `choreo` test scripts.

### High-Level Description

The `TerminalBackend` struct encapsulates a **pseudo-terminal (PTY)** session, acting as the bridge between the abstract test `Action`s in the AST and concrete operations in a real terminal.

Its primary challenge is solving the "blocking I/O" problem: reading from a terminal will freeze an application until there is new output. The `TerminalBackend` solves this by employing a **multi-threaded design**. Think of it like a newsroom:

* The **main thread** is the **news anchor**, running the test and deciding what needs to happen next.
* A dedicated **reader thread** acts as a **field reporter**, who is sent out to watch the terminal. The reporter waits patiently for any news (output) and immediately sends it back to the newsroom via a **channel** (a secure message queue).

This ensures the news anchor (the main thread) is never stuck waiting and can remain responsive, checking for new messages from the reporter whenever it needs to.


### Detailed Functionality

#### **1. `TerminalBackend` Struct**
* **`writer: Box<dyn Write + Send>`**: The input stream for the PTY. Writing bytes to this `writer` is equivalent to a user typing in the terminal.
* **`output_receiver: Receiver<String>`**: The receiving end of an **mpsc channel**. The dedicated background thread sends all PTY output through this channel.
* **`child: Box<dyn portable_pty::Child + Send + Sync>`**: A handle to the spawned shell process, used to manage its lifecycle and ensure clean termination.
* **`reader_thread: Option<JoinHandle<()>>`**: A handle to the background reader thread, allowing for proper management and shutdown.

#### **2. `new()` Function**
* This constructor sets up the entire PTY environment.
* It uses the `portable-pty` crate to create a new PTY pair (`master` and `slave`).
* It spawns a `bash` command attached to the `slave` end, which is the process the tests will interact with.
* It gets a `reader` and `writer` from the `master` end. The `writer` is stored for sending input, while the `reader` is moved into the newly spawned `reader_thread`.
* The `reader_thread`'s sole job is to read bytes from the PTY, convert them to a string, and send them over the channel. This asynchronous design is critical for the **non-blocking** model.

#### **3. `read_output()` Function**
* Allows the main thread to consume any output produced since the last check.
* It uses **`try_iter()`** on the `output_receiver` to immediately drain all pending messages from the channel without blocking.
* It appends the new output to a buffer supplied by the caller, which is then used to check for expected text (e.g., for an `output_contains` condition).

#### **4. `execute_action()` Function**
* The primary method for interacting with the terminal. It takes an `ast::Action` and translates it into a PTY operation.
* For an `Action::Type`, it writes the specified string to the PTY `writer`.
* For an `Action::Press` ("Enter"), it writes a newline character (`\n`) to simulate executing a command.

#### **5. `impl Drop`**
* This ensures the spawned shell process is terminated when the `TerminalBackend` instance goes out of scope.
* It calls `self.child.kill()`, which forcefully stops the child process, preventing orphaned shell processes after the program exits.