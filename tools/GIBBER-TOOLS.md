# Gibber Tools

Gibber Tools is the subsystem that compresses CLI command output before it reaches an AI agent's context. It is the same "AST + rules + runtime" architecture as the rest of Gibber, applied to the output of external programs (`cargo`, `git`, `kubectl`, `docker`, `npm`, etc.).

Version: `tools/v1`
Companion to: `GIBBER-SPEC.md`, `GIBBER-DICTIONARY.md`, `GIBBER-RULES.md`.

## The problem

When an AI agent runs a CLI command, the raw output dumps into conversation context:

- The agent pays for 100% of the noise (banners, progress bars, ANSI codes, hints, timestamps).
- The agent uses ~5-20% of the content (the actual answer it needs).
- A single `cargo build` can be 2000 tokens to say "it built."
- Over a long session, tool output is often the single largest consumer of context budget.

## The fix

Every compressible CLI tool has a small **rule file** in `tools/rules/<tool>.rules` that defines:

1. How to invoke the tool with its most structured output mode (e.g. `cargo build --message-format=json-render-diagnostics`).
2. How to parse the raw output into an AST (JSON parser, line parser, regex, or Tree-sitter grammar).
3. Which AST nodes are relevant for the default agent use case ("errors + summary" vs "full trace" vs "summary only").
4. How to emit the filtered AST as a compact Gibber form the agent can read cheaply.
5. How to render that Gibber form back to English if the user asks to see the filtered output directly.

The wrapper script `gibber-run` reads the rule for the requested tool, invokes the tool correctly, parses the output, filters it, and emits Gibber. The agent sees a tiny structured result instead of a flood of raw text.

## Two mechanisms

### Mechanism 1 — Wrapper script (opt-in, ship today)

The agent runs `gibber-run cargo build` instead of `cargo build`. The wrapper does everything above. Simple, no harness changes, fully opt-in.

```
gibber-run [--raw] [--profile NAME] <command> [args...]
```

- `--raw`: bypass the filter, return the raw command output. For debugging.
- `--profile NAME`: pick a named profile from the tool's rule file (`errors`, `full`, `summary`, etc.). Each rule defines its own profiles.

### Mechanism 2 — Claude Code Bash hook (automatic, ship later)

A post-Bash hook in `~/.claude/settings.json` intercepts every `Bash` tool call, looks up the command in the rules directory, and rewrites the output through the same filter. Automatic for every command; the agent does not need to remember a wrapper.

## Rule file format

Each rule file is itself a Gibber form (English body + gibber header) in `tools/rules/<tool>.rules`. The schema is:

```
(§tool-rule §tool:<tool-name> §version:<version>
  §invoke:(§cmd <argv-template> §output:§json | §lines | §raw)
  §parse:(§format §json | §regex | §lines
          §schema:<schema-spec>)
  §profiles:[
    (§profile §name:<name>
      §keep:[<node-selector> ...]
      §drop:[<node-selector> ...]
      §render:(<gibber-template>))]
  §default-profile:<name>
  §render-english:(<template>))
```

- `§invoke` specifies how to actually run the tool. It may include flags we add automatically to get structured output.
- `§parse` specifies how to turn raw output into a traversable structure. For JSON the schema is implicit; for line-based or regex parsing it names the fields per line.
- `§profiles` is the list of named filters. Each profile says which nodes to keep, which to drop, and how to render the keepers as Gibber.
- `§default-profile` is used when no `--profile` flag is passed.
- `§render-english` is used when the user wants to see the filtered result in human prose instead of Gibber.

## What the agent sees

Without Gibber Tools:

```
$ cargo build
   Compiling eidetic-core v0.1.0 (/Users/dz/eidetic/crates/eidetic-core)
   Compiling eidetic-store v0.1.0 (/Users/dz/eidetic/crates/eidetic-store)
   ... (hundreds of lines) ...
error[E0308]: mismatched types
  --> crates/eidetic-core/src/lib.rs:42:17
   |
42 |     let x: u32 = "hello";
   |            ---   ^^^^^^^ expected `u32`, found `&str`
   |            |
   |            expected due to this
   ...
error: could not compile `eidetic-core` due to previous error
```
~1500 tokens.

With Gibber Tools:

```
(§result §tool:cargo §cmd:build §outcome:§failed
  §errors:[
    (§diagnostic §level:§error §code:E0308
      §file:"crates/eidetic-core/src/lib.rs" §line:42 §col:17
      §message:"mismatched types: expected u32, found &str")]
  §summary:(§compiled 0 §failed 1))
```
~60 tokens. The agent has every fact it needs to fix the bug.

## Where this fits in the Gibber distribution

```
~/.claude/shared/gibber/
  GIBBER-SPEC.md
  GIBBER-DICTIONARY.md
  GIBBER-RULES.md
  GIBBER-VERSION
  GIBBER-CLAUDE.md
  gibber-install.sh
  templates/
  tools/
    GIBBER-TOOLS.md              (this file — the spec)
    tools-dictionary.md          (per-tool symbol extension)
    gibber-run                   (the wrapper script)
    rules/
      cargo.rules
      git.rules
      kubectl.rules
      docker.rules
      npm.rules
      pytest.rules
      rustc.rules
      generic.rules              (ANSI+stopword strip fallback)
```

## Compounding with the rest of Gibber

Gibber Tools reuses everything:

- The same spec (S-expression grammar).
- The same dictionary infrastructure (base + extensions).
- The same rules architecture (per-node-type behavior).
- The same skill (an agent that already knows Gibber already knows how to read tool output).
- The same version-check fast path.

From the agent's perspective, tool output is just another kind of Gibber form it reads fluently, governed by the same rules system it already loaded. No new protocol. No new skill. Just a new dictionary extension and a set of per-tool rule files.

## Honest caveats

1. **Tool output formats change** with versions of the upstream tool. Rule files need updates when, e.g., `cargo` changes its JSON schema. The conformance harness (capture real outputs, compare filtered results against expected) catches regressions.
2. **Some tools don't have clean outputs.** Interactive tools, animated progress bars, tools that use ANSI for layout. For these, `generic.rules` strips ANSI and common stopwords as a best-effort fallback.
3. **Sometimes the agent wants raw output** (debugging, unusual failures). The `--raw` flag and the `--profile full` profile always provide an escape hatch.
4. **Writing rule files is real work.** Each high-impact tool is ~50-150 lines. But it is paid once per tool and benefits every future session forever.

## How to add a new tool

1. Create `tools/rules/<toolname>.rules` following the schema above.
2. Add any new symbols to `tools/tools-dictionary.md` with English definitions.
3. Capture 3-5 real outputs (success, warning, error) and put them in a `tools/fixtures/<toolname>/` directory.
4. Run the conformance check (`gibber-run --selftest <toolname>`) to confirm the rule extracts every load-bearing fact.
5. Done. The agent can now run `gibber-run <toolname> [args]` and get compressed structured results.
