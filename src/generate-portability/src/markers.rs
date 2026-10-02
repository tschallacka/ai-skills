// MODE: DEV
// PACKAGE: PROD

//! marker_sightings: reimplements generate-portability.sh's own awk fold
//! algorithm directly in Rust, matching build-plan-libs/generate-skill-docs's
//! own precedent of reimplementing text-processing logic natively rather
//! than shelling to awk per file.
//!
//! The real awk loop is `while ((getline nx) > 0 && nx ~ /^[[:space:]]*#/)`.
//! This ALWAYS calls getline first (consuming exactly one following line)
//! before testing it -- so after every marker occurrence, the very next
//! line in the file is always pulled out of the input stream. If it matches
//! the comment-continuation pattern, it is folded in and the loop checks the
//! line after THAT the same way. The FIRST line that does not match is not
//! merely "left for the outer scan" -- it is unconditionally and
//! irrecoverably DISCARDED, never seen by the outer per-line marker check
//! again (AR-65, verified empirically against the real awk source: a marker
//! immediately followed by a non-#-leading line that itself carries a
//! second marker only as a trailing comment loses that second marker's
//! sighting entirely, silently). This module reproduces that exact
//! consume-then-discard behavior, not merely "don't treat a continuation
//! line as a new marker."

use std::path::Path;

const TRIGGER: &str = "# PORTABILITY(";

fn is_comment_continuation(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

fn fold_comment_text(line: &str) -> String {
    let trimmed = line.trim_start();
    let after_hash = trimmed.strip_prefix('#').unwrap_or(trimmed);
    after_hash.trim_start().to_string()
}

/// Extracts (rule_id, initial_text) from a line known to contain the
/// trigger. Assumes one marker per line, matching realistic usage; awk's
/// own greedy `.*` would technically anchor to the LAST occurrence on a
/// line with more than one, which never happens in practice.
fn extract_rule_and_text(line: &str) -> Option<(String, String)> {
    let start = line.find(TRIGGER)?;
    let after_open = start + TRIGGER.len();
    let close_rel = line[after_open..].find(')')?;
    let rule_id = line[after_open..after_open + close_rel].to_string();
    let mut rest = &line[after_open + close_rel + 1..];
    if let Some(stripped) = rest.strip_prefix(':') {
        rest = stripped;
    }
    Some((rule_id, rest.trim_start().to_string()))
}

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One (rule_id, file, text) triple per marker occurrence, in file-scan
/// order (the order `files` is given in, matching `script_list`'s own
/// output order).
pub fn marker_sightings(repo_root: &Path, files: &[String]) -> Vec<(String, String, String)> {
    let mut sightings = Vec::new();
    for file in files {
        let Ok(contents) = std::fs::read_to_string(repo_root.join(file)) else {
            continue;
        };
        let lines: Vec<&str> = contents.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            if !lines[i].contains(TRIGGER) {
                i += 1;
                continue;
            }
            let Some((rule_id, mut text)) = extract_rule_and_text(lines[i]) else {
                i += 1;
                continue;
            };
            let mut j = i + 1;
            loop {
                if j >= lines.len() {
                    break;
                }
                if is_comment_continuation(lines[j]) {
                    let folded = fold_comment_text(lines[j]);
                    if !folded.is_empty() {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(&folded);
                    }
                    j += 1;
                } else {
                    // Consumed and irrecoverably discarded -- AR-65.
                    j += 1;
                    break;
                }
            }
            sightings.push((rule_id, file.clone(), normalize(&text)));
            i = j;
        }
    }
    sightings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "generate-portability-markers-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sightings_for(dir: &Path, name: &str, content: &str) -> Vec<(String, String, String)> {
        fs::write(dir.join(name), content).unwrap();
        marker_sightings(dir, &[name.to_string()])
    }

    #[test]
    fn single_line_marker_no_trailing_text() {
        let dir = scratch("single-no-text");
        let sightings = sightings_for(&dir, "a.sh", "# PORTABILITY(rule-a)\nfoo\n");
        assert_eq!(sightings, vec![("rule-a".into(), "a.sh".into(), "".into())]);
    }

    #[test]
    fn single_line_marker_with_trailing_text() {
        let dir = scratch("single-with-text");
        let sightings = sightings_for(&dir, "a.sh", "foo  # PORTABILITY(rule-a): why here\nbar\n");
        assert_eq!(
            sightings,
            vec![("rule-a".into(), "a.sh".into(), "why here".into())]
        );
    }

    #[test]
    fn folded_multi_line_continuation() {
        let dir = scratch("folded");
        let sightings = sightings_for(
            &dir,
            "a.sh",
            "# PORTABILITY(rule-a): first line\n# second line\n# third line\nnot-a-comment\n",
        );
        assert_eq!(
            sightings,
            vec![(
                "rule-a".into(),
                "a.sh".into(),
                "first line second line third line".into()
            )]
        );
    }

    #[test]
    fn a_continuation_line_containing_a_second_marker_is_folded_not_treated_as_new() {
        let dir = scratch("no-re-scan");
        let sightings = sightings_for(
            &dir,
            "a.sh",
            "# PORTABILITY(rule-a): first\n# PORTABILITY(rule-b): swallowed as text\ncode()\n",
        );
        assert_eq!(sightings.len(), 1);
        assert_eq!(sightings[0].0, "rule-a");
        assert!(sightings[0].2.contains("PORTABILITY(rule-b)"));
    }

    /// AR-65: a marker immediately followed by a NON-#-leading line that
    /// itself carries a second marker only as a trailing comment loses that
    /// second marker's sighting entirely -- it is consumed by the
    /// unconditional getline and discarded, never re-scanned as its own
    /// marker.
    #[test]
    fn a_marker_immediately_followed_by_a_non_comment_line_discards_that_line_entirely() {
        let dir = scratch("ar-65-discard");
        let sightings = sightings_for(
            &dir,
            "a.sh",
            "# PORTABILITY(rule-a): first\nsome_code()  # PORTABILITY(rule-b): lost\n# PORTABILITY(rule-c): third\n",
        );
        let rule_ids: Vec<&str> = sightings.iter().map(|(id, _, _)| id.as_str()).collect();
        assert_eq!(rule_ids, vec!["rule-a", "rule-c"]);
    }

    #[test]
    fn a_file_with_no_markers_yields_nothing() {
        let dir = scratch("none");
        let sightings = sightings_for(&dir, "a.sh", "#!/usr/bin/env bash\necho hi\n");
        assert!(sightings.is_empty());
    }
}
