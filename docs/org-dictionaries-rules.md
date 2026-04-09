# Org Dictionaries — Phase 2 (Grammar Rules as Data)

**Status:** Design document. Not yet implemented. Depends on Phase 1
being complete (see `docs/org-dictionaries.md`).

**Author:** Frank (dzineer) — original idea. Collaborative design
distillation on 2026-04-09.

**Scope:** This document specifies what the Gibber rule language
*should* look like, compares it to prior art, lists the open questions
that can't be answered without prototyping, and proposes an
implementation plan broken into slices.

## What Phase 2 is for

Phase 1 made dictionaries into data. A Phase 1 dictionary can say:

> *"`§wire-transfer` has a field `§amount` of type `§int`."*

A Phase 1 dictionary cannot say:

> *"A `§wire-transfer` with `§amount` greater than 10000 is forbidden
> unless it also carries a `§dual-approval` field."*

Phase 2 adds that capability. Rules become first-class data inside
dictionaries and the parser becomes a policy engine that rejects forms
that violate them.

The thesis in one line: **Phase 2 is where the grammar actually becomes
a guardrail, not just a schema.**

## What a rule is

A rule is a Gibber form with a specific head (`§rule`) attached to a
dictionary entry. Each rule declares:

- **Subject:** which symbol(s) the rule applies to
- **Condition:** when the rule fires (always, or only in specific
 contexts)
- **Constraint:** what must be true when the condition fires

Rules are evaluated against a parsed Gibber form — not against source
text. The policy engine walks the form's AST and, for each node, checks
all applicable rules from the loaded dictionary. A rule violation
produces a structured `§violation` error that points at the specific
form and the specific rule.

## The five minimum rule types

Phase 2 ships five rule types as its first slice. These five cover
roughly 80% of the policies a real organization needs to express, and
the remaining 20% can be added incrementally. The five are, in order
of importance:

### 1. `§require` — a field must be present

```
(§rule §subject:§wire-transfer
 §kind:§require
 §field:§recipient
 §reason:"Every wire transfer must name a recipient.")
```

A form of the subject kind that lacks the named field is rejected.
This is the most basic rule type and the one that unlocks compliance
review, because "this field must always be present" is the most common
policy there is.

### 2. `§forbid` — a field must NOT be present

```
(§rule §subject:§wire-transfer
 §kind:§forbid
 §field:§raw-iban
 §reason:"Raw IBANs must be resolved to verified-contact before submission.")
```

Pairs with `§require` to cover both halves of "what's allowed."
Together they define the closed universe of fields that can appear on
a form.

### 3. `§enum` — a field's value must be one of a fixed set

```
(§rule §subject:§wire-transfer
 §kind:§enum
 §field:§currency
 §allowed:[§usd §eur §gbp]
 §reason:"Only supported currencies.")
```

Phase 1 already has `(§enum ...)` as a type constructor. Phase 2's
`§enum` rule is the same concept but explicit — it lets the reason be
attached and lets the allowed set be overridden or extended by
derivative dictionaries.

### 4. `§type` — a field's value must match a runtime type predicate

```
(§rule §subject:§wire-transfer
 §kind:§type
 §field:§amount
 §predicate:§positive-int
 §reason:"Amounts must be positive whole numbers.")
```

Phase 1 already has field types like `§int` and `§string`. The Phase 2
`§type` rule goes further: it can reference named predicates that are
themselves defined in the dictionary, such as `§positive-int`,
`§iso8601-date`, `§verified-contact-id`. The catalog of predicates is
itself extensible per dictionary.

### 5. `§max` and `§min` — numeric bounds

```
(§rule §subject:§wire-transfer
 §kind:§max
 §field:§amount
 §value:10000
 §unless:(§has-field §dual-approval)
 §reason:"Transfers over $10,000 require dual approval.")
```

The `§max`/`§min` rules are the first rule types that demonstrate
*conditional* constraints. The `§unless` clause is a small predicate
language that looks at the same form being validated and decides
whether the rule fires. This is where the design gets delicate — see
the open questions section below.

## What a rule is NOT

Three things rules are deliberately not, to keep the design reviewable:

1. **Rules are not code.** A rule is data. It is declarative. The
 policy engine evaluates rules against forms using a fixed, small,
 reviewable interpreter. No rule is allowed to execute arbitrary
 code, call external services, or depend on non-deterministic
 input. This is how rules remain auditable.

