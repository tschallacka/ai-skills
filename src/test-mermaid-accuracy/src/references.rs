// MODE: DEV
// PACKAGE: PROD

//! Checks 2 and 3: every `*.sh` named in a mermaid block must exist or be
//! written by some script; every other named artifact must be a real path or
//! be named by some script, with a documented-but-unreferenced name (AR-128:
//! against corpus 3, the markdown corpus with each tracked document's own
//! path excluded) downgraded to WARN rather than FAIL.

use crate::discovery::{script_writes, tree_has, Corpora};
use crate::mermaid::{Severity, Token};
use std::collections::HashMap;

pub struct ReferenceOutcome {
    pub scripts_seen: u32,
    pub artifacts_seen: u32,
    pub items: Vec<(Severity, String)>,
}

fn contains_on_some_line(text: &str, needle: &str) -> bool {
    text.lines().any(|line| line.contains(needle))
}

/// docs: (repo-relative doc path, that doc's own emitted tokens), fixed order.
///
/// AR-133: deduplicated by name ACROSS ALL DOCUMENTS COMBINED -- one row per
/// unique name total. This nix flake's own `sort -u -k1,1` (uutils
/// coreutils, verified directly: reordering the input flips which location
/// survives) keeps the FIRST occurrence in input order for tied keys, not a
/// lexicographic "smallest doc:line string" tiebreak -- so the surviving
/// location is the first one encountered walking the tracked documents in
/// their fixed order, top to bottom within each.
pub fn check_references(docs: &[(String, Vec<Token>)], corpora: &Corpora) -> ReferenceOutcome {
    let mut best: HashMap<String, String> = HashMap::new();
    for (doc, tokens) in docs {
        for tok in tokens {
            let candidate = format!("{doc}:{}", tok.line);
            best.entry(tok.text.clone()).or_insert(candidate);
        }
    }
    let mut names: Vec<&String> = best.keys().collect();
    names.sort();

    let mut scripts_seen = 0u32;
    let mut artifacts_seen = 0u32;
    let mut items = Vec::new();
    for name in names {
        let where_ = &best[name];
        if name.ends_with(".sh") {
            scripts_seen += 1;
            if tree_has(&corpora.tree, name) || script_writes(&corpora.script_text, name) {
                continue;
            }
            items.push((
                Severity::Fail,
                format!(
                    "{where_}: diagram names script {name} \u{2014} no such file, and no script writes it"
                ),
            ));
        } else {
            artifacts_seen += 1;
            if tree_has(&corpora.tree, name) {
                continue;
            }
            if contains_on_some_line(&corpora.script_text, name) {
                continue;
            }
            if contains_on_some_line(&corpora.markdown_text, name) {
                items.push((
                    Severity::Warn,
                    format!(
                        "{where_}: artifact {name} is documented but no script names it \u{2014} agent-authored or dead"
                    ),
                ));
            } else {
                items.push((
                    Severity::Fail,
                    format!(
                        "{where_}: diagram names artifact {name} \u{2014} no such file, and nothing in the repo names it"
                    ),
                ));
            }
        }
    }
    ReferenceOutcome {
        scripts_seen,
        artifacts_seen,
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::Token;

    fn corpora(tree: &[&str], script_text: &str, markdown_text: &str) -> Corpora {
        Corpora {
            tree: tree.iter().map(|s| s.to_string()).collect(),
            script_text: script_text.to_string(),
            markdown_text: markdown_text.to_string(),
        }
    }

    #[test]
    fn an_existing_script_passes() {
        let docs = vec![(
            "doc.md".to_string(),
            vec![Token {
                line: 1,
                text: "foo.sh".to_string(),
            }],
        )];
        let c = corpora(&["scripts/foo.sh"], "", "");
        let outcome = check_references(&docs, &c);
        assert_eq!(outcome.scripts_seen, 1);
        assert!(outcome.items.is_empty());
    }

    #[test]
    fn a_missing_script_fails() {
        let docs = vec![(
            "doc.md".to_string(),
            vec![Token {
                line: 1,
                text: "missing.sh".to_string(),
            }],
        )];
        let c = corpora(&[], "", "");
        let outcome = check_references(&docs, &c);
        assert_eq!(outcome.items.len(), 1);
        assert_eq!(outcome.items[0].0, Severity::Fail);
    }

    #[test]
    fn a_documented_but_unreferenced_artifact_warns() {
        let docs = vec![(
            "doc.md".to_string(),
            vec![Token {
                line: 1,
                text: "artifact.json".to_string(),
            }],
        )];
        let c = corpora(&[], "", "mentions artifact.json here");
        let outcome = check_references(&docs, &c);
        assert_eq!(outcome.artifacts_seen, 1);
        assert_eq!(outcome.items[0].0, Severity::Warn);
    }

    #[test]
    fn an_artifact_named_only_in_its_own_originating_document_fails_not_warns() {
        // corpus 3 (markdown_text) already had this document's own path
        // excluded by the caller (AR-128), so this simulates that: the
        // artifact does not appear in corpus 3 even though it would appear
        // in the raw originating document's own text.
        let docs = vec![(
            "doc.md".to_string(),
            vec![Token {
                line: 1,
                text: "self-only.json".to_string(),
            }],
        )];
        let c = corpora(&[], "", "");
        let outcome = check_references(&docs, &c);
        assert_eq!(outcome.items[0].0, Severity::Fail);
    }

    #[test]
    fn cross_document_and_repeated_names_are_counted_once() {
        let docs = vec![
            (
                "a.md".to_string(),
                vec![
                    Token {
                        line: 5,
                        text: "shared.sh".to_string(),
                    },
                    Token {
                        line: 9,
                        text: "shared.sh".to_string(),
                    },
                ],
            ),
            (
                "b.md".to_string(),
                vec![Token {
                    line: 2,
                    text: "shared.sh".to_string(),
                }],
            ),
        ];
        let c = corpora(&["scripts/shared.sh"], "", "");
        let outcome = check_references(&docs, &c);
        assert_eq!(outcome.scripts_seen, 1);
    }

    #[test]
    fn the_first_occurrence_in_scan_order_wins_among_duplicate_names() {
        let docs = vec![
            (
                "a.md".to_string(),
                vec![Token {
                    line: 9,
                    text: "missing.sh".to_string(),
                }],
            ),
            (
                "b.md".to_string(),
                vec![Token {
                    line: 1,
                    text: "missing.sh".to_string(),
                }],
            ),
        ];
        let c = corpora(&[], "", "");
        let outcome = check_references(&docs, &c);
        assert!(outcome.items[0].1.starts_with("a.md:9:"));
    }
}
