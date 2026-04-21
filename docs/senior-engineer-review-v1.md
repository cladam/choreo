# Senior Engineer Review: choreo BDD Framework — First Impression

**Reviewer:** Staff Engineer, Platform Tooling  
**Subject:** Evaluation of the `choreo` DSL based on a real-world test suite for `tbdflow` (5 `.chor` files, 40 tests)  
**Date:** 2026-02-07

---

## Executive Summary

choreo is a genuinely well-designed BDD tool. The `.chor` files we ended up with for tbdflow are the most readable
acceptance tests I've written for a CLI tool. The Four-Layer Model with tasks delivers on the promise of separating
business intent from implementation — without the usual Cucumber penalty of maintaining a separate glue-code layer. I
would adopt this for CLI and API testing.

**Rating: 8.5/10** — Production-ready for its niche, with a few rough edges to smooth out.

---

## What Works Exceptionally Well

### 1. The DSL Reads Like a Specification

The final state of our test files genuinely reads like executable acceptance criteria. Compare:

```choreo
test FeatBranch "Create a feature branch" {
    given:
        Test has_succeeded Setup
    when:
        create_branch("feat", "add-user-profile")
    then:
        verify_branch_created("feat/add-user-profile")
}
```

A product owner can read this. A new team member can read this. There is no ambiguity about what is being tested. The
task names (`create_branch`, `verify_branch_created`) describe intent, and the implementation is tucked away above. This
is the BDD ideal — and choreo achieves it without a separate step-definition layer.

### 2. Tasks Are the Right Abstraction

The task system hits a sweet spot:

- **No glue code** — tasks live in the same file, so there's no "where does this step map to?" confusion that plagues
  Cucumber.
- **Parameterised** — `init_repo(repo, bare)` and `commit_scoped(type, scope, message)` avoid the regex-matching
  fragility of traditional step definitions.
- **Properly scoped** — action tasks in `when:`, condition tasks in `then:`. Once we understood this constraint, the
  design fell into place naturally.

The documentation's guidance to "name tasks by intent, not implementation" led us to names like
`verify_commit_rejected()` rather than `check_exit_code_nonzero()`, which is exactly right.

### 3. Test Isolation Is First-Class

Each scenario gets its own repo (`/tmp/tbdflow_commit_valid`, `/tmp/tbdflow_commit_invalid`), and `after` blocks handle
cleanup unconditionally. The `set_cwd` action scoped per scenario means tests don't leak state. We never had a flaky
test due to ordering or shared state — that's rare for integration-level tests.

### 4. `foreach` Is a Killer Feature

`tbdflow_commit_types.chor` tests 11 commit types with a single `foreach` block. In Cucumber, this would be a Scenario
Outline with a full Examples table. In choreo, it's:

```choreo
foreach TYPE in ${COMMIT_TYPES} {
    test "Commit_${TYPE}" "tbdflow accepts type '${TYPE}'" {
        ...
        make_change_and_commit("${TYPE}")
        ...
    }
}
```

Data-driven testing with zero boilerplate. The generated test names (`Commit_feat`, `Commit_fix`, etc.) appear
individually in the report. Excellent.

### 5. Consistent, Opinionated Structure

Every file follows the same template:

1. User Story comment
2. `feature` / `actors` / `settings` / `var` declarations
3. **Implementation Layer** — tasks grouped by role (setup, action, condition)
4. **Business Specification Layer** — acceptance criteria comments, then scenarios

This consistency wasn't enforced by the tool — we chose it — but the DSL makes this structure natural. The visual weight
of the `═══` and `───` section dividers helps too.

### 6. Honest Error Messages

When things failed, choreo told us exactly what went wrong:

```
Stderr: cd: no such file or directory: tmp/tbdflow_commit_test && tbdflow commit ...
```

```
Checking if '...' contains 'Successfully committed'
```

The verbose mode (`--verbose`) prints every action as it executes, which made debugging the `set_cwd` scoping issue
possible. The Cucumber JSON report output is a bonus for CI integration.

---

## Rough Edges and Improvement Suggestions

### 1. Tasks Can't Be Called in `after` Blocks

We defined `cleanup_repo(repo, bare)` as a task but couldn't use it in `after`:

