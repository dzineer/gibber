# Gibber Meta-Work Dictionary (`meta/v1`)

This is the project-agnostic base dictionary for gibber-format internal working files: task files, learning logs, session logs, memory dumps, and scratch notes. Every symbol here is available in any gibber file that declares `gibber_dict: meta/v1` in its frontmatter.

Projects may extend this dictionary with domain-specific symbols in their own `gibber-dict-<project>.md` file, listed in the project's `CLAUDE.md`.

Every symbol is defined in plain English. If a symbol you want to use is not in this dictionary or in your project's extension, add it to the extension before using it.

## Top-level forms

| Symbol | Meaning |
|---|---|
| `§task` | A task: a unit of work with id, status, owner, goal, etc. |
| `§index` | An index of tasks. The shape of `tasks_index.md`. |
| `§completed` | A completed-tasks list. The shape of `tasks_completed.md`. |
| `§learning` | A learning entry: a failure, what was tried, the outcome, the lesson. |
| `§session` | A session log entry: a prompt and a response. |
| `§memory` | A memory dump: persistent context for cross-session recall. |
| `§note` | A free-form scratch note. |

## Identity and metadata

| Symbol | Meaning |
|---|---|
| `§id` | Unique identifier (e.g. `T042`, `L007`, `S123`). |
| `§title` | Short human-readable title. |
| `§created` | Creation timestamp (ISO 8601). |
| `§updated` | Last-update timestamp (ISO 8601). |
| `§version` | Version of the file or entry. |
| `§provenance` | Where this came from (a source file, a prior task, an external doc). |
| `§since` | When something started or became true. |
| `§until` | When something ended or stopped being true. |

## Status values

| Symbol | Meaning |
|---|---|
| `§wip` | Work in progress. |
| `§queued` | Queued, not yet started. |
| `§blocked` | Blocked on a dependency or external factor. |
| `§verifying` | Implementation done, currently being verified. |
| `§done` | Completed and verified. |
| `§abandoned` | Stopped, will not be completed. |
| `§deferred` | Pushed to a later phase or version. |

## Owners

| Symbol | Meaning |
|---|---|
| `§ai` | The AI agent. |
| `§human` | The human user. |
| `§pair` | Joint work between AI and human. |
| `§auto` | Automated process (CI, hook, scheduled job). |

## Verbs (what the work is)

| Symbol | Meaning |
|---|---|
| `§build` | Construct something new. |
| `§design` | Plan or specify. |
| `§implement` | Write the code. |
| `§test` | Add or run tests. |
| `§bench` | Add or run benchmarks. |
| `§verify` | Confirm correctness against a spec or invariant. |
| `§refactor` | Restructure existing code without changing behavior. |
| `§port` | Translate from one language or platform to another. |
| `§migrate` | Move data or schema from one form to another. |
| `§integrate` | Connect components together. |
| `§document` | Write or update documentation. |
| `§spec` | Define or update a specification. |
| `§review` | Inspect work for correctness, quality, or alignment. |
| `§debug` | Diagnose a problem. |
| `§fix` | Repair a defect. |
| `§optimize` | Improve performance or efficiency. |
| `§install` | Set something up so it can be used. |
| `§remove` | Delete or uninstall. |
| `§rename` | Change a name without changing meaning. |
| `§extract` | Pull a piece out of a larger whole. |
| `§inline` | Fold a piece back into a larger whole. |
| `§generate` | Produce automatically from a source. |

## Task fields

| Symbol | Meaning |
|---|---|
| `§goal` | What the task aims to achieve. |
| `§depends` | Other task IDs this task depends on. |
| `§blocks` | Other task IDs this task is blocking. |
| `§files` | Files affected by this task. |
| `§done` | Subitems already completed within this task. |
| `§todo` | Subitems still to do within this task. |
| `§budget` | Time, latency, and accuracy budgets. |
| `§tests` | Tests required for this task to be considered done. |
| `§notes` | Free-form notes about the task. |
| `§refs` | References to docs, issues, prior tasks, external links. |
| `§risk` | Known risks or open questions. |
| `§result` | Outcome of the task once complete. |

