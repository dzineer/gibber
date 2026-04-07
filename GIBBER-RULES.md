# Gibber Rules — Per-Node-Type Behavior

Version: `rules/v4`
Companion to: `GIBBER-SPEC.md` (grammar) and `GIBBER-DICTIONARY.md` (vocabulary).

This file is the **behavior layer** of Gibber. It tells the runtime (an AI agent, the translator, the verifier, the renderer, the wire compressor) what to do with each kind of AST node it encounters. The grammar says what a tree looks like. The dictionary says what each symbol means. The rules say what to do with each tree.

The rules are intentionally written in English so they are auditable, portable, and learnable by any AI on any model. Each rule entry is a structured list of contexts. A runtime reads this file once (or skips it via the fast-path version check) and from then on knows how to handle every defined node type in every defined context.

## Architecture: AST + Rules + Runtime

```
Layer 1: The AST                  (the data — pure tree, no behavior)
Layer 2: GIBBER-RULES.md          (the rules — what to do with each node)
Layer 3: The runtime (AI/script)  (executes the rules on the AST)
```

Adding new behaviors to Gibber means adding new contexts to existing rules. Adding new node types means adding new rule entries. Existing data files never need to change.

## Contexts

Every rule may define behavior in any of these contexts. A context describes a use case the runtime needs to handle.

| Context | Purpose |
|---|---|
| `validate` | Confirm the node has required fields with correct types. |
| `walk` | Identify the node's children for traversal in a known order. |
| `render-english` | Produce plain-English Markdown for the human half of a dual-language file or for direct display. |
| `translate-from-english` | Recognize an English description and produce the equivalent Gibber form. |
| `compress-wire` | Encode the node onto a runtime wire with maximum compactness. |
| `decompress-wire` | The inverse of `compress-wire`. |
| `verify` | Plant a marker, generate a canonical query, and define the expected response shape (used by the write-time verifier). |
| `diff` | Identify which fields are significant for change detection and which are cosmetic. |

A rule does not need to define every context. If a context is not defined, the runtime falls back to a sensible default (e.g. for `validate`, every defined field is treated as optional with a free-form value).

## Built-in operations available inside rules

Rules can reference these helpers, which any conformant runtime must implement:

- `(expand §sym)` — look up a symbol in the dictionary and return its English meaning.
- `(compress "english")` — the inverse: find the symbol whose English meaning matches the input.
- `(map FN LIST)` — apply FN to each element of LIST.
- `(join LIST SEP)` — join a list of strings with a separator.
- `(if COND TEXT)` — emit TEXT only if COND is non-empty.
- `(field NODE NAME)` — read the value of field NAME on NODE.
- `(template "string with {placeholders}")` — fill placeholders from the current node's fields.

These are the same as the built-in functions defined in `GIBBER-SPEC.md` plus a few rendering helpers. They are not part of the data; they are part of the rule language used to describe behavior.

---

## Rule entries

### `§task`

Used for task files (`tasks/current_task.md`, individual task entries inside `tasks/tasks_index.md`).

**validate**
- Required fields: `§id`, `§status`, `§owner`, `§goal`
- Optional fields: `§title`, `§depends`, `§blocks`, `§files`, `§done`, `§todo`, `§budget`, `§tests`, `§notes`, `§refs`, `§risk`, `§result`
- Types:
  - `§id` is an identifier (e.g. `T042`)
  - `§status` is one of: `§wip`, `§queued`, `§blocked`, `§verifying`, `§done`, `§abandoned`, `§deferred`
  - `§owner` is one of: `§ai`, `§human`, `§pair`, `§auto`
  - `§goal` is a form or string
  - `§depends`, `§blocks` are lists of identifiers
  - `§files` is a list of strings (file paths)
  - `§done`, `§todo`, `§tests` are lists of forms or symbols
  - `§budget` is a form

**walk**
- Children visited in this order: `§goal`, `§depends`, `§done`, `§todo`, `§tests`, `§budget`