```choreo
after {
    cleanup_repo("${REPO_DIR}", "${BARE_REPO}")  # Parse error!
}
```

We had to fall back to raw `Terminal run "rm -rf ..."`. This breaks the abstraction — the cleanup is now the one place
where implementation leaks into the scenario. **Suggestion:** Allow task calls in `after` blocks.

### 2. `init_repo` Is Duplicated Across Files

The same `init_repo` task is copy-pasted into all 5 `.chor` files. choreo doesn't seem to have an `import` or
shared-task mechanism. For a suite of 5 files this is tolerable; for 50 files it would be a maintenance problem.

**Suggestion:** Support `import "shared_tasks.chor"` or a `tasks/` directory convention so common drivers can be defined
once.

### 3. `FileSystem` Actor Is Declared but Never Used

`tbdflow_branch.chor` and `tbdflow_commit.chor` declare `actors: Terminal, FileSystem` but never use `FileSystem`.
choreo doesn't warn about unused actors. A minor lint, but it would help keep files clean.

### 4. `then:` Block With Only a Task Call Feels Odd for Verify-Only Tests

For tests that only verify (no action of their own), we needed to split what was conceptually a single check into
`when:` + `then:`:

```choreo
test StatusClean "Status reports a clean working tree" {
    when:
        check_status()       # action task: runs tbdflow status
    then:
        verify_status_clean() # condition task: asserts the output
}
```

This is correct and readable, but we initially wrote it without a `when:` block, which caused a parse error. The
documentation could be clearer that **every test must have a `when:` block**. Alternatively, choreo could allow a test
with only `then:` if the `then:` block contains only conditions (asserting on the state left by a previous test's
`when:`).

### 5. Background `set_cwd` Doesn't Persist to Scenarios

We discovered that `set_cwd` in a `background` block doesn't carry over to scenarios. The background runs and sets the
cwd, but each scenario starts fresh. This is arguably correct for isolation, but it's surprising because other
background side-effects (like `set_header` in the Web actor example from the docs) do persist. **Suggestion:** Document
this explicitly, or make `set_cwd` in background persist like other background actions.

### 6. No Shared Reporting Across Files

Running 5 `.chor` files requires a shell loop. There's no built-in way to run a directory of `.chor` files and get a
single aggregated report. For CI, we'd want:

```
choreo run --dir tests/ --report combined.json
```

---

## Comparison to Alternatives

| Aspect                       | choreo              | Cucumber                   | BATS    | Custom shell scripts |
|------------------------------|---------------------|----------------------------|---------|----------------------|
| Readability for stakeholders | ★★★★★               | ★★★★☆                      | ★★☆☆☆   | ★☆☆☆☆                |
| No glue code layer           | ✅                   | ❌                          | ✅       | ✅                    |
| Parameterised reuse (tasks)  | ✅                   | Partial (Scenario Outline) | ❌       | Manual               |
| Data-driven (`foreach`)      | ✅                   | Scenario Outline           | ❌       | Manual               |
| Test dependencies            | ✅ (`has_succeeded`) | ❌                          | ❌       | Manual               |
| Built-in cleanup (`after`)   | ✅                   | ✅ (`@After` hooks)         | ❌       | Manual               |
| JSON reporting               | ✅                   | ✅                          | ✅ (TAP) | ❌                    |
| Multi-actor (Web + Terminal) | ✅                   | Via step defs              | ❌       | Manual               |

choreo occupies a unique position: it's as readable as Cucumber but as direct as BATS, with the architectural
sophistication of neither. The Four-Layer Model with tasks is a genuine innovation over both.

---

## Verdict

choreo is a tool I'd recommend for testing CLI tools and APIs where the team values:

- **Living documentation** that stakeholders can actually read
- **ATDD traceability** from user stories through acceptance criteria to executable tests
- **Fast feedback** without the overhead of maintaining a separate glue-code layer

The test suite we built for tbdflow is proof that the model works. Five files, 40 tests, clean structure, and every test
reads like a specification. The rough edges (no imports, no task calls in `after`, no directory-level runner) are all
solvable and don't block adoption.

I would start using this on my next project.