2. **Rules are not Turing-complete.** The predicate language used in
 `§unless` and `§when` clauses is intentionally restricted. No
 recursion. No unbounded loops. No user-defined functions. A rule
 evaluation always terminates in O(form size) and produces a
 deterministic yes/no result.

3. **Rules are not global.** A rule belongs to a specific dictionary
 entry. When that entry is overridden by a derivative dictionary,
 its rules are also subject to override (with restrictions — see
 "frozen fields" below).

## Prior art — what each tradition got right and what to avoid

### XML Schema (XSD)

**What it got right:** Typed structure, required/optional fields,
enumerated values, namespace composition. The basic declarative field
constraints are a solved problem and Phase 2's first four rule types
are intentionally very XSD-shaped.

**What to avoid:** XSD's extensibility mechanism (schema-derivation
via `extension` and `restriction`) is famously hard to reason about.
Inheritance chains produce schemas that technically validate but are
nearly impossible to review by hand. Phase 2's inheritance model has
to be simpler than this — ideally flat enough that a reviewer can
read a resolved dictionary and see the full picture without walking a
tree of overrides.

### JSON Schema

**What it got right:** Machine-readable, declarative, composable via
`$ref` and `allOf`. The JSON Schema ecosystem proved that validation
tools and audit tools can share a spec. Phase 2's rules should be
similarly composable.

**What to avoid:** JSON Schema's evolution from draft-04 to draft-2020
introduced subtle semantic changes that broke tooling compatibility.
Phase 2's rule language has to lock in its semantics early and use a
version number on the rule spec itself (`rules/v1`, `rules/v2`) so
that old dictionaries keep working when the language evolves.

### OWL / RDFS

**What it got right:** Rigorous semantics, formal reasoning, the
ability to prove properties about a knowledge graph using automated
reasoners. Phase 2's audit tool will eventually want to answer
questions like "can any valid form of kind X ever produce a side
effect of kind Y?" and that's the territory OWL occupies.

**What to avoid:** OWL's full expressiveness is undecidable. OWL-DL
(description logic) is decidable but requires a PhD to use correctly.
Phase 2 must stay closer to "first-order predicates over a fixed AST"
than to "full description logic." The rule language should be
*provably tractable* rather than *maximally expressive*.

### OPA / Rego

**What it got right:** Policy-as-code that's genuinely readable by
engineers. Rego's declarative Datalog-like syntax for expressing
security policies is the closest existing thing to what Phase 2 wants
to be. Rego rules are small, composable, and auditable.

**What to avoid:** Rego is Turing-complete in practice (via recursion
and arbitrary data loading). It's also a whole separate language
users have to learn. Phase 2's rule language should be *expressed in
Gibber itself* — rules written as `§rule` forms, reviewable with the
same tools that review any other Gibber form, loaded by the same
parser.

### CEL (Common Expression Language)

**What it got right:** A small, safe, declarative expression language
for policy and config. CEL is intentionally non-Turing-complete,
has clearly defined semantics, and is embeddable in multiple host
languages. The CEL model is the closest single match to what Phase 2's
predicate sub-language should look like.

**What to avoid:** CEL is separate from the host data format (you
write CEL expressions as strings inside JSON or YAML). Phase 2 should
not go this way — rules should be native Gibber forms, not embedded
expressions inside a string. The whole point is that rules are
auditable the same way forms are.

## The predicate sub-language

The `§when`, `§unless`, `§and`, `§or`, `§not` predicates are the most
interesting design problem in Phase 2. Here is a proposed minimum
vocabulary:

### Accessors

- `(§field §name:§amount)` — the value of the named field on the
 current form
- `(§has-field §amount)` — true if the form has the named field
- `(§kind)` — the kind of the current form
- `(§count §field:§recipients)` — the count of elements if the field
 is a list

### Comparisons

- `(§eq <a> <b>)` — equality
- `(§neq <a> <b>)` — inequality
- `(§lt <a> <b>)`, `(§lte <a> <b>)`, `(§gt <a> <b>)`, `(§gte <a> <b>)`
 — numeric comparisons
- `(§in <value> [<allowed> ...])` — membership in a literal set
- `(§matches <value> <regex>)` — regex match on a string field

### Logical composition

- `(§and <pred> <pred> ...)` — all predicates must be true
- `(§or <pred> <pred> ...)` — at least one predicate must be true
- `(§not <pred>)` — negation