**render-english**
- Template:
  ```
  Task {§id}{ if §title: " — " + §title}.
  Status: {(expand §status)}. Owner: {(expand §owner)}.
  {if §depends: "Depends on: " + (join §depends ", ") + "."}
  Goal: {(render §goal)}.
  {if §files: "Files: " + (join §files ", ") + "."}
  {if §done: "Done so far: " + (join (map render §done) ", ") + "."}
  {if §todo: "Todo: " + (join (map render §todo) ", ") + "."}
  {if §budget: "Budget: " + (render §budget) + "."}
  {if §tests: "Tests: " + (join (map render §tests) ", ") + "."}
  {if §notes: "Notes: " + §notes + "."}
  ```

**translate-from-english**
- Patterns the runtime recognizes (case-insensitive, lenient on punctuation):
  - `Task {ID} status {STATUS}` -> `(§task §id:{ID} §status:(compress {STATUS}))`
  - `Task {ID} owner {OWNER}` -> `(§task §id:{ID} §owner:(compress {OWNER}))`
  - `Task {ID} depends on {LIST}` -> `(§task §id:{ID} §depends:[{LIST}])`
  - Free-form text falls back to `(§task §id:{ID} §goal:"raw english here")`

**compress-wire**
- Omit fields whose value is the schema default.
- Inline single-child forms: `(§goal §build)` -> `§goal:§build`.

**verify**
- Marker field: `§glp_marker`
- Canonical query: `(§query §kind:§task §id:{§id})`
- Expected response shape: `(§result §kind:§task §id:{§id} §marker:{marker})`
- Auto-repair strategies: re-validate, regenerate marker, re-emit canonical query.

**diff**
- Significant fields: `§status`, `§goal`, `§depends`, `§done`, `§todo`, `§budget`, `§result`
- Cosmetic fields: `§notes`, `§refs`, `§updated`

---

### `§index`

Used for `tasks/tasks_index.md`. Lists all known tasks grouped by phase.

**validate**
- Required fields: `§id`, `§updated`, `§active`, `§phases`
- Optional: `§next`, `§later`
- `§phases` is a list of `§phase` forms

**walk**
- Children: `§phases`, then for each phase, its `§tasks` list

**render-english**
- Template:
  ```
  # Task Index
  Last updated: {§updated}.
  {if §next: "Currently working on: " + §next + "."}
  Active tasks: {(join §active ", ")}.
  {if §later: "Queued for later: " + (join §later ", ") + "."}

  ## Phases
  {for each phase in §phases:
    "### " + §title + " (" + §id + ")\n" +
    "- " + (join §tasks ", ") + "\n"}
  ```

**diff**
- Significant: `§active`, `§next`, `§phases`
- Cosmetic: `§updated`

---

### `§phase`

A grouping inside `§index`. Holds a phase ID, title, and the list of task IDs in that phase.

**validate**
- Required: `§id`, `§title`, `§tasks`
- `§tasks` is a list of identifiers

**walk**
- Children: `§tasks`

**render-english**
- Template: `{§title} ({§id}): {(join §tasks ", ")}`

---

### `§completed`

Used for `tasks/tasks_completed.md`.

**validate**
- Required: `§id`, `§updated`, `§tasks`
- `§tasks` is a list of completed task forms or references

**render-english**
- Template:
  ```
  # Completed Tasks
  Last updated: {§updated}.
  {if §tasks empty: "No tasks completed yet." else: list each task}
  ```

---

### `§learning`

Used for `learning.md` entries.

**validate**
- Required: `§id`, `§created`, `§context`, `§tried`, `§result-fix`
- Optional: `§lesson`
- `§result-fix` is `§fixed` or `§not-fixed`

**walk**
- Children: `§context`, `§tried`, `§lesson`

**render-english**
- Template:
  ```
  ## Learning {§id} — {§created}
  Context: {(render §context)}.
  Tried: {(join (map render §tried) "; ")}.
  Result: {(expand §result-fix)}.
  {if §lesson: "Lesson: " + (render §lesson) + "."}
  ```

