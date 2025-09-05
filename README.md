# **choreo**

**A declarative DSL for choreographing and testing command-line workflows.**

Choreo is a test runner and automation tool that uses a simple, readable, rule-based language to define and execute complex interactions with command-line applications. It's designed for behavior-driven testing, integration testing, and generating reproducible demos of CLI tools.

Think of it as **Cucumber for the command line**, with the reactive power of a rules engine.

### **Key Features**

* **Declarative DSL**: Write tests in a simple, human-readable .chor format.
* **Reactive Rules Engine**: Define rules that fire based on time, application output, or the success of previous steps.
* **Dynamic State**: Capture values from terminal output (like IDs or filenames) and reuse them in subsequent commands.
* **Environment Integration**: Pull in configuration and secrets from environment variables to make your tests portable.
* **Extensible**: Built with a pluggable "Actor" model in mind, starting with the Terminal actor.

### **Example Usage**

Here is a simple `test_ls.chor` script that lists files and verifies the output:
```
# test_ls.chor

actors: Terminal  
outcomes: FilesListed

rule "List files in the current directory" {  
    when:  
        time >= 1s  
    then:  
        Terminal types "ls -l"  
        Terminal presses "Enter"  
}

rule "Verify that README.md is in the output" {  
    when:  
        Terminal output_contains "README.md"  
    then:  
        Test succeeds FilesListed  
}
```

### **How It Works**

Choreo is built on a simple yet powerful pipeline:

1. **Parser**: A `pest`-based parser reads your `.chor` script and validates its syntax.
2. **AST**: The script is transformed into a structured in-memory representation called an Abstract Syntax Tree.
3. **Runner**: A simulation loop ticks forward, checking the conditions of all rules against the current state.
4. **Backend**: When a rule's actions are executed, they are sent to an "Actor" backend. The `TerminalBackend` spawns a 
   pseudo-terminal (PTY) and interacts with it programmatically, just like a real user.

### **Getting Started**

#### **Prerequisites**

* Rust toolchain (stable)

#### **Build**

```bash
cargo build \--release
```

#### **Run a Script**

Currently, the path to the script is hardcoded in `src/main.rs`. In the future, this will be handled by a command-line argument.
```bash
# (After updating main.rs with the path to your .chor file)  
cargo run
```

### **Project Status & Roadmap**

Choreo is currently in the **alpha stage**. The core engine is functional, but it is not yet ready for production use.

The journey ahead includes:

* [ ] **CLI Arguments**: Implement proper command-line argument parsing with clap to specify the test file and other options.
* [ ] **More Actors**:
    * WebActor for making and asserting against HTTP API calls.
    * FileSystemActor for checking file existence, content, etc.
* [ ] **Richer Assertions**: Add more conditions, like regex matching without capturing (output\_matches\_pattern) and checking command exit codes.
* [ ] **Test Reporting**: Generate JUnit XML reports for better CI/CD integration.
* [ ] **GIF Generation**: Revisit the vhs inspiration by adding an option to record the terminal session as a GIF.
* [ ] **Publish**: Publish the crate to crates.io.

### **Contributing**

Contributions are welcome! Please feel free to open an issue or submit a pull request.

