// MODE: DEV
// PACKAGE: PROD
//! Shared finding and Markdown primitives for the plan validator.

use std::path::Path;

/// The state accumulated by validator passes. Warnings never change `errors`.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct Findings {
    pub errors: usize,
    pub warnings: usize,
    pub messages: Vec<Message>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Severity {
    Fail,
    Warn,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Message {
    pub severity: Severity,
    pub text: String,
}

impl Findings {
    pub fn fail(&mut self, text: impl Into<String>) {
        let text = text.into();
        eprintln!("FAIL: {text}");
        self.errors += 1;
        self.messages.push(Message {
            severity: Severity::Fail,
            text,
        });
    }

    pub fn warn(&mut self, text: impl Into<String>) {
        let text = text.into();
        eprintln!("WARN: {text}");
        self.warnings += 1;
        self.messages.push(Message {
            severity: Severity::Warn,
            text,
        });
    }
}

/// Match the shell helper: whitespace is removed, then one surrounding
/// backtick on each side is removed.
pub fn trim(value: &str) -> String {
    let value = value.trim_matches(|character: char| character.is_ascii_whitespace());
    value
        .strip_prefix('`')
        .unwrap_or(value)
        .strip_suffix('`')
        .unwrap_or_else(|| value.strip_prefix('`').unwrap_or(value))
        .to_string()
}

pub fn require_heading(file: &Path, heading: &str, findings: &mut Findings) {
    let present = std::fs::read_to_string(file)
        .map(|text| text.lines().any(|line| line == heading))
        .unwrap_or(false);
    if !present {
        findings.fail(format!("Missing '{heading}' in {}", file.display()));
    }
}

/// Extract a single Markdown list field (`- Label: value`). The shell helper
/// treats absent, duplicated, and empty-valued fields as the same refusal.
pub fn get_single_field(file: &Path, label: &str, findings: &mut Findings) -> String {
    let Ok(text) = std::fs::read_to_string(file) else {
        findings.fail(format!(
            "{} must declare exactly one '{label}:' field (found 0)",
            file.display()
        ));
        return String::new();
    };
    let mut values = Vec::new();
    for line in text.lines() {
        let rest = line.trim_start();
        let Some(rest) = rest.strip_prefix('-') else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(value) = rest.strip_prefix(label) else {
            continue;
        };
        let Some(value) = value.strip_prefix(':') else {
            continue;
        };
        let value = value.trim();
        if !value.is_empty() {
            values.push(value);
        }
    }
    if values.len() != 1 {
        findings.fail(format!(
            "{} must declare exactly one '{label}:' field (found {})",
            file.display(),
            values.len()
        ));
        return String::new();
    }
    trim(values[0])
}

#[cfg(test)]
mod tests {
    use super::{trim, Findings, Severity};

    #[test]
    fn trim_matches_markdown_cell_rules() {
        assert_eq!(trim("  `hello`  "), "hello");
        assert_eq!(trim("  hello  "), "hello");
        assert_eq!(trim("`hello"), "hello");
        assert_eq!(trim("hello`"), "hello");
    }

    #[test]
    fn warnings_do_not_become_errors() {
        let mut findings = Findings::default();
        findings.warn("advisory");
        findings.fail("gate");
        assert_eq!(findings.errors, 1);
        assert_eq!(findings.warnings, 1);
        assert_eq!(findings.messages[0].severity, Severity::Warn);
    }
}
