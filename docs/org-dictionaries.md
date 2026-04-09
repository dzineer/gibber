# Organization-Defined Dictionaries (Phase 1 — Dictionary as Data)

**Status:** Design document + Phase 1 implementation.

**Author:** Frank (dzineer). The idea of organization-authored dictionaries
that define not just new symbols but new *rules* — what an org's agents
are and are not allowed to express — was originated by Frank in a design
conversation on 2026-04-09. The specification in this document is a
collaborative distillation of that conversation. The architectural idea
is Frank's.

**Scope of this document:** Phase 1 only. Phase 2 (grammar rules and
executable constraints) is specified separately in
`docs/org-dictionaries-rules.md`.

## The thesis

Until now, Gibber dictionaries have been prose markdown files. They
describe symbols, not constrain them. Any parser reading a dictionary
is trusting a markdown file to tell the truth about what symbols mean,
and any rule about what's allowed lives in the head of whoever reviews
the agent's output.

Phase 1 changes that. Dictionaries become **structured data** — Gibber
forms themselves — and a parser can load, validate, and compose them
mechanically. This is the foundation that Phase 2 (executable rules)
builds on top of.

The thesis in one line: **a dictionary should be a Gibber form, not a
markdown file pretending to be one.**

Once dictionaries are data, organizations can define their own, extend
the base `meta/v1` vocabulary, and eventually (in Phase 2) attach
executable constraints that shape what their agents are allowed to
say. The universe of expressible actions becomes an organizational
decision, not a protocol-level one.

## Why this matters

Four reasons, each one a real unlock:

1. **The grammar becomes a guardrail at organizational scale.** Today's
 `meta/v1` has to serve every project, so it can't exclude anything
 aggressively. An organization-defined dictionary can. A bank ships a
 dictionary with no symbol for "send funds to arbitrary recipient."
 A healthcare org ships one with no symbol for "export PII to
 unverified endpoint." The bad action is not forbidden — it is
 inexpressible.

2. **Compliance review becomes a static data problem.** Today,
 compliance reviewers watch model behavior. Tomorrow, compliance
 reviewers review the dictionary and its rules. Did we allow
 `§send-email`? Does the grammar force a verified recipient? Static
 questions about a file, not dynamic questions about model behavior.
 The same kind of review a lawyer does on a contract.

3. **Dictionaries become a portable trust boundary.** An external agent
 loads your dictionary, and from that moment forward the parser will
 only accept forms that fit your grammar. Not because we trust the
 agent — because the parser won't accept anything else. This is the
 TLS certificate pattern applied to agent communication.

4. **Dictionaries become composable.** Organizations inherit from
 `meta/v1`. Industries publish shared dictionaries. Departments extend
 the org dictionary. Projects extend the department dictionary. A real
 hierarchy that matches how real policy is authored.

## The data model

A dictionary is a Gibber form with the top-level symbol `§dictionary`.
Every entry in a dictionary is itself a Gibber form describing a single
symbol in the language the dictionary defines.

### Top-level shape

```
(§dictionary
 §id:"meta/v1"
 §version:"1.0.0"
 §description:"Base vocabulary for all Gibber working files"
 §extends:[]                  ; dictionaries this one inherits from (empty for base)
 §entries:[
   (§symbol §name:§task
     §kind:§top-level-form
     §summary:"A unit of work with id, status, goal."
     §fields:[
       (§field §name:§id §type:§string §required:true)
       (§field §name:§status §type:(§enum §wip §done §queued) §required:true)
       (§field §name:§title §type:§string §required:false)
       ;; ... etc
     ])

   (§symbol §name:§wip
     §kind:§value
     §of-type:§status
     §summary:"Work in progress.")

   ;; ... more entries
 ])
```

### Entry kinds

Every `§symbol` entry has a `§kind` that tells the parser what kind of
thing the symbol is. Phase 1 defines five kinds; Phase 2 may add more.

