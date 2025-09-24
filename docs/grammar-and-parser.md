---
layout: default
title: Grammar & Parser
---

## choreo parser

`src/parser.rs` is responsible for parsing choreo into a structured format. It uses the `pest` parser generator library
to transform the text-based DSL into an **Abstract Syntax Tree (AST)**, defined in `src/ast.rs`.

### High-Level Description

The parser's main goal is to function as a **translator and architect**. It reads a string written in the `choreo`
language and translates it into a **`TestSuite`** struct—a structured, predictable blueprint of the test.

This process serves two critical functions:

1. **Validation**: It checks the script's syntax against the formal grammar (`src/resources/choreo.pest`). If the syntax
   is invalid,
   the parser rejects the input and returns a detailed error, ensuring that only valid commands are ever executed.
2. **Structuring**: It organises the validated script into a hierarchy of Rust `struct`s and `enum`s.
   This allows the test runner to work with a clean, type-safe representation of the test logic instead of raw text.
