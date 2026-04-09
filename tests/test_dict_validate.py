#!/usr/bin/env python3
"""
Test suite for the Phase 1 dictionary loader and validator.

Run with:
    python3 tests/test_dict_validate.py

This is a deliberately simple test harness — no pytest required, no
external dependencies. Each test is a function that returns (pass, msg).
The runner at the bottom collects results and prints a summary.

Phase 1 tests cover:
  1. Loading the real migrated meta-v1 dictionary
  2. Validating that meta-v1 passes validation
  3. Loading a toy org dictionary that extends meta-v1
  4. Resolving the org dictionary against meta-v1 and checking the
     merged result contains both base and derived entries
  5. Looking up a symbol by name
  6. Rejecting a duplicate-symbol dictionary at parse time
  7. Rejecting an unknown-kind dictionary at validation time
"""
from __future__ import annotations

import sys
from pathlib import Path

# Add repo root to sys.path so `tools.dict_validate` imports cleanly.
REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO))

from tools.dict_validate import (  # noqa: E402
    load,
    validate,
    resolve,
    lookup,
    ParseError,
)


META_PATH = REPO / "dictionaries" / "meta-v1.dict.gibber"
ORG_PATH = REPO / "tests" / "example-org.dict.gibber"
BAD_DUP_PATH = REPO / "tests" / "bad-duplicate.dict.gibber"
BAD_KIND_PATH = REPO / "tests" / "bad-kind.dict.gibber"


# ---------------------------------------------------------------------------
# Individual tests
# ---------------------------------------------------------------------------


def test_load_meta_v1() -> tuple[bool, str]:
    d = load(META_PATH)
    if d.id != "meta/v1":
        return False, f"expected id 'meta/v1', got {d.id!r}"
    if not d.entries:
        return False, "meta/v1 has no entries"
    return True, f"loaded {len(d.entries)} entries from meta/v1"


def test_validate_meta_v1() -> tuple[bool, str]:
    d = load(META_PATH)
    errors = validate(d)
    if errors:
        return False, f"meta/v1 failed validation: {errors[0]} (plus {len(errors) - 1} more)"
    return True, "meta/v1 passes validation"


def test_meta_v1_has_core_symbols() -> tuple[bool, str]:
    """Spot-check that the migration included the core top-level forms."""
    d = load(META_PATH)
    required = ["task", "index", "completed", "learning", "audit", "verification"]
    missing = [s for s in required if not d.by_name(s)]
    if missing:
        return False, f"meta/v1 is missing core symbols: {missing}"
    # Check that §task has fields
    task = lookup(d, "task")
    if not task or not task.fields:
        return False, "§task has no fields"
    return True, f"all core symbols present, §task has {len(task.fields)} fields"


def test_load_example_org() -> tuple[bool, str]:
    d = load(ORG_PATH)
    if d.id != "example-org/v1":
        return False, f"expected id 'example-org/v1', got {d.id!r}"
    if not d.by_name("wire-transfer"):
        return False, "example-org is missing §wire-transfer"
    if "meta/v1" not in d.extends:
        return False, f"example-org should extend meta/v1, extends={d.extends}"
    return True, f"loaded example-org with {len(d.entries)} entries"


def test_resolve_inheritance() -> tuple[bool, str]:
    base = load(META_PATH)
    derived = load(ORG_PATH)
    merged = resolve(derived, base)
    # Merged should contain all base entries
    if not merged.by_name("task"):
        return False, "merged dict is missing §task from base"
    # Merged should contain derived entries
    if not merged.by_name("wire-transfer"):
        return False, "merged dict is missing §wire-transfer from derived"
    # Merged id should compose
    if "example-org/v1" not in merged.id:
        return False, f"merged id should contain derived id, got {merged.id!r}"
    return True, (
        f"merged has {len(merged.entries)} entries "
        f"(base {len(base.entries)} + derived new {len(derived.entries)})"
    )


def test_lookup() -> tuple[bool, str]:
    d = load(META_PATH)
    sym = lookup(d, "task")
    if sym is None:
        return False, "lookup('task') returned None"
    if sym.kind != "top-level-form":
        return False, f"§task should be top-level-form, got {sym.kind}"
    # Lookup with § prefix
    sym2 = lookup(d, "§task")
    if sym2 is None or sym2.name != "task":
        return False, "lookup('§task') did not find the same symbol"
    # Lookup of missing symbol
    missing = lookup(d, "nonexistent-symbol-xyz")
    if missing is not None:
        return False, "lookup of missing symbol should return None"
    return True, "lookup found §task, handled § prefix, returned None for missing"


def test_reject_duplicate_symbol() -> tuple[bool, str]:
    try:
        load(BAD_DUP_PATH)
    except ParseError as e:
        if "duplicate" in str(e).lower():
            return True, f"correctly rejected: {e}"
        return False, f"parse failed but not with 'duplicate' message: {e}"
    return False, "parser accepted a dictionary with duplicate symbols"


def test_reject_unknown_kind() -> tuple[bool, str]:
    d = load(BAD_KIND_PATH)
    errors = validate(d)
    if not errors:
        return False, "validator accepted a dictionary with unknown §kind"
    if not any("unknown kind" in e for e in errors):
        return False, f"validator reported errors but not about unknown kind: {errors}"
    return True, f"correctly rejected: {errors[0]}"


def test_field_types_are_preserved() -> tuple[bool, str]:
    """Check that the example-org §wire-transfer §amount field loaded as §int."""
    d = load(ORG_PATH)
    wt = lookup(d, "wire-transfer")
    if wt is None:
        return False, "§wire-transfer not found"
    amount = next((f for f in wt.fields if f.name == "amount"), None)
    if amount is None:
        return False, "§amount field not found on §wire-transfer"
    # Amount type should be the symbol `int`
    if not isinstance(amount.type, dict) or amount.type.get("__symbol__") != "int":
        return False, f"§amount type should be §int, got {amount.type!r}"
    if not amount.required:
        return False, "§amount should be required"
    return True, "§wire-transfer.§amount is §int, required"


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------


TESTS = [
    ("load meta/v1", test_load_meta_v1),
    ("validate meta/v1", test_validate_meta_v1),
    ("meta/v1 core symbols present", test_meta_v1_has_core_symbols),
    ("load example-org", test_load_example_org),
    ("resolve inheritance", test_resolve_inheritance),
    ("lookup by name", test_lookup),
    ("reject duplicate symbol", test_reject_duplicate_symbol),
    ("reject unknown kind", test_reject_unknown_kind),
    ("field types preserved", test_field_types_are_preserved),
]


def main() -> int:
    passed = 0
    failed = 0
    print(f"Running {len(TESTS)} tests...\n")
    for name, fn in TESTS:
        try:
            ok, msg = fn()
        except Exception as e:  # noqa: BLE001
            ok, msg = False, f"raised {type(e).__name__}: {e}"
        marker = "PASS" if ok else "FAIL"
        print(f"  [{marker}] {name} — {msg}")
        if ok:
            passed += 1
        else:
            failed += 1
    print()
    print(f"{passed} passed, {failed} failed")
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
