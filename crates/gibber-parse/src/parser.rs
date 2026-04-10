use nom::branch::alt;
use nom::bytes::complete::{tag, take_until, take_while, take_while1};
use nom::character::complete::{char, multispace1, satisfy};
use nom::combinator::{opt, peek, recognize, value};
use nom::multi::many0;
use nom::sequence::terminated;
use nom::{IResult, Parser};
use std::collections::BTreeMap;

use crate::ast::*;

// ---------------------------------------------------------------------------
// Whitespace and comments
// ---------------------------------------------------------------------------

fn ws(input: &str) -> IResult<&str, ()> {
    value(
        (),
        many0(alt((
            value((), multispace1),
            value((), line_comment),
        ))),
    )
    .parse(input)
}

fn line_comment(input: &str) -> IResult<&str, &str> {
    let (input, _) = char(';').parse(input)?;
    let (input, comment) = take_while(|c| c != '\n').parse(input)?;
    let (input, _) = opt(char('\n')).parse(input)?;
    Ok((input, comment))
}

// ---------------------------------------------------------------------------
// Frontmatter (YAML-style between --- delimiters)
// ---------------------------------------------------------------------------

pub fn parse_frontmatter(input: &str) -> IResult<&str, BTreeMap<String, String>> {
    let (input, _) = ws(input)?;

    // Check if there's a frontmatter block
    if !input.starts_with("---") {
        return Ok((input, BTreeMap::new()));
    }

    let (input, _) = tag("---").parse(input)?;
    let (input, _) = take_while(|c| c == ' ' || c == '\t').parse(input)?;
    let (input, _) = opt(char('\n')).parse(input)?;
    let (input, body) = take_until("---").parse(input)?;
    let (input, _) = tag("---").parse(input)?;

    let mut map = BTreeMap::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((key, val)) = line.split_once(':') {
            map.insert(key.trim().to_string(), val.trim().to_string());
        }
    }

    Ok((input, map))
}

// ---------------------------------------------------------------------------
// Atoms
// ---------------------------------------------------------------------------

/// Parse a symbol: `§` followed by ASCII letters (lower or upper), digits, hyphens, or dots.
fn parse_symbol_raw(input: &str) -> IResult<&str, &str> {
    recognize((
        satisfy(|c| c == '\u{00A7}'),
        take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_'),
    ))
    .parse(input)
}

fn parse_symbol(input: &str) -> IResult<&str, GibberValue> {
    let (input, sym) = parse_symbol_raw(input)?;
    // Strip the leading § (which is 2 bytes in UTF-8: 0xC2 0xA7)
    let name = &sym[sym.char_indices().nth(1).map(|(i, _)| i).unwrap_or(sym.len())..];
    Ok((input, GibberValue::Symbol(name.to_string())))
}

/// Parse a quoted symbol: `§"..."` — a symbol whose name is a quoted string.
/// This handles patterns like `§"hippocampus for AI"` appearing as list items.
fn parse_symbol_quoted(input: &str) -> IResult<&str, GibberValue> {
    // Must start with §
    let (input, _) = satisfy(|c| c == '\u{00A7}').parse(input)?;
    // Followed immediately by a quoted string
    let (input, val) = parse_string(input)?;
    if let GibberValue::Str(s) = val {
        Ok((input, GibberValue::Symbol(s)))
    } else {
        // Should not happen since parse_string always returns Str
        Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Satisfy)))
    }
}

/// Parse a quoted string: `"..."`
fn parse_string(input: &str) -> IResult<&str, GibberValue> {
    let (input, _) = char('"').parse(input)?;
    let mut result = String::new();
    let mut remaining = input;
    loop {
        match remaining.chars().next() {
            None => return Err(nom::Err::Error(nom::error::Error::new(remaining, nom::error::ErrorKind::Char))),
            Some('"') => {
                remaining = &remaining[1..];
                break;
            }
            Some('\\') => {
                remaining = &remaining[1..];
                match remaining.chars().next() {
                    Some('n') => { result.push('\n'); remaining = &remaining[1..]; }
                    Some('t') => { result.push('\t'); remaining = &remaining[1..]; }
                    Some('\\') => { result.push('\\'); remaining = &remaining[1..]; }
                    Some('"') => { result.push('"'); remaining = &remaining[1..]; }
                    Some(c) => { result.push(c); remaining = &remaining[c.len_utf8()..]; }
                    None => return Err(nom::Err::Error(nom::error::Error::new(remaining, nom::error::ErrorKind::Char))),
                }
            }
            Some(c) => {
                result.push(c);
                remaining = &remaining[c.len_utf8()..];
            }
        }
    }
    Ok((remaining, GibberValue::Str(result)))
}

