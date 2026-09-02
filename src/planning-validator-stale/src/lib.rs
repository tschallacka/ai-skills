// MODE: DEV
// PACKAGE: PROD
//! The `--stale` validation pass formerly provided by
//! `validate-plan-stale-lib.sh`.

use planning_validator_common::Findings;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_PHRASES: &[&str] = &[
    "all four",
    "all six",
    "all three",
    "all five",
    "the eleven",
    "the six states",
    "all states",
    "four per-state",
];

pub const COMPARISON_PHRASES: &[&str] = &[
    "byte-identical",
    "byte-for-byte",
    "pixel-identical",
    "exactly identical",
];

const MARKERS: &[&str] = &[
    "an earlier version",
    "previously",
    "superseded by",
    "supersedes",
    "no longer",
    "was removed",
    "historically",
    "now replaced by",
];

#[derive(Debug, Eq, PartialEq)]
pub struct Hit {
    pub file: PathBuf,
    pub label: String,
    pub paragraph: usize,
    pub text: String,
}

pub fn stale_scan_doc(file: &Path, phrase: &str) -> Vec<Hit> {
    let Ok(text) = fs::read_to_string(file) else {
        return Vec::new();
    };
    let mut scanner = ParagraphScanner::default();
    for line in text.lines() {
        if is_heading(line) {
            scanner.flush(file, phrase);
            scanner.label = line.to_owned();
            scanner.paragraph = 0;
        } else if line.bytes().all(|byte| byte.is_ascii_whitespace()) {
            scanner.flush(file, phrase);
        } else if !scanner.label.is_empty() {
            scanner.content.push_str(line);
            scanner.content.push('\n');
            if scanner.flat.is_empty() {
                scanner.flat.push_str(line);
            } else {
                scanner.flat.push(' ');
                scanner.flat.push_str(line);
            }
        }
    }
    scanner.flush(file, phrase);
    scanner.hits
}

#[derive(Default)]
struct ParagraphScanner {
    label: String,
    content: String,
    flat: String,
    paragraph: usize,
    hits: Vec<Hit>,
}

impl ParagraphScanner {
    fn flush(&mut self, file: &Path, phrase: &str) {
        if self.label.is_empty() || self.flat.is_empty() {
            self.content.clear();
            self.flat.clear();
            return;
        }
        self.paragraph += 1;
        let flat_lower = self.flat.to_lowercase();
        if self.content.contains(phrase)
            && !MARKERS.iter().any(|marker| flat_lower.contains(marker))
        {
            self.hits.push(Hit {
                file: file.to_path_buf(),
                label: self.label.clone(),
                paragraph: self.paragraph,
                text: truncate(&self.flat),
            });
        }
        self.content.clear();
        self.flat.clear();
    }
}

fn truncate(value: &str) -> String {
    if value.chars().count() > 120 {
        value.chars().take(117).collect::<String>() + "..."
    } else {
        value.to_owned()
    }
}

fn is_heading(line: &str) -> bool {
    let hash_count = line.bytes().take_while(|byte| *byte == b'#').count();
    hash_count > 0 && line.as_bytes().get(hash_count) == Some(&b' ')
}

pub fn validate_stale(
    plan: &Path,
    plan_docs: &[PathBuf],
    stale_requested: bool,
    stale_file: Option<&Path>,
    findings: &mut Findings,
) {
    if !stale_requested {
        return;
    }
    let mut docs = plan_docs.to_vec();
    let mut companions = fs::read_dir(plan)
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|goal| {
            let steps = goal.path().join("steps");
            fs::read_dir(steps)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file()
                        && path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.ends_with("-testing.md"))
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    companions.sort();
    docs.extend(companions);

    let (phrases, comparison_active) = match stale_file {
        Some(path) if path != Path::new("default") => {
            if !path.is_file() {
                findings.fail(format!("--stale file not found: {}", path.display()));
                return;
            }
            (read_phrases(path), false)
        }
        _ => (
            DEFAULT_PHRASES.iter().map(|s| (*s).to_owned()).collect(),
            true,
        ),
    };
    for phrase in phrases {
        if phrase.trim().is_empty() {
            continue;
        }
        for doc in &docs {
            for hit in stale_scan_doc(doc, &phrase) {
                findings.warn(format!(
                    "count '{phrase}' in an unmarked paragraph: {}: {} [paragraph {}] {} -- a count drifts the moment a case is added, so enumerate the items or name the section that lists them",
                    hit.file.display(), hit.label, hit.paragraph, hit.text
                ));
            }
        }
    }
    if comparison_active {
        for phrase in COMPARISON_PHRASES {
            for doc in &docs {
                for hit in stale_scan_doc(doc, phrase) {
                    findings.warn(format!(
                        "wording '{phrase}' in an unmarked paragraph: {}: {} [paragraph {}] {} -- if this is an acceptance criterion, declare it in the step's '## Artifact comparisons' table (update-plan-content.sh -tp) so the comparison is checked instead of guessed",
                        hit.file.display(), hit.label, hit.paragraph, hit.text
                    ));
                }
            }
        }
    }
}

fn read_phrases(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::stale_scan_doc;
    use std::fs;

    #[test]
    fn marker_only_exempts_its_own_paragraph() {
        let root = std::env::temp_dir().join(format!("validator-stale-{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        let file = root.join("doc.md");
        fs::write(
            &file,
            "# Heading\n\nPreviously all four states were listed.\n\nThe check covers all four states today.\n",
        )
        .unwrap();
        let hits = stale_scan_doc(&file, "all four");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].paragraph, 2);
        assert!(hits[0].text.contains("today"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn headings_reset_paragraph_numbers_and_long_text_is_truncated() {
        let root =
            std::env::temp_dir().join(format!("validator-stale-long-{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        let file = root.join("doc.md");
        let long = "x".repeat(130);
        fs::write(
            &file,
            format!("# One\n\nall four {long}\n\n# Two\n\nall four now\n"),
        )
        .unwrap();
        let hits = stale_scan_doc(&file, "all four");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].paragraph, 1);
        assert_eq!(hits[1].paragraph, 1);
        assert_eq!(hits[0].text.chars().count(), 120);
        assert!(hits[0].text.ends_with("..."));
        let _ = fs::remove_dir_all(root);
    }
}
