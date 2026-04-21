---
layout: default
title: Imports & Shared Tasks
---

# Imports: Sharing Tasks Across Files

As your test suite grows, you'll find yourself copying the same task definitions (setup helpers, teardown routines,
verification drivers) across multiple `.chor` files. The `import` keyword lets you define tasks once in a shared file
and reuse them everywhere.

## The Problem: Copy-Paste Drift

Consider a CLI test suite with five files, each of which needs to initialise a git repository and clean it up
afterwards:

```
tests/
├── branch.chor          ← defines init_repo(), cleanup_repo()
├── commit.chor          ← defines init_repo(), cleanup_repo()  (copy)
├── commit_types.chor    ← defines init_repo(), cleanup_repo()  (copy)
├── complete.chor        ← defines init_repo(), cleanup_repo()  (copy)
└── sync_status.chor     ← defines init_repo(), cleanup_repo()  (copy)
```

Five copies of the same tasks. If you need to change the setup logic, you have to
update all five files. Miss one, and your tests will diverge.

## The Solution: `import`

Extract shared tasks into a dedicated file and import them:

```
tests/
├── shared/
│   └── git_tasks.chor   ← defines init_repo(), cleanup_repo() once
├── branch.chor          ← import "shared/git_tasks.chor"
├── commit.chor          ← import "shared/git_tasks.chor"
├── commit_types.chor    ← import "shared/git_tasks.chor"
├── complete.chor        ← import "shared/git_tasks.chor"
└── sync_status.chor     ← import "shared/git_tasks.chor"
```

One source of truth. Change it once, every file picks it up.

## Syntax

```choreo
import "path/to/shared_tasks.chor"
```

The import path is **relative to the directory of the importing file**. Place `import` statements at the top of your
file, before `feature`, `actors`, and `scenario` declarations.

## What Gets Imported

Only **task definitions** (`task`) and **variable definitions** (`var`) from the imported file are brought into scope.
Everything else like `feature`, `actors`, `settings`, `scenario`, `background` is ignored. This keeps imports focused:
shared files provide *drivers*, not *specifications*.

| Statement    | Imported? | Reason                                    |
|:-------------|:----------|:------------------------------------------|
| `task`       | ✅ Yes     | Shared drivers are the primary use case   |
| `var`        | ✅ Yes     | Shared constants (URLs, paths) are useful |
| `feature`    | ❌ No      | Each file declares its own feature        |
| `actors`     | ❌ No      | Each file declares its own actors         |
| `settings`   | ❌ No      | Settings are file-specific                |
| `scenario`   | ❌ No      | Scenarios belong to the importing file    |
| `background` | ❌ No      | Background blocks are file-specific       |

## Creating a Shared Tasks File

A shared tasks file is a valid `.chor` file that contains only task and variable definitions. You can validate and lint
it independently.

### Example: `shared/git_tasks.chor`

```choreo
/*
    Shared git tasks for CLI test suites
    This file is imported by test files that need git repo setup/teardown.
*/

# Setup: initialise a bare repo and a working clone
task init_repo(repo_dir, bare_repo) {
    Terminal run "mkdir -p ${repo_dir} ${bare_repo}"
    Terminal run "git init --bare ${bare_repo}"
    Terminal run "git clone ${bare_repo} ${repo_dir}"
    Terminal run "cd ${repo_dir} && git commit --allow-empty -m 'Initial commit'"
}

# Teardown: remove repo directories
task cleanup_repo(repo_dir, bare_repo) {
    Terminal run "rm -rf ${repo_dir} ${bare_repo}"
}

# Verify the working tree is clean
task verify_working_tree_clean() {
    Terminal last_command succeeded
    Terminal output_contains "nothing to commit, working tree clean"
}
```

You can validate it just like any other `.chor` file:

```bash
choreo validate -f tests/shared/git_tasks.chor
# ✅ Test suite is valid.
```

## Using Imports in a Test File

```choreo
# Import shared tasks — path relative to this file's directory
import "shared/git_tasks.chor"

feature "Branch Naming Conventions"
actors: Terminal

var REPO_DIR = "/tmp/test_repo"
var BARE_REPO = "/tmp/test_bare"

scenario "Branch creation follows conventions" {
    test Setup "Initialise repository" {
        given:
            Test can_start
        when:
            init_repo("${REPO_DIR}", "${BARE_REPO}")
        then:
            Terminal last_command succeeded
    }

    test VerifyClean "Working tree is clean after setup" {
        given:
            Test has_succeeded Setup
        when:
            Terminal run "cd ${REPO_DIR} && git status"
        then:
            verify_working_tree_clean()
    }

    after {
        cleanup_repo("${REPO_DIR}", "${BARE_REPO}")
    }
}
```

