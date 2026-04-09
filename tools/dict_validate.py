#!/usr/bin/env python3
"""
Phase 1 loader and validator for structured Gibber dictionaries.

A `.dict.gibber` file is a Gibber form with the top-level head
`§dictionary`. This module provides:

    load(path)       - parse a .dict.gibber file into an in-memory
                       dictionary object
    validate(dict)   - check the dictionary's own structure is
                       correct (every entry is a valid §symbol, every
                       field has a known type, no duplicate names)
    resolve(dict, base_dict) - apply inheritance between two loaded
                               dictionaries
    lookup(dict, symbol) - find a symbol definition by name

The parser in this file is a minimal Gibber S-expression reader. It
does NOT validate Gibber-against-dictionary (that's Phase 2) — it only
validates the dictionary file itself.

The goal of Phase 1 is to prove the data model round-trips on a real
dictionary (meta/v1) and to provide a foundation that Phase 2 can
build executable rules on top of.

Usage (as a library):

    from tools.dict_validate import load, validate, resolve, lookup

    d = load('dictionaries/meta-v1.dict.gibber')
    errors = validate(d)
    if errors:
        for e in errors:
            print(f"ERROR: {e}")
        sys.exit(1)

    org = load('acme-finance.dict.gibber')
    merged = resolve(org, base=d)
    sym = lookup(merged, 'wire-transfer')
    print(sym.fields)

Usage (as a CLI):

    python3 tools/dict_validate.py <path.dict.gibber>
"""
from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Optional


# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------


VALID_KINDS = {"top-level-form", "nested-form", "field", "value", "verb"}

SCALAR_TYPES = {"string", "int", "float", "bool", "symbol", "any"}


@dataclass
class FieldDef:
    """A named field on a Gibber form."""
    name: str
    type: Any   # may be a str (scalar), a list (enum), or a dict (ref/list)
    required: bool = False


@dataclass
class SymbolDef:
    """A single §symbol entry in a dictionary."""
    name: str
    kind: str
    summary: str = ""
    fields: list[FieldDef] = field(default_factory=list)
    of_type: Optional[str] = None


@dataclass
class Dictionary:
    """
    A loaded dictionary.

    Note on `entries`: Gibber intentionally reuses the same symbol
    name in multiple positions. `§completed`, for example, is both a
    top-level form (the completed-tasks list) AND a field (on
    §index and §task records). To represent this faithfully, entries
    are keyed by (name, kind) tuples rather than by name alone. The
    `by_name` helper returns all entries with a given name; callers
    that only care about one kind can filter further.
    """
    id: str
    version: str
    description: str = ""
    extends: list[str] = field(default_factory=list)
    migration_note: str = ""
    entries: dict[tuple[str, str], SymbolDef] = field(default_factory=dict)
    source_path: Optional[str] = None

    def by_name(self, name: str) -> list[SymbolDef]:
        """Return all entries with the given symbol name (across kinds)."""
        if name.startswith("§"):
            name = name[1:]
        return [s for (n, _k), s in self.entries.items() if n == name]


# ---------------------------------------------------------------------------
# S-expression parser
# ---------------------------------------------------------------------------


class ParseError(Exception):
    """Raised when a .dict.gibber file can't be parsed."""


def _strip_frontmatter(src: str) -> str:
    """Remove optional YAML frontmatter delimited by --- ... ---."""
    m = re.match(r"^---\s*\n(.*?)\n---\s*\n(.*)$", src, re.DOTALL)
    if m:
        return m.group(2)
    return src


