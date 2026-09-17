// MODE: DEV
// PACKAGE: PROD

//! Check 4: every backticked lower_snake identifier or `name()` function
//! reference in a tracked document must resolve against the script-text
//! corpus. AR-134: the printed summary count reflects ONLY the name()-style
//! sub-check (funcs_seen), even though the backticked-identifier sub-check
//! runs in the same group and its own failures still affect the overall
//! exit code.

use crate::mermaid::Severity;
use regex::Regex;
use std::collections::BTreeSet;

pub struct IdentifierOutcome {
    pub funcs_seen: u32,
    pub items: Vec<(Severity, String)>,
}

fn matches_any_line(re: &Regex, text: &str) -> bool {
    text.lines().any(|line| re.is_match(line))
}

/// docs: (repo-relative doc path, raw document text), fixed order.
pub fn check_identifiers(docs: &[(String, String)], script_text: &str) -> IdentifierOutcome {
    let func_call_re = Regex::new(r"[a-z][a-z0-9_]*\(\)").unwrap();
    let backtick_re = Regex::new(r"`[a-z][a-z0-9_]*[^`]*`").unwrap();
    let leading_ident_re = Regex::new(r"^`([a-z][a-z0-9_]*)").unwrap();

    let mut funcs_seen = 0u32;
    let mut items = Vec::new();

    for (doc, text) in docs {
        let mut fn_names: BTreeSet<String> = BTreeSet::new();
        for m in func_call_re.find_iter(text) {
            let raw = m.as_str();
            fn_names.insert(raw[..raw.len() - 2].to_string());
        }
        for fname in &fn_names {
            funcs_seen += 1;
            let def_re = Regex::new(&format!(
                r"(?m)^\s*(function\s+)?{}\(\)",
                regex::escape(fname)
            ))
            .unwrap();
            if !matches_any_line(&def_re, script_text) {
                items.push((
                    Severity::Fail,
                    format!("{doc}: names {fname}() but no script defines it"),
                ));
            }
        }

        let mut ids: BTreeSet<String> = BTreeSet::new();
        for m in backtick_re.find_iter(text) {
            if let Some(caps) = leading_ident_re.captures(m.as_str()) {
                let ident = caps.get(1).unwrap().as_str();
                if ident.contains('_') {
                    ids.insert(ident.to_string());
                }
            }
        }
        for id in &ids {
            let mention_re = Regex::new(&format!(
                r"(?m)([^A-Za-z0-9_]|^){}([^A-Za-z0-9_]|$)",
                regex::escape(id)
            ))
            .unwrap();
            if !matches_any_line(&mention_re, script_text) {
                items.push((
                    Severity::Fail,
                    format!("{doc}: names identifier {id} but no script mentions it"),
                ));
            }
        }
    }
    IdentifierOutcome { funcs_seen, items }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_defined_function_passes() {
        let docs = vec![("doc.md".to_string(), "calls `do_thing()` here".to_string())];
        let script_text = "do_thing() {\n  true\n}\n";
        let outcome = check_identifiers(&docs, script_text);
        assert_eq!(outcome.funcs_seen, 1);
        assert!(outcome.items.is_empty());
    }

    #[test]
    fn an_undefined_function_fails() {
        let docs = vec![(
            "doc.md".to_string(),
            "calls `missing_fn()` here".to_string(),
        )];
        let outcome = check_identifiers(&docs, "");
        assert_eq!(outcome.funcs_seen, 1);
        assert_eq!(outcome.items[0].0, Severity::Fail);
    }

    #[test]
    fn a_function_keyword_definition_is_recognised() {
        let docs = vec![("doc.md".to_string(), "calls `do_thing()` here".to_string())];
        let script_text = "function do_thing() {\n  true\n}\n";
        let outcome = check_identifiers(&docs, script_text);
        assert!(outcome.items.is_empty());
    }

    #[test]
    fn a_backticked_identifier_without_underscore_is_ignored() {
        let docs = vec![("doc.md".to_string(), "a `plain` mention".to_string())];
        let outcome = check_identifiers(&docs, "");
        assert!(outcome.items.is_empty());
        assert_eq!(outcome.funcs_seen, 0);
    }

    #[test]
    fn a_mentioned_underscore_identifier_passes() {
        let docs = vec![("doc.md".to_string(), "the `some_var` field".to_string())];
        let script_text = "echo $some_var\n";
        let outcome = check_identifiers(&docs, script_text);
        assert!(outcome.items.is_empty());
    }

    #[test]
    fn an_unmentioned_underscore_identifier_fails_but_does_not_affect_funcs_seen() {
        let docs = vec![("doc.md".to_string(), "the `missing_var` field".to_string())];
        let outcome = check_identifiers(&docs, "");
        assert_eq!(outcome.funcs_seen, 0);
        assert_eq!(outcome.items.len(), 1);
    }
}
