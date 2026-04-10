use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A complete .gibber file: optional YAML frontmatter + one root value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GibberFile {
    pub frontmatter: BTreeMap<String, String>,
    pub root: GibberValue,
}

/// Any value in the Gibber S-expression grammar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GibberValue {
    /// A symbol starting with `§`, e.g. `§task`, `§wip`
    Symbol(String),
    /// A bare identifier, e.g. `T042`, `crates/foo.rs`
    Ident(String),
    /// A number with optional unit suffix, e.g. `2d`, `15ms`, `100`, `0.95`
    Number(f64, Option<String>),
    /// A quoted string, e.g. `"hello world"`
    Str(String),
    /// An ordered list: `[a b c]`
    List(Vec<GibberValue>),
    /// A named form: `(§head child1 child2 ...)`
    Form(GibberForm),
    /// A field binding: `§key:value` or `§key<value` etc.
    Field(GibberField),
}

/// A named S-expression form: `(§head children...)`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GibberForm {
    pub head: String,
    pub children: Vec<GibberValue>,
}

/// A field binding: `§key:value`, `§key<value`, `§key=value`, etc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GibberField {
    pub key: String,
    pub op: FieldOp,
    pub value: Box<GibberValue>,
}

/// The operator in a field binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldOp {
    Colon,
    Lt,
    Gt,
    Eq,
    Lte,
    Gte,
}

impl std::fmt::Display for FieldOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldOp::Colon => write!(f, ":"),
            FieldOp::Lt => write!(f, "<"),
            FieldOp::Gt => write!(f, ">"),
            FieldOp::Eq => write!(f, "="),
            FieldOp::Lte => write!(f, "<="),
            FieldOp::Gte => write!(f, ">="),
        }
    }
}

impl GibberValue {
    /// Returns the symbol name if this is a Symbol, None otherwise.
    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            GibberValue::Symbol(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the string if this is a Str, None otherwise.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            GibberValue::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the form if this is a Form, None otherwise.
    pub fn as_form(&self) -> Option<&GibberForm> {
        match self {
            GibberValue::Form(f) => Some(f),
            _ => None,
        }
    }

    /// Returns the list if this is a List, None otherwise.
    pub fn as_list(&self) -> Option<&[GibberValue]> {
        match self {
            GibberValue::List(l) => Some(l),
            _ => None,
        }
    }
}

impl GibberForm {
    /// Find a field by key name within this form's children.
    pub fn field(&self, key: &str) -> Option<&GibberField> {
        self.children.iter().find_map(|child| match child {
            GibberValue::Field(f) if f.key == key => Some(f),
            _ => None,
        })
    }

    /// Get the value of a field by key name.
    pub fn field_value(&self, key: &str) -> Option<&GibberValue> {
        self.field(key).map(|f| f.value.as_ref())
    }
}
