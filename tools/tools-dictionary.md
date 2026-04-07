# Gibber Tools Dictionary

This is the shared extension dictionary for tool-output Gibber forms. Loaded alongside the base `meta/v1` dictionary whenever `gibber-run` emits a filtered tool result.

Every symbol here is English-defined so an AI can learn the vocabulary from this one file.

## Top-level result forms

| Symbol | Meaning |
|---|---|
| `§result` | A tool-output result. The top-level form emitted by `gibber-run`. |
| `§tool` | The name of the tool that ran. |
| `§cmd` | The specific subcommand (e.g. `build`, `test`, `status`). |
| `§outcome` | The overall outcome of the command. |
| `§duration` | How long the command took. |
| `§exit-code` | The tool's exit code. |

## Outcomes

| Symbol | Meaning |
|---|---|
| `§passed` | The command succeeded with no problems. |
| `§failed` | The command failed. |
| `§warned` | The command succeeded but emitted warnings. |
| `§partial` | The command partially succeeded (some items passed, some failed). |
| `§skipped` | The command was skipped for some reason. |

## Diagnostics

| Symbol | Meaning |
|---|---|
| `§diagnostic` | A compiler or linter diagnostic (error, warning, note, help). |
| `§level` | The severity level of a diagnostic. |
| `§error` | Diagnostic severity: error. |
| `§warning` | Diagnostic severity: warning. |
| `§note` | Diagnostic severity: note (informational). |
| `§help` | Diagnostic severity: help (suggestion). |
| `§code` | An error code (e.g. `E0308`, `no-unused-vars`). |
| `§message` | The main human-readable diagnostic message. |
| `§file` | The file path the diagnostic refers to. |
| `§line` | The line number. |
| `§col` | The column number. |
| `§span` | A file:line:col range (start and end). |
| `§suggestion` | A fix suggestion attached to the diagnostic. |

## Counts and summaries

| Symbol | Meaning |
|---|---|
| `§summary` | A rolled-up summary of the command's outcome. |
| `§compiled` | Count of crates/modules that compiled successfully. |
| `§errors-count` | Count of errors. |
| `§warnings-count` | Count of warnings. |
| `§passed-count` | Count of passing tests/items. |
| `§failed-count` | Count of failing tests/items. |
| `§skipped-count` | Count of skipped tests/items. |
| `§total` | Total count. |

## Tests (for cargo test, pytest, go test, etc.)

| Symbol | Meaning |
|---|---|
| `§tests` | A list of test results. |
| `§test` | A single test entry. |
| `§name` | The test name. |
| `§module` | The module/package containing the test. |
| `§assertion` | The assertion text that failed. |
| `§expected` | The expected value in a failed assertion. |
| `§actual` | The actual value in a failed assertion. |
| `§backtrace` | A backtrace associated with a test failure. |

## Git

| Symbol | Meaning |
|---|---|
| `§branch` | A git branch name. |
| `§head` | The current HEAD reference. |
| `§staged` | Files staged for commit. |
| `§unstaged` | Modified but unstaged files. |
| `§untracked` | Untracked files. |
| `§conflicted` | Files with merge conflicts. |
| `§ahead` | Commits ahead of upstream. |
| `§behind` | Commits behind upstream. |
| `§commit` | A git commit reference. |
| `§sha` | A commit SHA. |
| `§author` | The commit author. |
| `§subject` | The commit subject line. |

## Kubernetes / Docker

| Symbol | Meaning |
|---|---|
| `§pod` | A Kubernetes pod. |
| `§container` | A container (pod container or docker container). |
| `§image` | A container image. |
| `§status` | A pod or container status. |
| `§running` | Status: running. |
| `§pending` | Status: pending. |
| `§crashloop` | Status: crashloopbackoff. |
| `§restarts` | Restart count. |
| `§namespace` | A Kubernetes namespace. |
| `§port` | A port number or mapping. |
| `§node` | A Kubernetes or Docker node. |

## Filesystem

| Symbol | Meaning |
|---|---|
| `§dir` | A directory entry. |
| `§file-entry` | A file entry (for `ls` and similar). |
| `§size` | Size in bytes or human-readable form. |
| `§mtime` | Modification time. |
| `§perm` | File permissions. |
| `§owner` | File owner. |
| `§group` | File group. |

## Package managers

| Symbol | Meaning |
|---|---|
| `§package` | A package entry. |
| `§added` | Packages added during an install. |
| `§removed` | Packages removed. |
| `§updated` | Packages updated. |
| `§version-range` | A semver range. |
| `§locked-version` | The version actually installed. |
