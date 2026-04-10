use crate::ast::*;

/// Emit a GibberFile back to canonical .gibber text.
pub fn emit(file: &GibberFile) -> String {
    let mut out = String::new();

    if !file.frontmatter.is_empty() {
        out.push_str("---\n");
        for (key, val) in &file.frontmatter {
            out.push_str(key);
            out.push_str(": ");
            out.push_str(val);
            out.push('\n');
        }
        out.push_str("---\n\n");
    }

    emit_value(&file.root, &mut out, 0);
    out.push('\n');
    out
}

/// Emit a single value at the given indentation level.
pub fn emit_value(val: &GibberValue, out: &mut String, indent: usize) {
    match val {
        GibberValue::Symbol(s) => {
            out.push('§');
            out.push_str(s);
        }
        GibberValue::Ident(s) => {
            out.push_str(s);
        }
        GibberValue::Number(n, unit) => {
            // Emit integer-looking numbers without decimal point
            if *n == n.floor() && n.abs() < 1e15 {
                out.push_str(&(*n as i64).to_string());
            } else {
                out.push_str(&n.to_string());
            }
            if let Some(u) = unit {
                out.push_str(u);
            }
        }
        GibberValue::Str(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    _ => out.push(c),
                }
            }
            out.push('"');
        }
        GibberValue::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                emit_value(item, out, indent + 1);
            }
            out.push(']');
        }
        GibberValue::Form(form) => {
            out.push('(');
            out.push('§');
            out.push_str(&form.head);
            for child in &form.children {
                out.push('\n');
                for _ in 0..=(indent) {
                    out.push_str("  ");
                }
                emit_value(child, out, indent + 1);
            }
            out.push(')');
        }
        GibberValue::Field(field) => {
            out.push('§');
            out.push_str(&field.key);
            out.push_str(&field.op.to_string());
            emit_value(&field.value, out, indent);
        }
    }
}
