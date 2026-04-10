use crate::ast::*;
use std::path::Path;

/// A validation issue found in a .gibber file.
#[derive(Debug)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl std::fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag = match self.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARN",
        };
        write!(f, "[{}] {}", tag, self.message)
    }
}

/// Validate a .gibber file: parse it, check structure, round-trip it.
pub fn validate_file(path: &Path) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // 1. Read
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                message: format!("cannot read file: {}", e),
            });
            return issues;
        }
    };

    // 2. Parse
    let file = match crate::parse(&content) {
        Ok(f) => f,
        Err(e) => {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                message: format!("parse failed: {}", e),
            });
            return issues;
        }
    };

    // 3. Frontmatter checks
    if file.frontmatter.is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            message: "missing frontmatter (expected id and gibber_dict)".to_string(),
        });
    } else {
        if !file.frontmatter.contains_key("id") {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                message: "frontmatter missing 'id' field".to_string(),
            });
        }
        if !file.frontmatter.contains_key("gibber_dict") {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                message: "frontmatter missing 'gibber_dict' field".to_string(),
            });
        }
    }

    // 4. Root must be a form
    if file.root.as_form().is_none() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "root value is not a form (expected a top-level S-expression)".to_string(),
        });
    }

    // 5. Round-trip check: emit and re-parse, verify ASTs match
    let emitted = crate::emit(&file);
    match crate::parse(&emitted) {
        Ok(reparsed) => {
            if file.root != reparsed.root {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    message: "round-trip failed: emitted output parses to a different AST".to_string(),
                });
            }
        }
        Err(e) => {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                message: format!("round-trip failed: emitted output cannot be re-parsed: {}", e),
            });
        }
    }

    // 6. Walk the AST for structural warnings
    check_value(&file.root, &mut issues);

    issues
}

/// Validate a string of Gibber content (not from a file).
pub fn validate_str(content: &str) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    let file = match crate::parse(content) {
        Ok(f) => f,
        Err(e) => {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                message: format!("parse failed: {}", e),
            });
            return issues;
        }
    };

    if file.root.as_form().is_none() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "root value is not a form".to_string(),
        });
    }

    let emitted = crate::emit(&file);
    match crate::parse(&emitted) {
        Ok(reparsed) => {
            if file.root != reparsed.root {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    message: "round-trip failed".to_string(),
                });
            }
        }
        Err(e) => {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                message: format!("round-trip failed: {}", e),
            });
        }
    }

    issues
}

/// Recursively check a value for structural issues.
fn check_value(val: &GibberValue, issues: &mut Vec<ValidationIssue>) {
    match val {
        GibberValue::Form(form) => {
            if form.head.is_empty() {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    message: "form has empty head symbol".to_string(),
                });
            }
            if form.children.is_empty() {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    message: format!("form '{}' has no children", form.head),
                });
            }
            for child in &form.children {
                check_value(child, issues);
            }
        }
        GibberValue::List(items) => {
            for item in items {
                check_value(item, issues);
            }
        }
        GibberValue::Field(field) => {
            if field.key.is_empty() {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    message: "field has empty key".to_string(),
                });
            }
            check_value(&field.value, issues);
        }
        _ => {}
    }
}