def _tokenize(src: str) -> list[tuple[str, str]]:
    """
    Tokenize a Gibber form into (kind, value) pairs.

    Kinds:
      LPAREN, RPAREN, LBRACKET, RBRACKET
      SYMBOL   - §name
      KEY      - §name:
      STRING   - "..."
      NUMBER   - 42 or 3.14
      ATOM     - bare identifier (no § prefix)
      COMMENT  - ;; to end of line (dropped, but tracked)
    """
    tokens: list[tuple[str, str]] = []
    i = 0
    n = len(src)
    while i < n:
        c = src[i]

        # Whitespace
        if c.isspace():
            i += 1
            continue

        # Line comment
        if c == ";" and i + 1 < n and src[i + 1] == ";":
            while i < n and src[i] != "\n":
                i += 1
            continue

        # Parens / brackets
        if c == "(":
            tokens.append(("LPAREN", "("))
            i += 1
            continue
        if c == ")":
            tokens.append(("RPAREN", ")"))
            i += 1
            continue
        if c == "[":
            tokens.append(("LBRACKET", "["))
            i += 1
            continue
        if c == "]":
            tokens.append(("RBRACKET", "]"))
            i += 1
            continue

        # String literal
        if c == '"':
            j = i + 1
            buf = []
            while j < n and src[j] != '"':
                if src[j] == "\\" and j + 1 < n:
                    buf.append(src[j + 1])
                    j += 2
                else:
                    buf.append(src[j])
                    j += 1
            if j >= n:
                raise ParseError(f"Unterminated string starting at offset {i}")
            tokens.append(("STRING", "".join(buf)))
            i = j + 1
            continue

        # §symbol or §key:
        if c == "§":
            j = i + 1
            while j < n and (src[j].isalnum() or src[j] in "_-/.?!<>=+*"):
                j += 1
            name = src[i + 1 : j]
            if not name:
                raise ParseError(f"Empty symbol at offset {i}")
            if j < n and src[j] == ":":
                tokens.append(("KEY", name))
                i = j + 1
            else:
                tokens.append(("SYMBOL", name))
                i = j
            continue

        # Number
        if c.isdigit() or (c == "-" and i + 1 < n and src[i + 1].isdigit()):
            j = i + 1
            while j < n and (src[j].isdigit() or src[j] == "."):
                j += 1
            tokens.append(("NUMBER", src[i:j]))
            i = j
            continue

        # Bare atom (e.g., `true`, `false`, or an unquoted identifier)
        if c.isalpha() or c == "_":
            j = i + 1
            while j < n and (src[j].isalnum() or src[j] in "_-/."):
                j += 1
            tokens.append(("ATOM", src[i:j]))
            i = j
            continue

        raise ParseError(f"Unexpected character {c!r} at offset {i}")

    return tokens


@dataclass
class Form:
    """A parsed S-expression form."""
    head: Optional[str]  # the leading §symbol, or None for list literals
    args: list[Any]      # positional args (forms or scalar values)
    kwargs: dict[str, Any]  # §key:value pairs


def _parse_value(tokens: list[tuple[str, str]], i: int) -> tuple[Any, int]:
    """Parse a single value starting at tokens[i]. Returns (value, new_i)."""
    if i >= len(tokens):
        raise ParseError("Unexpected end of input")
    kind, val = tokens[i]

    if kind == "LPAREN":
        return _parse_form(tokens, i)

    if kind == "LBRACKET":
        items: list[Any] = []
        i += 1
        while i < len(tokens) and tokens[i][0] != "RBRACKET":
            v, i = _parse_value(tokens, i)
            items.append(v)
        if i >= len(tokens):
            raise ParseError("Unterminated list literal")
        return items, i + 1  # skip RBRACKET

    if kind == "STRING":
        return val, i + 1

    if kind == "NUMBER":
        if "." in val:
            return float(val), i + 1
        return int(val), i + 1

    if kind == "SYMBOL":
        return {"__symbol__": val}, i + 1

    if kind == "ATOM":
        return val, i + 1

    raise ParseError(f"Unexpected token {kind}:{val!r} at position {i}")


def _parse_form(tokens: list[tuple[str, str]], i: int) -> tuple[Form, int]:
    """Parse a (§head ...) form starting at LPAREN at tokens[i]."""
    if tokens[i][0] != "LPAREN":
        raise ParseError(f"Expected LPAREN at position {i}, got {tokens[i]}")
    i += 1
    head: Optional[str] = None
    args: list[Any] = []
    kwargs: dict[str, Any] = {}

    # First token should be the head symbol
    if i < len(tokens) and tokens[i][0] == "SYMBOL":
        head = tokens[i][1]
        i += 1
    else:
        raise ParseError(f"Form must start with a §symbol head, got {tokens[i] if i < len(tokens) else 'EOF'}")

    while i < len(tokens) and tokens[i][0] != "RPAREN":
        tk, tv = tokens[i]
        if tk == "KEY":
            # §key:value
            i += 1
            val, i = _parse_value(tokens, i)
            kwargs[tv] = val
        else:
            val, i = _parse_value(tokens, i)
            args.append(val)

    if i >= len(tokens):
        raise ParseError(f"Unterminated form (§{head} ...)")
    return Form(head=head, args=args, kwargs=kwargs), i + 1  # skip RPAREN


def parse(src: str) -> Form:
    """Parse a full .dict.gibber source into the top-level form."""
    src = _strip_frontmatter(src)
    tokens = _tokenize(src)
    if not tokens:
        raise ParseError("Empty dictionary file")
    form, i = _parse_form(tokens, 0)
    # Allow trailing whitespace / comments but no additional forms
    if i < len(tokens):
        raise ParseError(f"Unexpected tokens after top-level form at position {i}: {tokens[i]}")
    return form