/// Returns true if the input looks like a date (`NNNN-NN`) or timestamp (`NNNN-NNT`).
/// Used to prevent `parse_number` from consuming the leading digits of a date.
fn looks_like_date(input: &str) -> bool {
    // Pattern: 4 digits, hyphen, 2 digits (e.g. 2026-04...)
    let bytes = input.as_bytes();
    bytes.len() >= 7
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'-'
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
}

/// Parse a number with optional unit suffix: `42`, `2.5`, `2d`, `15ms`, `100%`
/// Does NOT match date-like patterns such as `2026-04-11`.
fn parse_number(input: &str) -> IResult<&str, GibberValue> {
    // Reject dates before trying to parse as number
    let check_input = if input.starts_with('-') { &input[1..] } else { input };
    if looks_like_date(check_input) {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Satisfy,
        )));
    }

    let (input, num_str) = recognize((
        opt(alt((char('-'), char('+')))),
        take_while1(|c: char| c.is_ascii_digit()),
        opt((char('.'), take_while1(|c: char| c.is_ascii_digit()))),
    ))
    .parse(input)?;

    let num: f64 = num_str.parse().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Float))
    })?;

    // Optional unit suffix (includes hyphens for compound units like `66.9-percent`)
    let (input, unit) = opt(take_while1(|c: char| {
        c.is_ascii_lowercase() || c == '%' || c == '-'
    }))
    .parse(input)?;

    Ok((input, GibberValue::Number(num, unit.map(|s: &str| s.to_string()))))
}

/// Parse a bare identifier: alphanumeric, hyphens, underscores, dots, slashes.
/// Also handles date-like tokens (e.g. `2026-04-11`) and ISO timestamps
/// (e.g. `2026-04-11T01:00:00Z`) that begin with digits.
fn parse_ident(input: &str) -> IResult<&str, GibberValue> {
    // Allow digit-starting idents only if they look like a date/timestamp
    let starts_with_digit = input.chars().next().map_or(false, |c| c.is_ascii_digit());
    if starts_with_digit {
        if !looks_like_date(input) {
            // Digit-starting but not a date — reject so parse_number can handle it
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Satisfy,
            )));
        }
        // Date/timestamp: consume alphanumerics, hyphens, colons, '+', and 'T'/'Z' suffix chars
        // The '+' is needed for RFC3339 timezone offsets like 2026-04-10T19:37:32+00:00
        let (input, id) = take_while1(|c: char| {
            c.is_ascii_alphanumeric() || c == '-' || c == ':' || c == '.' || c == '+'
        })
        .parse(input)?;
        return Ok((input, GibberValue::Ident(id.to_string())));
    }

    // Normal ident: must start with alphabetic, '_', '/', '~', or '.'
    let (_, _first) = peek(satisfy(|c: char| {
        c.is_ascii_alphabetic() || c == '_' || c == '/' || c == '~' || c == '.'
    }))
    .parse(input)?;

    let (input, id) = take_while1(|c: char| {
        c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/' || c == '~'
    })
    .parse(input)?;

    Ok((input, GibberValue::Ident(id.to_string())))
}

// ---------------------------------------------------------------------------
// Field operator
// ---------------------------------------------------------------------------

fn parse_field_op(input: &str) -> IResult<&str, FieldOp> {
    alt((
        value(FieldOp::Lte, tag("<=")),
        value(FieldOp::Gte, tag(">=")),
        // Compound colon-prefix operators: `:<`, `:>`, `:<=`, `:>=`
        // These appear in data files as shorthand for comparison fields, e.g. `§latency:<1ms`
        value(FieldOp::Lte, tag(":<=")),
        value(FieldOp::Gte, tag(":>=")),
        value(FieldOp::Lt, tag(":<")),
        value(FieldOp::Gt, tag(":>")),
        value(FieldOp::Colon, tag(":")),
        value(FieldOp::Lt, tag("<")),
        value(FieldOp::Gt, tag(">")),
        value(FieldOp::Eq, tag("=")),
    ))
    .parse(input)
}

// ---------------------------------------------------------------------------
// Compound structures
// ---------------------------------------------------------------------------

/// Parse a list: `[val val val]`
fn parse_list(input: &str) -> IResult<&str, GibberValue> {
    let (input, _) = char('[').parse(input)?;
    let (input, _) = ws(input)?;
    let (input, items) = many0(terminated(parse_value, ws)).parse(input)?;
    let (input, _) = char(']').parse(input)?;
    Ok((input, GibberValue::List(items)))
}

