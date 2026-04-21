# Senior Engineer Review: choreo — Second Impression

**Reviewer:** Staff Engineer, Platform Tooling  
**Subject:** Re-evaluation of choreo after maintainer addressed v0.12.0 feedback  
**Scope:** 5 `.chor` files for tbdflow (40 tests), full documentation site  
**Date:** 2026-02-28

---

## Context

Three weeks ago I filed six improvement suggestions after building a real test suite with choreo. The maintainer shipped
fixes for three of them in a single release cycle (#1 task calls in `after`, #3 unused actor lint, #5 `set_cwd` docs)
and put the remaining three on a public roadmap. That turnaround alone says something about the project's health.

This is a re-review of the same tbdflow test suite after adopting those fixes.

---

## The Test Suite at a Glance

| File                        | Tests | Purpose                                                  | Key choreo features used                              |
|-----------------------------|-------|----------------------------------------------------------|-------------------------------------------------------|
| `tbdflow_branch.chor`       | 4     | Branch naming conventions                                | tasks, `set_cwd`, `cleanup_repo()` in `after`         |
| `tbdflow_commit.chor`       | 11    | Conventional commit creation + rejection                 | dual-scenario scoping, action vs condition task split |
| `tbdflow_commit_types.chor` | 12    | Data-driven coverage of all 11 commit types              | `foreach`, `System log`, list variables               |
| `tbdflow_complete.chor`     | 6     | Full branch lifecycle (create → commit → merge → verify) | 6-step dependency chain, `sync()` / `check_status()`  |
| `tbdflow_sync_status.chor`  | 7     | Status, sync, and changelog generation                   | `init_repo_with_tag()`, dual scenarios                |

**40 tests. 0 failures. 0 skipped. Every file cleans up after itself.**

---

## What Has Improved Since the Last Review

### 1. `after` Blocks Are Now First-Class

The single most satisfying change. Before:

```choreo
after {
    Terminal run "rm -rf ${REPO_DIR} ${BARE_REPO}"   # implementation leak
}
```

After:

```choreo
after {
    cleanup_repo("${REPO_DIR}", "${BARE_REPO}")      # intent only
}
```

This was the last place where raw shell commands bled into the specification layer. Now every block in every file —
`given`, `when`, `then`, `after` — speaks in tasks. The Four-Layer Model is fully realised with zero exceptions. That's
a clean architecture that I'd hold up as a reference implementation.

### 2. The Unused Actor Lint (W019) Works

We previously declared `FileSystem` in `tbdflow_branch.chor` and `tbdflow_commit.chor` without using it. After the
linter update, we cleaned those up. The actors declarations are now truthful:

```
tbdflow_branch.chor         → actors: Terminal
tbdflow_commit.chor         → actors: Terminal
tbdflow_commit_types.chor   → actors: Terminal, System
tbdflow_complete.chor       → actors: Terminal
tbdflow_sync_status.chor    → actors: Terminal, System
```

`System` is used exactly where `System log` appears. No waste, no noise. The lint catches this — good.

---

## Design Assessment: The Files as a Whole

### Consistency Is the Standout Quality

Every file follows the exact same skeleton:

```
1.  Comment block: User Story
2.  feature / actors / settings / var
3.  ═══ IMPLEMENTATION LAYER ═══
      Setup drivers    → init_repo(), init_repo_with_tag()
      Action drivers   → commit(), create_branch(), sync(), check_status()
      Condition drivers → verify_commit_succeeded(), verify_working_tree_clean()
      Cleanup driver   → cleanup_repo()
4.  ═══ BUSINESS SPECIFICATION LAYER ═══
      AC comments
      Scenarios with tests
      after { cleanup_repo(...) }
```

A developer opening any of these files for the first time knows exactly where to look for what. That's the mark of a
mature test framework — not just that it *can* produce clean tests, but that it *guides* you toward a consistent shape.

### Task Naming Reads Like a Domain Vocabulary

Across all 5 files, the task names form a coherent vocabulary for the tbdflow domain:

| Verb         | Tasks                                                                                                                                                                                                                                             |
|--------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Create**   | `init_repo`, `init_repo_with_tag`, `create_branch`, `create_branch_with_issue`, `make_change`                                                                                                                                                     |
| **Act**      | `commit`, `commit_scoped`, `commit_breaking`, `commit_with_issue`, `complete_branch`, `sync`, `check_status`, `generate_unreleased_changelog`                                                                                                     |
| **Verify**   | `verify_commit_succeeded`, `verify_commit_rejected`, `verify_history_contains`, `verify_branch_created`, `verify_branch_merged_and_deleted`, `verify_on_main_with_commit`, `verify_working_tree_clean`, `verify_status_clean`, `verify_no_errors` |
| **Teardown** | `cleanup_repo`, `return_to_main`                                                                                                                                                                                                                  |

Every task name answers *what* it does, not *how*. `verify_branch_merged_and_deleted()` doesn't say "check exit code and
grep for string" — it declares the business outcome. This is exactly what the choreo documentation prescribes, and it
works.

### The Action / Condition Split Is Natural

choreo enforces that `when:` blocks contain actions and `then:` blocks contain conditions. Initially this felt like a
constraint, but it produced a healthy design discipline:

```choreo
# ACTION task — wraps a Terminal run (goes in when:)
task sync() {
    Terminal run "tbdflow sync"
}

# CONDITION task — wraps assertions (goes in then:)
task verify_history_contains(expected_message) {
    Terminal last_command succeeded
    Terminal output_contains "${expected_message}"
}

# Used together — clear separation
when:
    sync()
then:
    verify_history_contains("feat: add user greeting")
```

This forced us to decompose "verify the commit is in history" into two concerns: *doing* the sync (action) and
*asserting* the content (condition). The test reads better for it.

### `foreach` Eliminates a Whole Category of Boilerplate

`tbdflow_commit_types.chor` is 82 lines and tests 11 commit types. In Cucumber this would be a Scenario Outline with an
11-row Examples table, plus step definitions, plus a support file. In BATS it would be 11 nearly-identical test
functions. In choreo:

```choreo
var COMMIT_TYPES = ["feat", "fix", "chore", "docs", "refactor", "test", "build", "ci", "perf", "revert", "style"]

foreach TYPE in ${COMMIT_TYPES} {
    test "Commit_${TYPE}" "tbdflow accepts type '${TYPE}'" {
        given:
            Test has_succeeded Setup
        when:
            make_change_and_commit("${TYPE}")
        then:
            verify_commit_succeeded()
    }
}
```

Each generated test has a unique name, runs independently, and appears individually in the report. This is data-driven
testing done right.

---

## What I'd Still Like to See

### 1. Shared task imports (roadmapped)

`init_repo()` and `cleanup_repo()` are copy-pasted across all 5 files. With an `import` mechanism:

```choreo
import "tests/shared/git_tasks.chor"
```

…we'd define them once. For a 5-file suite it's tolerable. For a 50-file suite testing a larger CLI, it would be a hard
blocker. Glad this is on the roadmap.

### 2. Directory-level runner (roadmapped)

We currently run tests in a shell loop:

```bash
for f in tests/*.chor; do choreo run -f "$f"; done
```

A first-class `choreo run --dir tests/` with aggregated reporting and a unified exit code would make CI integration
seamless. Also on the roadmap.

### 3. Minor: `task_example.chor` is still in the tests directory

The example file from the documentation (`task_example.chor`) lives alongside the real tests. It should either be moved
to `docs/examples/` or excluded from the test directory to avoid confusion when running all `.chor` files.

---

## The Syntax Itself

After spending a full day with the DSL, some observations on the language design:

**What feels right:**

- `given: / when: / then:` with colons — instantly familiar to anyone who's seen BDD
- `Test has_succeeded Setup` — reads like English, declarative dependency
- `Terminal output_contains "..."` / `Terminal last_command succeeded` — verb-first, clear subject
- Task call syntax `task_name("arg1", "arg2")` — no ceremony, no decorators
- `var`, `foreach`, `${substitution}` — the right level of programmability without becoming a general-purpose language

**What feels slightly off (nitpicks, not blockers):**

- `actors: Terminal, System` uses a comma-separated shorthand, but the docs also show `actors { Terminal \n System }` —
  having two syntaxes for the same thing can confuse new users
- The `#` comment style is fine but there's no block comment syntax; the `═══` dividers work, but a `/* */` or
  doc-comment would be tidier for the User Story preamble

---

## Final Verdict

choreo has matured from "promising tool with rough edges" to "tool I would standardise on for CLI acceptance testing."
The three fixes shipped since our first review were not cosmetic — they closed real abstraction gaps:

| Issue                    | Impact                                                               |
|--------------------------|----------------------------------------------------------------------|
| Tasks in `after` blocks  | Eliminated the last implementation leak from the specification layer |
| Unused actor lint (W019) | Keeps actor declarations honest — catches drift early                |
| `set_cwd` docs           | Prevents a confusing debugging session for every new user            |

The test suite we've built is, frankly, the cleanest BDD suite I've seen for a CLI tool. Five files. Forty tests. Every
file follows the same structure. Every scenario reads like a specification. Every task names an intent. Every `after`
block calls a task. No glue code. No regex step matching. No separate support files.

**Rating: 9/10** — up from 8.5. The remaining point is for imports and the directory runner, both of which are
acknowledged and roadmapped. When those land, this is a 10.

I'm recommending choreo for our platform team's CLI testing standard.