## Index fields (for `tasks_index.md`)

| Symbol | Meaning |
|---|---|
| `§active` | Currently active tasks. |
| `§next` | The next task to start. |
| `§later` | Tasks queued for later phases. |
| `§phases` | Named phases that group tasks. |
| `§phase` | A single phase definition. |

## Learning entry fields

| Symbol | Meaning |
|---|---|
| `§context` | What was happening when the failure occurred. |
| `§tried` | What was attempted to fix it. |
| `§result-fix` | Whether the fix worked: `§fixed` or `§not-fixed`. |
| `§fixed` | The attempted fix worked. |
| `§not-fixed` | The attempted fix did not work. |
| `§lesson` | The takeaway for future work. |

## Session entry fields

| Symbol | Meaning |
|---|---|
| `§prompt` | The user's prompt (or its summary). |
| `§response` | The agent's response (or its summary). |
| `§decisions` | Decisions captured in this exchange. |
| `§artifacts` | Files or outputs produced in this exchange. |

## Budgets and metrics

| Symbol | Meaning |
|---|---|
| `§time` | Wall-clock time budget for the task. |
| `§lat` | Latency budget for the produced code. |
| `§p50` | 50th percentile latency. |
| `§p99` | 99th percentile latency. |
| `§throughput` | Operations per second budget. |
| `§mem` | Memory budget. |
| `§size` | Storage size budget. |
| `§acc` | Accuracy floor. |
| `§recall` | Recall metric (for retrieval tasks). |
| `§precision` | Precision metric. |
| `§floor` | A minimum threshold. |
| `§ceiling` | A maximum threshold. |

## Time units (used as suffixes on numbers)

| Suffix | Meaning |
|---|---|
| `ns` | Nanoseconds |
| `us` | Microseconds |
| `ms` | Milliseconds |
| `s` | Seconds |
| `m` | Minutes |
| `h` | Hours |
| `d` | Days |

## Size units (used as suffixes on numbers)

| Suffix | Meaning |
|---|---|
| `b` | Bytes |
| `kb` | Kilobytes |
| `mb` | Megabytes |
| `gb` | Gigabytes |

## Operators

| Symbol | Meaning |
|---|---|
| `<` | Less than. |
| `>` | Greater than. |
| `<=` | Less than or equal. |
| `>=` | Greater than or equal. |
| `=` | Equal. |

## Generic component words (often extended per project)

These are common across many projects but a specific project may rebind them in its extension dictionary.

| Symbol | Meaning |
|---|---|
| `§cli` | A command-line interface or binary. |
| `§api` | An application programming interface. |
| `§ui` | A user interface. |
| `§ws` | A WebSocket service. |
| `§mcp` | A Model Context Protocol server. |
| `§db` | A database. |
| `§store` | A storage layer. |
| `§index` | An index (database index, search index). |
| `§cache` | A cache. |
| `§queue` | A queue. |
| `§log` | A log. |
| `§schema` | A schema definition. |
| `§model` | A data model or ML model. |
| `§trait` | A trait/interface in code. |
| `§struct` | A struct/class in code. |
| `§fn` | A function. |
| `§module` | A module. |
| `§crate` | A Rust crate (or generic package). |
| `§test-unit` | A unit test. |
| `§test-int` | An integration test. |
| `§test-e2e` | An end-to-end test. |

## How to extend

Projects extend this dictionary by creating `gibber-dict-<project-name>.md` in the project root, listing additional symbols with English definitions. The project's `CLAUDE.md` references both `meta/v1` and the extension. Agents load both. Symbols defined in the extension override or add to the base.

When you find yourself wanting a symbol that isn't in either dictionary, add it to the extension first (with a clear definition) and then use it. Never use an undefined symbol.