/// Parse a form: `(§head child child ...)`
///
/// The head must be a `§symbol`. If the head symbol is immediately followed by a
/// field operator (e.g. `(§time<1d)`), the entire `§head<op>value` expression is
/// treated as the first child field and the head name is taken from the symbol name.
fn parse_form(input: &str) -> IResult<&str, GibberValue> {
    let (input, _) = char('(').parse(input)?;
    let (input, _) = ws(input)?;
    let (input, head_raw) = parse_symbol_raw(input)?;
    let head_name = &head_raw[head_raw.char_indices().nth(1).map(|(i, _)| i).unwrap_or(head_raw.len())..];

    // If the head symbol is immediately followed by a field operator (no whitespace),
    // parse the whole thing as the first child field (e.g. `§time<1d`).
    let (input, first_child) = match parse_field_op(input) {
        Ok((after_op, op)) => {
            let (after_val, val) = parse_value(after_op)?;
            let field = GibberValue::Field(GibberField {
                key: head_name.to_string(),
                op,
                value: Box::new(val),
            });
            (after_val, Some(field))
        }
        Err(_) => (input, None),
    };

    let (input, _) = ws(input)?;
    let (input, mut children) = many0(terminated(parse_value, ws)).parse(input)?;
    let (input, _) = char(')').parse(input)?;

    if let Some(fc) = first_child {
        children.insert(0, fc);
    }

    Ok((
        input,
        GibberValue::Form(GibberForm {
            head: head_name.to_string(),
            children,
        }),
    ))
}

/// Parse a field: `§key:value`, `§key<value`, etc.
/// A field is a symbol immediately followed by an operator and a value (no space).
fn parse_field(input: &str) -> IResult<&str, GibberValue> {
    let (input, sym) = parse_symbol_raw(input)?;
    let key = &sym[sym.char_indices().nth(1).map(|(i, _)| i).unwrap_or(sym.len())..];
    let (input, op) = parse_field_op(input)?;
    let (input, val) = parse_value(input)?;
    Ok((
        input,
        GibberValue::Field(GibberField {
            key: key.to_string(),
            op,
            value: Box::new(val),
        }),
    ))
}

// ---------------------------------------------------------------------------
// Top-level value parser
// ---------------------------------------------------------------------------

/// Parse any single Gibber value.
pub fn parse_value(input: &str) -> IResult<&str, GibberValue> {
    let (input, _) = ws(input)?;
    alt((
        parse_string,
        parse_form,
        parse_list,
        parse_field,          // must come before parse_symbol (both start with §)
        parse_symbol_quoted,  // §"..." must come before parse_symbol
        parse_symbol,
        parse_number,
        parse_ident,
    ))
    .parse(input)
}

/// Parse a complete .gibber file.
pub fn parse_file(input: &str) -> IResult<&str, GibberFile> {
    let (input, frontmatter) = parse_frontmatter(input)?;
    let (input, _) = ws(input)?;
    let (input, root) = parse_value(input)?;
    let (input, _) = ws(input)?;
    Ok((input, GibberFile { frontmatter, root }))
}

// ---------------------------------------------------------------------------
// Convenience function
// ---------------------------------------------------------------------------