| `§kind` | Meaning |
|---|---|
| `§top-level-form` | A symbol that can appear as the head of a top-level Gibber form (e.g. `§task`, `§index`, `§completed`). |
| `§nested-form` | A symbol that is the head of a form nested inside another form but cannot appear at the top level (e.g. `§field`, `§claim`). |
| `§field` | A named child slot on a form. Written with a trailing colon (`§id:T042`) in real use. |
| `§value` | A bare symbol that is a valid value for a typed field (e.g. `§wip`, `§done` for a status field). |
| `§verb` | A symbol used as a verb in narrative blocks, e.g. inside `§did` lists (`§wrote`, `§ran`, `§fixed`). |

The kind determines where in a Gibber form a symbol is allowed to
appear. A Phase 1 validator will reject a form that places a `§value`
symbol in a `§top-level-form` position.

### Field types

A `§field` entry has a `§type` that defines what values the field
accepts. Phase 1 defines a small fixed set of types; Phase 2 extends
this with rule-based constraints.

| Type | Meaning |
|---|---|
| `§string` | A double-quoted string literal. |
| `§int` | A whole number. |
| `§float` | A decimal number. |
| `§bool` | `§true` or `§false`. |
| `§symbol` | Any defined symbol in the dictionary. |
| `§list` | A square-bracket list. Further constrained with `§of-type`. |
| `(§enum §a §b §c)` | One of the listed bare symbols. |
| `(§ref §other-form)` | A nested form of the named kind. |
| `§any` | Unchecked. Phase 1 fallback for fields whose type isn't yet specified. |

### Inheritance

A dictionary declares the dictionaries it extends via `§extends`. The
loader resolves inheritance transitively, then merges the entries. If
two dictionaries define the same symbol name, the deriving dictionary
wins (later in the inheritance chain overrides earlier). Phase 2 will
add rules for *when* overriding is allowed — today, Phase 1 allows it
unconditionally but logs a warning.

```
(§dictionary
 §id:"acme-finance/v1"
 §version:"0.3.0"
 §extends:["meta/v1"]
 §entries:[
   ;; New symbols specific to Acme Finance
   (§symbol §name:§wire-transfer
     §kind:§top-level-form
     §summary:"A wire transfer request."
     §fields:[
       (§field §name:§amount §type:§int §required:true)
       (§field §name:§recipient §type:(§ref §verified-contact) §required:true)
       (§field §name:§currency §type:(§enum §usd §eur §gbp) §required:true)])

   (§symbol §name:§verified-contact
     §kind:§nested-form
     §summary:"A contact that has passed KYC verification."
     §fields:[
       (§field §name:§id §type:§string §required:true)
       (§field §name:§kyc-verified-at §type:§string §required:true)])
 ])
```

## The fast-path version check, extended

Today, every Gibber project declares a version string like:

```
gibber/3 dict-meta/v1 rules/v3 tools/v1
```

Phase 1 extends this to allow multiple dictionaries:

```
gibber/3 dict-meta/v1+acme-finance/v1 rules/v3 tools/v1
```

The `+` separator lists the inheritance chain, base first. A loader
seeing this version string knows to:

1. Load `meta/v1` from `~/.claude/shared/gibber/dictionaries/meta-v1.gibber`
2. Load `acme-finance/v1` from the project-local
 `gibber-dict-acme-finance.gibber`
3. Merge them with the project dictionary winning on conflicts
4. Cache the merged result under a compound key

Phase 1 does not change the existing single-dictionary format — old
projects keep working. Phase 1 only *adds* the ability to declare
multiple dictionaries and the loader machinery to merge them.

## File layout

Phase 1 introduces a new file extension for structured dictionaries:
`.dict.gibber`. The existing `GIBBER-DICTIONARY.md` prose format is
not deleted — it's kept as the human-readable reference, and a
one-time migration tool generates a structured version from it.

```
gibber/
├── dictionaries/
│ ├── meta-v1.dict.gibber ; new: structured meta dictionary
│ └── meta-v1.human.md ; auto-generated from the above
├── GIBBER-DICTIONARY.md ; legacy prose reference (kept)
└── tools/
 ├── dict-validate.py ; new: Phase 1 loader + validator
 ├── dict-migrate.py ; new: one-shot prose-to-structured migration
 └── dict-merge.py ; new: inheritance resolver
```