**diff**
- Significant: every field. Learning entries are append-only and meaningful as a whole.

---

### `§session`

Used for `session.md` entries.

**validate**
- Required: `§id`, `§created`, `§prompt`, `§response`
- Optional: `§decisions`, `§artifacts`

**walk**
- Children: `§prompt`, `§response`, `§decisions`, `§artifacts`

**render-english**
- Template:
  ```
  ## {§id} — {§created}
  Prompt: {(render §prompt)}.
  Response: {(render §response)}.
  {if §decisions: "Decisions: " + (join (map render §decisions) "; ") + "."}
  {if §artifacts: "Artifacts: " + (join §artifacts ", ") + "."}
  ```

**compress-wire**
- Drop `§created` if the runtime can infer it from message order.

---

### `§memory`

Used for `memory/MEMORY.md` entries.

**validate**
- Required: `§id`
- Optional: `§title`, `§content`, `§updated`, `§entries` (a memory file may be a flat list of entries)

**walk**
- Children: `§content` or `§entries`

**render-english**
- For a single entry:
  ```
  ## {§id}{if §title: " — " + §title}
  {(render §content)}
  ```
- For a memory file with `§entries`:
  ```
  # Memory Dump
  Last updated: {§updated}.

  {for each entry in §entries: render the entry}
  ```

**diff**
- Significant: `§content`, `§entries`
- Cosmetic: `§updated`, `§title`

---

### `§budget`

Used inside `§task` to express time, latency, and accuracy budgets.

**validate**
- Optional fields: `§time`, `§lat`, `§p50`, `§p99`, `§throughput`, `§mem`, `§size`, `§acc`, `§recall`, `§precision`
- Each field's value is a number with a unit suffix or a comparator expression like `<2d` or `=100%`

**render-english**
- Template:
  ```
  {join (for each defined field: (expand fieldname) + ": " + value) ", "}
  ```
  Example: `Time: less than 2 days, p50 latency: less than 1 ms, p99: less than 3 ms, accuracy: 100%`

---

---

## Explanation and verification node types (added in `rules/v4`)

These node types support the **task closeout flow**: every completed task carries an `§explanation` block with falsifiable claims, and an independent verifier produces a `§verification` form recording whether those claims hold.

### `§explanation`
- **validate** — Required: `§did`. Optional: `§claims`, `§deferred`, `§learned`.
- **walk** — `§did`, `§claims`, `§deferred`, `§learned`.

### `§claim`
- **validate** — Required: at least one of `§exists`, `§contains`, `§matches`, `§passed`, `§equals`, `§absent` (the predicate). Optional: `§file`, `§cmd`, `§value`, `§pattern`, `§notes`.

### `§verification`
- **validate** — Required: `§task`, `§verified-by`, `§verified-at`, `§verdict`, `§claims-checked`, `§claims-passed`, `§claims-failed`. Optional: `§rejected-claims`, `§notes`.
- **walk** — `§task`, `§verdict`, `§rejected-claims`.

### Verifier flow

1. The working agent finishes a task and writes a complete `§explanation` block as part of the completed-task entry in `tasks_completed.gibber`. The block must include a `§claims` list with at least one falsifiable claim.
2. The working agent runs `~/.claude/shared/gibber/tools/gibber-verify <task-id>` which spawns a fresh `claude -p` invocation with no prior context.
3. The verifier reads only the completed-task entry and the project's `CLAUDE.md`. It does not see the conversation that produced the task.
4. The verifier checks each `§claim` mechanically against the filesystem and any commands it can re-run.
5. The verifier emits a single `§verification` form to a known output path.
6. The working agent reads the `§verification` form and either:
   - On `§verdict:§verified`: leaves the task as `§done` and proceeds.
   - On `§verdict:§rejected`: changes the task status to `§verifying-failed`, fixes the rejected claims, regenerates the `§explanation`, and re-runs `gibber-verify`.

The working agent never marks a task fully complete without a passing `§verification` from a fresh agent. This is the standard task closeout flow.

---

## Tool-output node types (added in `rules/v2`, slimmed in `rules/v3`)

