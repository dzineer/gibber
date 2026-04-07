# Gibber

Gibber is a compact symbolic file format for AI agent project files. It is designed to be **read by AI agents, not by humans**. Files written in Gibber use a small S-expression-style notation that is several times denser than the equivalent English, lets long agent sessions hold dramatically more historical context, and is fully self-describing through a versioned dictionary.

Current version: **`gibber/3 dict-meta/v1 rules/v3 tools/v1`**

## Why Gibber exists

As AI agents become more capable, more of an agent's working files (task lists, learning logs, session notes, memory dumps, tool outputs) are written **by the agent for the agent** with very little human review in the middle. The right design assumption is that the human will not be reading these files. Build for the agent.

Gibber follows that principle. It is dense, low-token, structured, and designed so an AI can read and write it as fluently as English while burning a fraction of the tokens. When a human does want to see what's in a Gibber file, the agent generates a plain-English `.human` view on demand from the matching `.gibber` source.

## The two-file model

- **`.gibber`** — the source of truth. Holds a single Gibber AST form. Read and edited by the agent. No English section.
- **`.human`** — derivative. Generated on demand by the agent when a human asks to see a `.gibber` file in plain English. Read-only and ephemeral. Generally gitignored.

The `.gibber` file is always authoritative. The `.human` file is always derived.

## What's in this directory

- `GIBBER-VERSION` — the canonical version triple. The first thing every agent reads.
- `GIBBER-SPEC.md` — the grammar, the file model, the built-in functions.
- `GIBBER-DICTIONARY.md` — the standard meta-work dictionary `meta/v1`.
- `GIBBER-RULES.md` — per-node-type behavior rules (`validate`, `walk`, `render-english`, etc.).
- `GIBBER-CLAUDE.md` — a snippet to paste into any project's `CLAUDE.md` so the agent learns Gibber on its next read.
- `templates/` — starter `.gibber` templates for task files, learning logs, session logs, memory dumps, and the task index.
- `tools/` — Gibber Tools: a wrapper called `gibber-run` that compresses CLI tool output (like `cargo build` or `git status`) into compact Gibber forms instead of letting raw multi-thousand-token output flood the agent's context.
- `gibber-install.sh` — one-shot installer that sets up Gibber in any project.
- `README.md` — this file.

## Install in a project

From the project root:

```sh
~/.claude/shared/gibber/gibber-install.sh
```

Or with an explicit path:

```sh
bash ~/.claude/shared/gibber/gibber-install.sh /path/to/project
```

The installer is idempotent. It:

1. Creates the project's extension dictionary `gibber-dict-<project>.md`.
2. Appends the Gibber Protocol block to the project's `CLAUDE.md` (creating it if missing).
3. Adds `*.human` to the project's `.gitignore`.
4. Seeds `.gibber` task files, learning log, session log, and memory dump from templates.

## Fast-path version check (the optimization that makes Gibber free for known agents)

When an AI agent enters a Gibber-using project, the very first thing it does is check whether it already knows Gibber at the version this project uses. If yes, it skips the teaching files entirely (~10k tokens of spec, dictionary, and rules) and starts working immediately. If no, it reads the three teaching files, then writes a memory note so future sessions take the fast path.

The version string lives in `GIBBER-VERSION` and is referenced from every project's `CLAUDE.md`. This means the teaching cost is paid **once per agent per machine per version**, not once per session.

## Sharing Gibber

The entire Gibber distribution is the contents of this directory. To share with another developer:

```sh
tar czf gibber.tar.gz -C ~/.claude/shared gibber/
```

Send the tarball. The recipient extracts it into their own `~/.claude/shared/` and runs `gibber-install.sh` in any project they want to enable. Their AI agents will speak Gibber from the next message onward.

## Gibber Tools (`tools/`)

`gibber-run` is a wrapper script that compresses CLI tool output before it reaches an agent's context. Example:

```sh
$ gibber-run cargo build
(§result §tool:cargo §cmd:build §outcome:§passed
  §summary:(§errors-count 0 §warnings-count 0))
```

That two-line Gibber form replaces what would otherwise be hundreds or thousands of lines of cargo's compilation chatter. The agent reads it instantly, and the saved tokens stretch the conversation budget dramatically over a long session.

Tools currently supported with first-class rules: `cargo`, `git`. Others fall back to ANSI-stripped raw output.

## Using the skill

There is a Claude Code skill at `~/.claude/skills/gibber/SKILL.md` that wraps the protocol so an agent can invoke it as a first-class capability. The skill performs the fast-path version check, falls through to teaching when needed, and handles project extension loading.

## Extending the dictionary for a project

Each project has its own `gibber-dict-<project>.md` file in the project root that lists project-specific symbols (component names, domain verbs, etc.) on top of the base `meta/v1` dictionary. The agent loads both on startup. To add a symbol, edit the extension file and write a one-line English definition.

Never use a symbol that is not defined in either the base or the extension. If you need one, define it first.

## Status

Gibber is private and experimental. The protocol is stable enough to use, but the Rust runtime, conformance test suite, editor plugins, and additional language bindings (Python, JavaScript) are not yet built. The current implementation of `gibber-run` is a portable shell script that uses `jq` for JSON parsing. A canonical Rust implementation will follow.