# ---------------------------------------------------------------------------
# From parsed Form -> Dictionary data model
# ---------------------------------------------------------------------------


def _sym_name(v: Any) -> str:
    """Extract a symbol name from a parsed value."""
    if isinstance(v, dict) and "__symbol__" in v:
        return v["__symbol__"]
    if isinstance(v, str):
        return v
    raise ParseError(f"Expected a symbol, got {v!r}")


def _coerce_field(f: Form) -> FieldDef:
    """Turn a parsed (§field ...) form into a FieldDef."""
    if f.head != "field":
        raise ParseError(f"Expected (§field ...), got (§{f.head} ...)")
    name = _sym_name(f.kwargs.get("name"))
    type_val = f.kwargs.get("type")
    required = f.kwargs.get("required", False)
    if isinstance(required, dict) and "__symbol__" in required:
        # §true / §false
        required = required["__symbol__"] == "true"
    elif isinstance(required, bool):
        pass
    else:
        required = bool(required)
    return FieldDef(name=name, type=type_val, required=required)


def _coerce_symbol(f: Form) -> SymbolDef:
    """Turn a parsed (§symbol ...) form into a SymbolDef."""
    if f.head != "symbol":
        raise ParseError(f"Expected (§symbol ...), got (§{f.head} ...)")
    name = _sym_name(f.kwargs.get("name"))
    kind = _sym_name(f.kwargs.get("kind"))
    summary = f.kwargs.get("summary", "") or ""
    of_type = None
    if "of-type" in f.kwargs:
        of_type = _sym_name(f.kwargs["of-type"])
    fields: list[FieldDef] = []
    if "fields" in f.kwargs:
        raw = f.kwargs["fields"]
        if not isinstance(raw, list):
            raise ParseError(f"§fields must be a list, got {type(raw).__name__}")
        for item in raw:
            if not isinstance(item, Form):
                raise ParseError(f"§fields entries must be (§field ...) forms")
            fields.append(_coerce_field(item))
    return SymbolDef(
        name=name,
        kind=kind,
        summary=summary,
        fields=fields,
        of_type=of_type,
    )


def _coerce_dictionary(f: Form, source_path: Optional[str] = None) -> Dictionary:
    """Turn a parsed (§dictionary ...) form into a Dictionary."""
    if f.head != "dictionary":
        raise ParseError(f"Top-level form must be (§dictionary ...), got (§{f.head} ...)")
    did = f.kwargs.get("id")
    version = f.kwargs.get("version", "")
    description = f.kwargs.get("description", "")
    migration_note = f.kwargs.get("migration-note", "")
    extends_raw = f.kwargs.get("extends", [])
    if not isinstance(extends_raw, list):
        raise ParseError("§extends must be a list of strings")
    extends = [str(x) for x in extends_raw]
    entries_raw = f.kwargs.get("entries", [])
    if not isinstance(entries_raw, list):
        raise ParseError("§entries must be a list")
    entries: dict[tuple[str, str], SymbolDef] = {}
    for item in entries_raw:
        if not isinstance(item, Form):
            raise ParseError("§entries must contain (§symbol ...) forms")
        sym = _coerce_symbol(item)
        key = (sym.name, sym.kind)
        if key in entries:
            raise ParseError(
                f"duplicate symbol: §{sym.name} with kind §{sym.kind}"
            )
        entries[key] = sym
    return Dictionary(
        id=str(did),
        version=str(version),
        description=str(description),
        extends=extends,
        migration_note=str(migration_note),
        entries=entries,
        source_path=source_path,
    )


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def load(path: str | Path) -> Dictionary:
    """Load a .dict.gibber file into a Dictionary."""
    path = Path(path)
    src = path.read_text(encoding="utf-8")
    form = parse(src)
    return _coerce_dictionary(form, source_path=str(path))


def validate(d: Dictionary) -> list[str]:
    """
    Check the dictionary's own structure for correctness.
    Returns a list of error messages; empty list means valid.
    """
    errors: list[str] = []
    if not d.id:
        errors.append("dictionary missing §id")
    if not d.version:
        errors.append(f"dictionary {d.id!r} missing §version")
    if not d.entries:
        errors.append(f"dictionary {d.id!r} has no entries")

    for (name, kind), sym in d.entries.items():
        where = f"{d.id}:§{name}[{kind}]"
        if sym.kind not in VALID_KINDS:
            errors.append(
                f"{where}: unknown kind §{sym.kind} (valid: {sorted(VALID_KINDS)})"
            )
        if not sym.summary and sym.kind != "value":
            # §value entries may be self-explanatory; other kinds should have a summary
            errors.append(f"{where}: missing §summary")
        for fd in sym.fields:
            ferrors = _validate_field_type(fd.type, where + f":§{fd.name}")
            errors.extend(ferrors)
    return errors