/// Parse a .gibber file from a string. Returns the parsed file or an error message.
pub fn parse(input: &str) -> Result<GibberFile, String> {
    match parse_file(input) {
        Ok(("", file)) => Ok(file),
        Ok((remaining, _)) => Err(format!(
            "Parser did not consume all input. Remaining: {:?}",
            &remaining[..remaining.len().min(100)]
        )),
        Err(e) => Err(format!("Parse error: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_symbol() {
        let (_, val) = parse_value("§wip").unwrap();
        assert_eq!(val, GibberValue::Symbol("wip".to_string()));
    }

    #[test]
    fn test_parse_string() {
        let (_, val) = parse_value(r#""hello world""#).unwrap();
        assert_eq!(val, GibberValue::Str("hello world".to_string()));
    }

    #[test]
    fn test_parse_number() {
        let (_, val) = parse_value("42").unwrap();
        assert_eq!(val, GibberValue::Number(42.0, None));
    }

    #[test]
    fn test_parse_number_with_unit() {
        let (_, val) = parse_value("2d").unwrap();
        assert_eq!(val, GibberValue::Number(2.0, Some("d".to_string())));
    }

    #[test]
    fn test_parse_ident() {
        let (_, val) = parse_value("T042").unwrap();
        assert_eq!(val, GibberValue::Ident("T042".to_string()));
    }

    #[test]
    fn test_parse_list() {
        let (_, val) = parse_value("[§a §b §c]").unwrap();
        assert_eq!(
            val,
            GibberValue::List(vec![
                GibberValue::Symbol("a".to_string()),
                GibberValue::Symbol("b".to_string()),
                GibberValue::Symbol("c".to_string()),
            ])
        );
    }

    #[test]
    fn test_parse_field() {
        let (_, val) = parse_value("§status:§wip").unwrap();
        assert_eq!(
            val,
            GibberValue::Field(GibberField {
                key: "status".to_string(),
                op: FieldOp::Colon,
                value: Box::new(GibberValue::Symbol("wip".to_string())),
            })
        );
    }

    #[test]
    fn test_parse_field_with_lt() {
        let (_, val) = parse_value("§time<2d").unwrap();
        assert_eq!(
            val,
            GibberValue::Field(GibberField {
                key: "time".to_string(),
                op: FieldOp::Lt,
                value: Box::new(GibberValue::Number(2.0, Some("d".to_string()))),
            })
        );
    }

    #[test]
    fn test_parse_form() {
        let input = "(§task §id:T042 §status:§wip)";
        let (_, val) = parse_value(input).unwrap();
        let form = val.as_form().unwrap();
        assert_eq!(form.head, "task");
        assert_eq!(form.children.len(), 2);
    }

    #[test]
    fn test_parse_file_with_frontmatter() {
        let input = r#"---
id: T042
gibber_dict: meta/v2
---

(§task §id:T042 §status:§wip)
"#;
        let file = parse(input).unwrap();
        assert_eq!(file.frontmatter.get("id").unwrap(), "T042");
        assert_eq!(file.frontmatter.get("gibber_dict").unwrap(), "meta/v2");
        let form = file.root.as_form().unwrap();
        assert_eq!(form.head, "task");
    }

    #[test]
    fn test_parse_nested_form() {
        let input = "(§task §goal:(§build §parser §with:[§nom §serde]))";
        let (_, val) = parse_value(input).unwrap();
        let form = val.as_form().unwrap();
        assert_eq!(form.head, "task");
    }

    #[test]
    fn test_parse_comment() {
        let input = r#"(§task
  ; this is a comment
  §id:T001)"#;
        let (_, val) = parse_value(input).unwrap();
        let form = val.as_form().unwrap();
        assert_eq!(form.head, "task");
    }

    #[test]
    fn test_parse_path_ident() {
        let (_, val) = parse_value("crates/gibber-parse/src/lib.rs").unwrap();
        assert_eq!(
            val,
            GibberValue::Ident("crates/gibber-parse/src/lib.rs".to_string())
        );
    }
}

#[cfg(test)]
mod debug_tests {
    use super::*;

    #[test]
    fn debug_t001_segment_by_segment() {
        // Test each child of the T001 task form individually
        let segments = vec![
            r#"§id:T001"#,
            r#"§title:"gibber S-expression lexer and AST types""#,
            r#"§status:§queued"#,
            r#"§owner:§ai"#,
            r#"§depends:[]"#,
            r#"§goal:(§build §lexer §and §ast §for §gibber §sexpr §grammar)"#,
            "§files:[crates/gibber-parse/src/lib.rs\n          crates/gibber-parse/src/lexer.rs\n          crates/gibber-parse/src/ast.rs\n          crates/gibber-parse/Cargo.toml]",
            r#"§done:[]"#,
            r#"§todo:[§define-ast-types §write-lexer §handle-symbols §handle-idents §handle-numbers §handle-strings §handle-operators]"#,
            r#"§notes:"tokens: §sym ident number string op:< > = <= >= punct:( ) [ ] field-sep:: comment:;""#,
            r#"§budget:(§time<1d)"#,
            r#"§tests:[§test-unit]"#,
        ];
        for seg in &segments {
            let result = parse_value(seg);
            assert!(result.is_ok(), "FAILED: {:?}\nError: {:?}", seg, result);
        }
    }

    #[test]
    fn debug_d004_segment_by_segment() {
        // Test patterns that appear in D004.gibber
        let segments = vec![
            r#"§p-value:<0.001"#,
            r#"§p-value:<0.01"#,
            r#"§fast-path:<1ms"#,
            r#"§2-hop-31m-edges:<1ms"#,
            r#"§latency:<1ms"#,
            r#"§win-rate:72-83-percent"#,
            r#"§0.700-judge-score"#,
            r#"§15-percent-plus"#,
            r#"§fb15k-237-mrr:0.338"#,
            r#"§wn18rr-mrr:0.476"#,
            r#"§natural-questions:44.5-em"#,
            r#"§delta:+7.9"#,
            r#"[research-agent]"#,
            r#"§mem0g:68.4-percent"#,
            r#"§mem0g:2.59s"#,
        ];
        for seg in &segments {
            let result = parse_value(seg);
            assert!(result.is_ok(), "FAILED: {:?}\nError: {:?}", seg, result);
        }
    }
}