Organization dictionaries live in the organization's own repo or file
system, named like `acme-finance.dict.gibber`. The loader finds them
via a lookup path declared in the project's CLAUDE.md, similar to how
`gibber-dict-<project>.md` is declared today.

## Phase 1 deliverables

This is what ships as Phase 1:

1. **The spec in this document** — the data model, entry kinds, field
 types, inheritance model, file layout, version-string extension.

2. **A structured `meta-v1.dict.gibber` file** — the existing
 `meta/v2` dictionary migrated from prose to the structured format
 defined here. Every symbol from the original becomes a `§symbol`
 entry with the correct `§kind` and (where applicable) `§fields`.

3. **`dict-validate.py`** — a small Python loader/validator with:
 - `load(path)` — parse a `.dict.gibber` file into an in-memory
 dictionary object
 - `validate(dict)` — check the dictionary's own structure for
 correctness (every entry is a valid `§symbol`, every field has a
 known `§type`, no duplicate symbol names)
 - `resolve(dict, base)` — apply inheritance, returning a merged
 dictionary
 - `lookup(dict, symbol)` — find a symbol definition by name

4. **Example tests** — a tests directory with:
 - A load-and-validate test that loads the migrated `meta-v1.dict.gibber`
 - An inheritance test using a toy `example-org.dict.gibber`
 that extends `meta/v1` with a handful of new symbols
 - A rejection test showing that a malformed dictionary (duplicate
 name, unknown kind, missing required field) fails validation
 with a clear error message

## What Phase 1 does NOT include (explicit deferrals)

These are all deferred to Phase 2 and are specified in
`docs/org-dictionaries-rules.md`:

- **Executable rules and constraints** — Phase 1 has field types but
 no rule language. A Phase 1 dictionary can say *"this field is an
 int"* but cannot say *"this amount must not exceed $10,000 when the
 recipient is unverified."* That's Phase 2.

- **Audit tooling** — the CLI that walks a dictionary and proves
 properties about it ("no reachable form can send funds to an
 unverified recipient") is Phase 2 territory because it depends on
 the rule language.

- **Cross-language parsers** — Phase 1 ships only a Python
 loader/validator because Python is what the existing `scripts/`
 directory uses and the goal is to prove the data model round-trips
 on real dictionaries, not to ship an SDK. Rust and TypeScript
 parsers come later.

- **Runtime enforcement during Gibber parsing** — Phase 1 validates
 dictionaries themselves, not Gibber forms against those dictionaries.
 Form-against-dictionary validation is Phase 2.

- **Dictionary inheritance conflict policy** — Phase 1 allows later
 dictionaries to override earlier ones unconditionally with a warning.
 Phase 2 will add explicit rules about when overriding is allowed
 (some fields may be marked `§frozen` so derivative dictionaries
 can't weaken them).

- **Marketplace / distribution** — Phase 3 or later.

## The honest caveat

Phase 1 is a foundation, not a finished product. On its own, a
structured dictionary is only modestly more useful than the prose
version — you can lint for typos, you can write one cross-language
loader instead of a grep for every symbol, and you can resolve
inheritance mechanically. That's real value, but it is not the value
that makes the idea important.

**The value that makes the idea important is Phase 2** — executable
rules that turn the dictionary into an actual constraint on what forms
are valid. Phase 1 ships because Phase 2 can't ship first. The rule
language in Phase 2 has to reference entries in the data model, and
the data model has to exist before the rule language can exist.

If you stop reading here, the one-line summary is: **Phase 1 turns
dictionaries into data so Phase 2 can turn that data into rules.**

## Next steps after Phase 1 lands

1. Review the Phase 1 spec, the migrated `meta-v1.dict.gibber`, and the
 loader tests
2. Read `docs/org-dictionaries-rules.md` — the Phase 2 design doc
3. If the Phase 2 design is acceptable, pick a scope for the first
 Phase 2 slice (probably `§require` + `§forbid` + `§enum` + `§type` +
 `§max`, in that order of importance)
4. Ship Phase 2 slice 1 with the same care this Phase 1 shipment got
5. Iterate on the rule language with real org dictionaries as the
 forcing function — not toy examples
