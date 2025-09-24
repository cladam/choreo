---
layout: default
title: Grammar & Parser
---

# Grammar & Parser

The heart of the choreo DSL is its parser, which is responsible for taking the human-readable text from a `.chor` file
and transforming it into a structured, executable format. This process is powered by `pest`, a powerful parser generator
for Rust.

## The Grammar (`choreo.pest`)

The entire syntax of the choreo language is formally defined in a single grammar file written in `pest`'s parsing
expression grammar (PEG) syntax. This file, `choreo.pest`, acts as a definitive blueprint for the language, defining
every
valid token, from keywords like feature to the structure of a test block.

Using a formal grammar file ensures that the parsing is consistent, robust, and easy to maintain. See the full grammar
[here](../src/resources/choreo.pest).

## The Parser Logic

The `pest` tool uses the `choreo.pest` grammar to generate a Rust module that can parse choreo syntax. The core of the
choreo test runner uses this generated module to perform two key steps:

- **Generate an Abstract Syntax Tree (AST)**: The parser first reads a .chor file and builds a tree-like structure that
  represents the code's hierarchy—features, scenarios, steps, and their relationships.

- **Walk the AST**: The runner then "walks" this tree, interpreting each node and converting it into a series of
  commands and assertions. For example, it identifies the declared actors and ensures that the steps in a scenario are
  valid for those actors.

This two-step process separates the understanding of the language from the execution, making the system clean and
modular.

## From Text to Execution

Once the parser has successfully transformed the `.chor` file into a structured set of instructions, this in-memory
representation is passed to the declared backends (like `Web` or `Terminal`). The backends are then responsible for
executing these instructions and asserting that the outcomes match the conditions defined in your test.

This architecture makes sure that the logic for understanding the `choreo` language is completely decoupled from the
logic of interacting with the system under test.