These node types appear in the gibber-only output of `gibber-run` and any other Gibber Tools wrapper. **Tool output is gibber-only.** No English rendering happens inside the wrapper. Only the AI ever reads tool output. If a human asks the AI to summarize a tool result, the AI produces English on the spot using its general knowledge of the symbols in `tools/tools-dictionary.md` — no per-node-type render template is needed.

The vocabulary for these symbols lives in `tools/tools-dictionary.md`. Each entry below provides only `validate` and `walk`. There is intentionally no `render-english` because the wrapper never invokes it.

### `§result`
- **validate** — Required: `§tool`, `§cmd`, `§outcome`. Optional: `§errors`, `§warnings`, `§tests`, `§summary`, `§commits`, `§files`, `§staged`, `§unstaged`, `§untracked`, `§conflicted`, `§branch`, `§head`, `§ahead`, `§behind`, `§remotes`, `§duration`, `§exit-code`.
- **walk** — `§errors`, `§warnings`, `§tests`, `§summary`, `§files`, `§commits`.

### `§diagnostic`
- **validate** — Required: `§level`. Optional: `§code`, `§file`, `§line`, `§col`, `§message`, `§suggestion`.

### `§test`
- **validate** — Required: `§name`, `§outcome`. Optional: `§module`, `§assertion`, `§expected`, `§actual`, `§backtrace`.

### `§commit`
- **validate** — Required: `§sha`. Optional: `§author`, `§subject`, `§date`.

### `§file-entry`
- **validate** — Required: `§file`. Optional: `§added`, `§removed`, `§size`, `§mtime`, `§perm`, `§owner`, `§group`.

### `§summary`
- **validate** — Optional fields: `§errors-count`, `§warnings-count`, `§passed-count`, `§failed-count`, `§skipped-count`, `§total`, `§compiled`, `§total-changes`.

### `§remote`
- **validate** — Required: `§name`, `§url`.

---

### Built-in functions (`§rep`, `§range`, `§ref`, `§sub`, `§expand`, `§compress`, `§concat`, `§lookup`)

These are defined in `GIBBER-SPEC.md`. They are not data nodes — they are operations the runtime executes on read. The rules above use them in templates and patterns. A conformant runtime must implement them as described in the spec.

---

## How a runtime uses this file

1. On startup (or on Gibber-version change), read this file once. Build an internal lookup table from node-type symbol to rule entry.
2. When parsing a Gibber file, for each node type encountered, look up the rule and apply the appropriate context:
   - When loading: apply `validate` and (if needed) `decompress-wire`.
   - When a human asks to see a `.gibber` file as English: apply `render-english` and either print to the conversation or write the result to a derivative `.human` file.
   - When the user types English: apply `translate-from-english`.
   - When sending a node over a runtime wire: apply `compress-wire`.
   - When the verifier writes a record: apply `verify` to plant the marker and produce the canonical query.
   - When comparing two versions of a node: apply `diff`.
3. If the parser encounters a node type not in this rules file, the runtime emits a clear error and stops. Unknown node types are not silently ignored.

## How to extend the rules

Projects with domain-specific node types add their own rule entries in a file named `gibber-rules-<project>.md` in the project root, alongside the project's `gibber-dict-<project>.md`. The runtime loads both the base rules and the extension on startup.

When you add a new node type to a project:

1. Add the symbol to the project's extension dictionary (with an English definition).
2. Add a rule entry for it in the project's extension rules file (with at minimum `validate`, `walk`, and `render-english`).
3. Bump the project's local rules version.
4. Existing files using the new node type now work without further changes.

## Versioning

This file is `rules/v1`. The full Gibber version triple is in `GIBBER-VERSION` and is referenced from every project's `CLAUDE.md` Gibber Protocol block. Any change to this file that affects behavior must bump `rules/vN` and update `GIBBER-VERSION`. Backward-compatible additions (new optional contexts, new node types) keep the same version. Breaking changes (removed contexts, type changes) bump the version and require all consumers to re-read.