### Example

```
(§rule §subject:§wire-transfer
 §kind:§max
 §field:§amount
 §value:10000
 §unless:(§and
   (§has-field §dual-approval)
   (§eq (§field §name:§dual-approval §subfield:§status) §approved))
 §reason:"Transfers over $10,000 require an approved dual-approval field.")
```

The unless clause reads: *"unless the form has a dual-approval field
and that field's status is approved."* A human reviewer can read this
and understand it without running any code. A policy engine can
evaluate it deterministically in O(form size).

## Frozen fields

A critical feature for real org dictionaries: some fields can be
marked `§frozen:true` in a dictionary entry, which means derivative
dictionaries cannot weaken or remove them. An org might say:

```
(§symbol §name:§wire-transfer §kind:§top-level-form
 §fields:[
   (§field §name:§amount §type:§int §required:true §frozen:true)
   (§field §name:§recipient §type:(§ref §verified-contact) §required:true §frozen:true)
 ])

(§rule §subject:§wire-transfer
 §kind:§max §field:§amount §value:10000
 §frozen:true
 §reason:"Hard cap at org level; no team can override this.")
```

A department-level dictionary that tries to raise the cap to 20000
would be rejected at dictionary-load time with a clear error: *"rule
at acme-finance/v1:§wire-transfer:§max is frozen; derivative
dictionaries cannot override it."*

This is the mechanism that lets a compliance team set org-wide rules
that individual teams cannot bypass. It is the single most important
feature for making the dictionary-as-policy story work in practice.

## The audit tool

Once rules exist, the audit tool becomes tractable. Phase 2 will ship
`gibber-dict-audit` (working name) that answers questions like:

- **"Can any valid form of kind X produce side effect Y?"** — walks
 all rules applicable to X and checks whether any combination of
 values can satisfy them while also producing Y.
- **"Does this dictionary forbid X?"** — returns yes/no with the
 rule that implements the forbid, or "no rule found."
- **"What changed between two versions of a dictionary?"** — diffs
 the rules and reports additions, removals, strengthening, and
 weakening (the last one requires frozen-field logic to be
 meaningful).

The audit tool is the deliverable that a compliance team actually
uses. It's what turns Phase 2 from "structured data" into "compliance
infrastructure." Shipping rules without the audit tool would miss the
point — the rules only matter if you can ask questions about them.

## Open questions — things we can't answer without prototyping

These are questions I cannot answer in a design document because
they depend on what real org dictionaries actually need. They must be
answered during Phase 2 implementation through prototyping and real
test cases.

### Q1. How expressive does the predicate sub-language need to be?

The minimum vocabulary above is probably enough for 80% of rules. But
some real policies might need:

- **Quantifiers** — *"every recipient in the recipients list must be
 verified"* requires a `§forall` or `§exists` over a list field.
 Tractable but adds complexity.
- **Joins across fields** — *"if field A is X, then field B must be
 Y"* is already covered by `§when`, but more complex joins might
 need something like `§let` bindings for readability.
- **Temporal predicates** — *"this rule applies only during business
 hours"* — probably out of scope. Rules should be pure functions of
 the form, not of external state. Temporal rules belong in a
 separate runtime layer, not in the dictionary.

**Answer strategy:** prototype with the minimum vocabulary, try to
write rules for five real scenarios (see below), and only add
quantifiers or joins when a real rule can't be expressed without
them.

### Q2. How do rules compose across inherited dictionaries?

Three possible semantics:

- **Conjunction** (all rules apply): strictest, safest default, but
 makes it impossible for a derivative dictionary to relax a rule.
- **Override** (later rule wins): most flexible, but defeats the
 point of org-level policy — teams can just override their way out.
- **Conjunction with frozen**: default is conjunction, but fields or
 rules marked `§frozen:true` cannot be overridden at all.

**Answer strategy:** ship conjunction-with-frozen as the default. This
matches the compliance model most orgs actually want (strict by
default, explicit override only where the org allows it). If a real
org dictionary needs override semantics, we can add an opt-in
`§override-allowed:true` flag on individual rules.

### Q3. What's the error message format?

Rule violations need structured errors that callers can act on.
Proposed shape:

```
(§violation
 §rule-id:"acme-finance/v1:wire-transfer:max:amount"
 §subject:§wire-transfer
 §field:§amount
 §actual:15000
 §constraint:(§max §value:10000)
 §reason:"Transfers over $10,000 require dual approval."
 §location:(§form §id:"wt-2026-001" §line:42))
```