The imported tasks — `init_repo()`, `cleanup_repo()`, and `verify_working_tree_clean()` — are available in `given`,
`when`, `then`, and `after` blocks, exactly as if they were defined in the same file.

## Nested Imports

Imported files can themselves contain `import` statements. Paths in nested imports are resolved relative to the
*imported* file's directory, not the original file's directory.

```
tests/
├── shared/
│   ├── base_tasks.chor      ← defines cleanup_repo()
│   └── git_tasks.chor       ← import "base_tasks.chor"  (sibling file)
└── my_test.chor             ← import "shared/git_tasks.chor"
```

In `git_tasks.chor`:

```choreo
import "base_tasks.chor"    # resolves to tests/shared/base_tasks.chor
```

In `my_test.chor`:

```choreo
import "shared/git_tasks.chor"   # resolves to tests/shared/git_tasks.chor
                                  # which in turn imports base_tasks.chor
```

## Circular Import Protection

`choreo` tracks which files have already been imported using their canonical (absolute) paths. If a file is imported a
second time, it is skipped. This prevents infinite loops:

```choreo
# a.chor
import "b.chor"    # ← imports b.chor

# b.chor
import "a.chor"    # ← skipped, a.chor is already in the import chain
```

When running with `--verbose`, choreo will log a message when a duplicate import is skipped.

## Local Definitions Take Priority

If the importing file defines a task with the same name as an imported task, **the local definition wins**. Imported
tasks are loaded first, and local definitions overwrite them. This lets you override a shared task when a specific file
needs different behaviour:

```choreo
import "shared/git_tasks.chor"      # defines init_repo(repo_dir, bare_repo)

# Override init_repo for this file — add an extra config step
task init_repo(repo_dir, bare_repo) {
    Terminal run "mkdir -p ${repo_dir} ${bare_repo}"
    Terminal run "git init --bare ${bare_repo}"
    Terminal run "git clone ${bare_repo} ${repo_dir}"
    Terminal run "cd ${repo_dir} && git config user.email 'test@example.com'"
    Terminal run "cd ${repo_dir} && git commit --allow-empty -m 'Initial commit'"
}
```

## Error Handling

`choreo` provides clear error messages for import problems:

**File not found:**

```
Error: Import error: Import path not found: 'shared/missing.chor'
       (resolved to '/path/to/tests/shared/missing.chor')
```

**Parse error in imported file:**

```
Error: Import error: Parse error in imported file 'shared/broken.chor':
       expected identifier at line 5, column 10
```

## Best Practices

### Organise Shared Files in a `shared/` Directory

Keep shared task files in a dedicated directory to make the structure clear:

```
tests/
├── shared/
│   ├── git_tasks.chor
│   ├── docker_tasks.chor
│   └── api_tasks.chor
├── feature_a.chor
└── feature_b.chor
```

### Name Shared Files by Domain

Use descriptive names that reflect the domain the tasks cover:

- `git_tasks.chor` — git repository setup/teardown
- `docker_tasks.chor` — container lifecycle management
- `api_tasks.chor` — shared HTTP authentication and request helpers

### Keep Shared Files Focused

Each shared file should cover a single domain. Don't create a single `all_tasks.chor` with everything — it makes it
harder to understand dependencies and increases the chance of naming conflicts.

### Validate Shared Files Independently

Run `choreo validate` and `choreo lint` on your shared files as part of CI. They're valid `.chor` files and should be
treated as first-class citizens:

```bash
choreo validate -f tests/shared/git_tasks.chor
choreo lint -f tests/shared/git_tasks.chor
```

## Summary

| Aspect                 | Details                                            |
|:-----------------------|:---------------------------------------------------|
| **Syntax**             | `import "relative/path/to/file.chor"`              |
| **Path resolution**    | Relative to the importing file's directory         |
| **What's imported**    | `task` and `var` definitions only                  |
| **Nested imports**     | Supported, paths relative to the nested file       |
| **Circular imports**   | Automatically detected and skipped                 |
| **Override behaviour** | Local definitions overwrite imported ones          |
| **Validation**         | Shared files can be validated/linted independently |

