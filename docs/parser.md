## choreo parser

`src/parser.rs` is responsible for parsing choreo into a structured format. It uses the `pest` parser generator library to 
transform the text-based DSL into an **Abstract Syntax Tree (AST)**, defined in `src/ast.rs`.

### High-Level Description

The parser's main goal is to function as a **translator and architect**. It reads a string written in the `choreo` language 
and translates it into a **`TestSuite`** struct—a structured, predictable blueprint of the test.

This process serves two critical functions:
1.  **Validation**: It checks the script's syntax against the formal grammar (`src/resources/choreo.pest`). If the syntax is invalid, 
    the parser rejects the input and returns a detailed error, ensuring that only valid commands are ever executed.
2.  **Structuring**: It organises the validated script into a hierarchy of Rust `struct`s and `enum`s. 
    This allows the test runner to work with a clean, type-safe representation of the test logic instead of raw text.


### Detailed Functionality

1.  **`ChoreoParser` Struct**:
    -  This struct is derived using `#[derive(Parser)]`.
    -  The `#[grammar = "resources/choreo.pest"]` attribute links it directly to the grammar file. `pest` uses this file to automatically generate the parsing logic for the `ChoreoParser`.

2.  **`parse()` Function**:
    -  This is the public entry point of the module. It takes a string slice `source` containing the `choreo` script.
    -  `ChoreoParser::parse(Rule::grammar, source)` initiates the parsing process, starting from the top-level `grammar` rule defined in the `.pest` file.
    -  It iterates through the top-level statements of the script.
    -  A `match` statement dispatches each parsed statement (`pair`) to a specific helper function (e.g., `build_rule`, `build_actors_def`) based on its grammatical rule (`pair.as_rule()`).
    -  The results from these helper functions (which are `Statement` enums) are collected into a vector.
    -  Finally, it constructs and returns a `TestSuite` struct, which holds the vector of all parsed statements.

3.  **`build_*` Helper Functions**:
    -  These functions handle the logic for converting a specific part of the parse tree (`Pair<Rule>`) into its corresponding AST node.
    -  **`build_actors_def` / `build_outcomes_def`**: These functions parse simple lists of names. They iterate through the inner pairs, find all `identifier` rules, and collect their string values into a `Vec<String>`, which is then wrapped in a `Statement::ActorDef` or `Statement::OutcomeDef`.
    -  **`build_rule`**: This handles the complex `rule` structure. It navigates the inner pairs to extract the rule's name, then calls `build_conditions` and `build_actions` to parse the `when` and `then` blocks, respectively. It assembles these parts into an `ast::Rule` struct.
    -  **`build_setting`**: This extracts a key-value pair for a setting, converting the value into the appropriate `Value` enum variant (`String` or `Number`).
    -  **`build_conditions` / `build_actions`**: These functions iterate over a set of pairs from a `when` or `then` block. They use `filter_map` and a `match` statement to identify the specific type of condition or action (e.g., `time_condition`, `type_action`) and construct the corresponding AST enum variant (`Condition::Time`, `Action::Type`). They extract the necessary data, like actors, text, or commands, from the inner parts of the parsed rule.