Each violation is itself a Gibber form so downstream tools can walk
violations the same way they walk any other Gibber data. Multiple
violations per form are collected into a list rather than aborting on
the first.

### Q4. What about rules that need to look *outside* the form?

Example: *"the recipient must exist in the organization's
verified-contacts table."* This needs a lookup into an external data
source, which breaks the "rules are pure functions of the form" rule.

**Answer strategy:** treat external lookups as a separate layer.
Phase 2 rules operate on the form alone. External-lookup rules are a
Phase 3 feature, implemented via a pluggable `§resolver` trait that a
host application provides. The policy engine calls the resolver to
answer questions like "does this contact id exist?" and the resolver
is responsible for providing ground-truth answers. This cleanly
separates "grammar rules" (pure, Phase 2) from "data rules"
(resolver-backed, Phase 3) without polluting the core language.

### Q5. How do we handle rule conflicts within a single dictionary?

If a dictionary has two rules on the same field, one requiring the
field and another forbidding it, the dictionary is internally
inconsistent. The question is: do we catch this at dictionary-load
time or only at form-validation time?

**Answer strategy:** catch it at dictionary-load time by running a
static consistency check. The check walks all rules for each field
and looks for direct contradictions (`§require` + `§forbid`,
overlapping `§min`/`§max` bounds, disjoint `§enum` sets when combined
with inheritance). If any inconsistency is found, the dictionary
fails to load with a specific error naming the two conflicting rules.

This is solvable for the five rule types in the minimum vocabulary
but gets harder if quantifiers are added. Another reason to resist
quantifiers unless they're absolutely needed.

## Five real scenarios that should drive rule-language design

The design above is only useful if it can express real policies. The
Phase 2 implementation plan requires writing rules for all five of
these scenarios as acceptance criteria before the implementation is
considered done.

### Scenario 1 — Finance org

- Wire transfers under $10,000 are allowed with just a recipient.
- Transfers from $10,000 to $100,000 require dual approval.
- Transfers over $100,000 require an additional compliance-officer
 sign-off.
- Recipients must be from a verified-contact list.
- No wire transfers to recipients outside an allow-listed set of
 currencies.

### Scenario 2 — Healthcare org

- Patient records can only be read by agents with a `§role:§clinician`
 or `§role:§admin` claim.
- PII fields (SSN, DOB, full name) cannot appear in outbound messages.
- Any form that references a patient id must carry a matching
 `§consent-verified` field.
- Deletion of a record requires a `§retention-exception` justification.

### Scenario 3 — Research lab

- Every `§claim` in a research-log form must have an `§evidence`
 field.
- Every `§evidence` field must cite at least one source (by URL or
 document id).
- Claims about causation must also carry a `§method` field stating
 the methodology used.
- Claims with `§confidence:§low` must be marked `§preliminary:true`.

### Scenario 4 — Customer support org

- Responses to customers cannot include internal tooling symbols
 (`§debug-log`, `§internal-note`, etc.).
- Refund requests over $500 require a supervisor approval field.
- Any response referencing a legal topic must include a disclaimer
 field.
- Responses must carry a `§ticket-id` matching the request they are
 replying to.

### Scenario 5 — Dev tools / code review org

- Code review comments cannot approve changes to frozen files
 (a list of paths the org designates as requiring extra review).
- Approvals cannot come from the same agent that authored the change.
- Comments on security-sensitive files must cite the specific
 security-review checklist item.
- No agent can approve a merge without at least one passing
 integration test reference.

If the Phase 2 rule language cannot express all five scenarios
cleanly, the language is not ready to ship. These scenarios are the
acceptance criteria.

## Proposed implementation plan

Phase 2 is not one change — it's a sequence of small, testable
slices. Each slice ships independently, with tests, before the next
one starts.

### Slice 2.1 — Rule data model (no evaluation yet)

Add `§rule` entries to the dictionary data model. Extend the Phase 1
loader to parse `§rule` forms and attach them to the relevant
`SymbolDef`. No evaluation yet — just loading and static structural
validation.

**Deliverable:** a dictionary can declare rules, the loader reads
them, and a `SymbolDef` exposes a `rules` attribute.

### Slice 2.2 — The first five rule types

Implement evaluation for `§require`, `§forbid`, `§enum`, `§type`,
`§max`/`§min`. No predicate sub-language yet — rules are
unconditional (fire on every form of the subject kind).