def _validate_field_type(t: Any, where: str) -> list[str]:
    """Check a field's type is one the Phase 1 type system understands."""
    errors: list[str] = []
    if isinstance(t, dict) and "__symbol__" in t:
        name = t["__symbol__"]
        if name not in SCALAR_TYPES and name not in {"list", "enum", "ref"}:
            # Could be a user-defined symbol used as a type-by-reference
            pass
        return errors
    if isinstance(t, Form):
        if t.head == "enum":
            if not t.args:
                errors.append(f"{where}: (§enum ...) must have at least one value")
        elif t.head == "list":
            if "of-type" not in t.kwargs:
                errors.append(f"{where}: (§list ...) must have §of-type")
        elif t.head == "ref":
            if not t.args or not isinstance(t.args[0], dict):
                errors.append(f"{where}: (§ref ...) must name a symbol")
        else:
            errors.append(f"{where}: unknown compound type (§{t.head} ...)")
        return errors
    if t is None:
        errors.append(f"{where}: missing §type")
        return errors
    # A bare string is not a valid type in Phase 1
    errors.append(f"{where}: type must be a symbol or compound form, got {type(t).__name__}")
    return errors


def resolve(derived: Dictionary, base: Dictionary) -> Dictionary:
    """
    Merge a derived dictionary on top of a base dictionary.
    Entries defined in `derived` override entries in `base` with the
    same (name, kind) key. A warning is printed to stderr on conflicts.
    """
    merged = Dictionary(
        id=f"{base.id}+{derived.id}",
        version=f"{base.version}+{derived.version}",
        description=(
            derived.description
            or base.description
            or f"Merged dictionary: {base.id} + {derived.id}"
        ),
        extends=base.extends + [base.id],
        entries=dict(base.entries),
        source_path=None,
    )
    for key, sym in derived.entries.items():
        if key in merged.entries:
            name, kind = key
            print(
                f"warning: {derived.id} overrides §{name}[{kind}] from {base.id}",
                file=sys.stderr,
            )
        merged.entries[key] = sym
    return merged


def lookup(
    d: Dictionary,
    symbol: str,
    kind: Optional[str] = None,
) -> Optional[SymbolDef]:
    """
    Find a symbol by name in a dictionary.

    If `kind` is given, returns the specific (name, kind) entry or
    None. If `kind` is not given, prefers top-level-form, then
    nested-form, then field, then value, then verb — the first one
    that exists. Returns None if no entry with that name exists.
    """
    # Allow callers to pass either `task` or `§task`.
    if symbol.startswith("§"):
        symbol = symbol[1:]
    if kind is not None:
        return d.entries.get((symbol, kind))
    # No kind specified — try the standard preference order.
    for k in ("top-level-form", "nested-form", "field", "value", "verb"):
        sym = d.entries.get((symbol, k))
        if sym is not None:
            return sym
    return None


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def _cli(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: dict_validate.py <dictionary.dict.gibber> [base.dict.gibber]", file=sys.stderr)
        return 2

    try:
        d = load(argv[1])
    except ParseError as e:
        print(f"parse error in {argv[1]}: {e}", file=sys.stderr)
        return 1
    except FileNotFoundError:
        print(f"not found: {argv[1]}", file=sys.stderr)
        return 1

    errors = validate(d)
    if errors:
        print(f"FAIL: {len(errors)} validation error(s) in {d.id}", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1

    print(f"OK: {d.id} v{d.version} — {len(d.entries)} entries")

    if len(argv) >= 3:
        base = load(argv[2])
        base_errors = validate(base)
        if base_errors:
            print(f"base dictionary {base.id} has errors:", file=sys.stderr)
            for e in base_errors:
                print(f"  - {e}", file=sys.stderr)
            return 1
        merged = resolve(d, base=base)
        overrides = sum(1 for k in d.entries if k in base.entries)
        new = sum(1 for k in d.entries if k not in base.entries)
        print(
            f"OK: merged {merged.id} — {len(merged.entries)} entries "
            f"({len(base.entries)} base + {overrides} overrides + {new} new)"
        )

    return 0


if __name__ == "__main__":
    sys.exit(_cli(sys.argv))