**Deliverable:** `validate_form(form, dict)` returns a list of
`§violation` objects for any rule failures.

### Slice 2.3 — The predicate sub-language

Add `§when`, `§unless`, and the minimum predicate vocabulary
(`§field`, `§has-field`, `§eq`/`§neq`/`§lt`/`§lte`/`§gt`/`§gte`,
`§in`, `§and`/`§or`/`§not`). Rules become conditional.

**Deliverable:** the Finance Scenario 1 can be fully expressed as a
Phase 2 dictionary.

### Slice 2.4 — Frozen fields

Add `§frozen:true` support on fields and rules. Derivative
dictionaries that attempt to override frozen entries fail to load
with a clear error.

**Deliverable:** a department dictionary inheriting from a frozen
org dictionary cannot weaken the org's rules.

### Slice 2.5 — Audit tool (static dictionary queries)

Ship `gibber-dict-audit` with three subcommands:
- `check <dict>` — runs the consistency check (Q5) on a loaded
 dictionary
- `explain <dict> <symbol>` — shows all rules that apply to a symbol
- `diff <dict-a> <dict-b>` — diffs two versions of a dictionary and
 reports rule additions/removals/changes

**Deliverable:** the audit tool can answer "what does this dictionary
allow?" by static inspection.

### Slice 2.6 — Acceptance tests on all five scenarios

Write dictionaries for all five scenarios from the previous section.
Each dictionary must load cleanly, pass consistency checks, and
correctly accept/reject a set of example forms that exercise each
rule. If any scenario can't be expressed, add a sixth slice with
whatever language extension is minimally required — and document
that decision as a learning.

**Deliverable:** Phase 2 ships when all five scenarios are
expressible.

## What Phase 2 is NOT going to do

Explicit non-goals so scope doesn't drift:

- **No runtime policy engine integration with host applications.**
 Phase 2 is a library and CLI. Integrating it into an actual agent
 runtime (so rule violations block tool calls in real time) is a
 downstream consumer problem, not Phase 2's problem.
- **No UI for authoring rules.** Rules are written as Gibber forms in
 text editors. A visual rule editor is a future project.
- **No import of rules from other policy languages (OPA, XSD, etc.).**
 If someone wants to move from OPA to Gibber rules, they can write
 the rules by hand or build a translator as a separate project.
- **No performance work beyond "it terminates in O(form size)."**
 Phase 2's evaluator should be correct and simple. Optimizing it is
 a later concern if profiling shows a problem.
- **No external-data rules (resolvers).** Deferred to Phase 3 per Q4.

## The honest timeline

Phase 2 is a multi-week project. Here's the honest breakdown:

- **Slice 2.1 (rule data model):** 2–3 days
- **Slice 2.2 (unconditional rules):** 3–4 days
- **Slice 2.3 (predicate sub-language):** 5–7 days — this is the
 hard slice
- **Slice 2.4 (frozen fields):** 2 days
- **Slice 2.5 (audit tool):** 4–5 days
- **Slice 2.6 (scenario acceptance tests):** 3–4 days

**Total: roughly 3 weeks of focused work.** Longer if any of the
scenarios reveal a gap in the predicate language and slice 2.3 has
to be revisited. This is the reason I pushed back against shipping
Phase 2 implementation in a single session — the slice 2.3 work alone
needs more sustained attention than any single session provides, and
rushing it produces a language that can only express toy policies.

## Next steps after this document lands

1. Review this design doc. Challenge the tradeoffs — especially the
 predicate language scope and the frozen-field semantics.
2. Write acceptance criteria for slice 2.1 (rule data model only,
 no evaluation).
3. Start slice 2.1 in a new branch, with the rule data model as pure
 data extension to Phase 1. No evaluation logic yet. Ship that
 first and review it before touching the evaluator.
4. Proceed slice by slice. After each slice, reassess whether the
 design needs updating based on what was learned during
 implementation.

The five scenarios above are the forcing function. If at any point
during implementation one of the five becomes impossible to express,
stop and update the design before continuing to code.

## The one-line summary

**Phase 1 made dictionaries into data. Phase 2 makes rules into data.
Both phases together turn a Gibber dictionary into a reviewable
organizational policy that agents physically cannot violate — not
because they're watched, but because the grammar rejects anything
outside the policy before it can execute.**
