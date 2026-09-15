// MODE: DEV
// PACKAGE: PROD
//! Intra-document self-coherence checks (B110) formerly provided by
//! `validate-plan-coherence-lib.sh` and its two awk programs
//! (`validate-plan-stale-wording.awk`, `validate-plan-countable-enumeration.awk`).

use planning_validator_common::Findings;
use planning_validator_stale::STALE_MARKERS;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

struct Paragraph {
    number: usize,
    text: String,
    is_retraction: bool,
}

/// `^#+ ` in the shared awk programs: one or more `#` immediately followed by
/// a space, anchored at the start of the line.
fn is_heading_line(line: &str) -> bool {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    hashes >= 1 && line.as_bytes().get(hashes) == Some(&b' ')
}

fn flush_paragraph(
    current: &mut String,
    seen_heading: bool,
    number: &mut usize,
    result: &mut Vec<Paragraph>,
) {
    if seen_heading && !current.is_empty() {
        *number += 1;
        let lower = current.to_ascii_lowercase();
        let is_retraction = STALE_MARKERS.iter().any(|marker| lower.contains(marker));
        result.push(Paragraph {
            number: *number,
            text: std::mem::take(current),
            is_retraction,
        });
    }
    current.clear();
}

/// Paragraphs after the first heading, blank-line and heading delimited, each
/// flattened to one line -- the same buffering `flush()` uses in both awk
/// programs (a heading flushes and starts a new paragraph; text before the
/// first heading is never counted).
fn paragraphs(text: &str) -> Vec<Paragraph> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut seen_heading = false;
    let mut number = 0usize;
    for line in text.lines() {
        if is_heading_line(line) {
            flush_paragraph(&mut current, seen_heading, &mut number, &mut result);
            seen_heading = true;
            continue;
        }
        if line.trim().is_empty() {
            flush_paragraph(&mut current, seen_heading, &mut number, &mut result);
            continue;
        }
        if seen_heading {
            if current.is_empty() {
                current.push_str(line);
            } else {
                current.push(' ');
                current.push_str(line);
            }
        }
    }
    flush_paragraph(&mut current, seen_heading, &mut number, &mut result);
    result
}

/// Double-quoted spans, or single-quoted spans whose opening quote is not
/// itself a contraction's apostrophe (guarded by requiring the character
/// before it not be a letter, and the character after its closing match not
/// be a letter), at least 8 characters long. Mirrors `extract_claims` in
/// `validate-plan-stale-wording.awk` exactly, character by character, since
/// POSIX awk's char-scanned approach has no direct regex equivalent here that
/// preserves the same contraction guard.
fn extract_claims(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut result = Vec::new();
    let mut i = 0;
    while i < n {
        let c = chars[i];
        if c == '"' {
            let start = i + 1;
            if let Some(end) = (start..n).find(|&j| chars[j] == '"') {
                let claim: String = chars[start..end].iter().collect();
                if claim.chars().count() >= 8 {
                    result.push(claim);
                }
                i = end + 1;
                continue;
            }
        } else if c == '\'' {
            let prev_is_letter = i > 0 && chars[i - 1].is_ascii_alphabetic();
            if !prev_is_letter {
                let mut found = None;
                let mut j = i + 1;
                while j < n {
                    if chars[j] == '\'' {
                        let next_is_letter = j + 1 < n && chars[j + 1].is_ascii_alphabetic();
                        if !next_is_letter {
                            found = Some(j);
                            break;
                        }
                    }
                    j += 1;
                }
                if let Some(end) = found {
                    let claim: String = chars[i + 1..end].iter().collect();
                    if claim.chars().count() >= 8 {
                        result.push(claim);
                    }
                    i = end + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    result
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut truncated: String = text.chars().take(limit.saturating_sub(3)).collect();
    truncated.push_str("...");
    truncated
}

/// T138/B110: a claim a paragraph declares stale (quoted, beside a
/// `STALE_MARKERS` phrase) that survives verbatim in another, non-retraction
/// paragraph of the same document.
pub fn validate_stale_wording_retained(stale_docs: &[&Path], findings: &mut Findings) {
    for doc in stale_docs {
        let Ok(text) = fs::read_to_string(doc) else {
            continue;
        };
        let paras = paragraphs(&text);
        let mut claims = Vec::new();
        for paragraph in &paras {
            if paragraph.is_retraction {
                for claim in extract_claims(&paragraph.text) {
                    claims.push((claim, paragraph.number));
                }
            }
        }
        for (claim, origin) in &claims {
            for paragraph in &paras {
                if paragraph.number == *origin || paragraph.is_retraction {
                    continue;
                }
                if paragraph.text.contains(claim.as_str()) {
                    findings.fail(format!(
                        "{}: paragraph {origin} declares '{}' stale, but paragraph {} still carries it verbatim -- update or remove the stale wording",
                        doc.display(),
                        truncate(claim, 80),
                        paragraph.number,
                    ));
                }
            }
        }
    }
}

fn trigger_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"the following (one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|[0-9]+) (steps?|goals?|files?|units?|documents?|docs?|bugs?|todos?|items?|criteria|stories|findings?)",
        )
        .expect("trigger regex is a fixed literal")
    })
}

fn member_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(W[0-9][0-9]+)|([BT][0-9]+)|([A-Za-z0-9_-]+/[A-Za-z0-9_./-]+)|([A-Za-z0-9_-]+\.[A-Za-z]+)")
            .expect("member regex is a fixed literal")
    })
}

/// T139/B110: "the following N `<things>`" with no explicit list of the N in
/// the same paragraph -- a count with no concrete referent (a WNN/BNN/TNN id
/// or a file-path-like token) a reader, or this check, could verify it
/// against.
pub fn validate_countable_enumeration(stale_docs: &[&Path], findings: &mut Findings) {
    for doc in stale_docs {
        let Ok(text) = fs::read_to_string(doc) else {
            continue;
        };
        for paragraph in paragraphs(&text) {
            let lower = paragraph.text.to_ascii_lowercase();
            if trigger_regex().is_match(&lower) && !member_regex().is_match(&paragraph.text) {
                findings.fail(format!(
                    "{}: paragraph {} [{}] names a count with no explicit member in the same paragraph -- list the specific ids/paths, or reduce to a plain count with no promise of a following list",
                    doc.display(),
                    paragraph.number,
                    truncate(&paragraph.text, 160),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch_doc(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "validator-coherence-{name}-{}.md",
            std::process::id()
        ));
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn a_retracted_claim_surviving_verbatim_is_flagged() {
        let doc = scratch_doc(
            "survivor",
            "## Objective\n\nleave dropship buckets unchanged\n\n## Correction\n\nAn earlier version of this said \"leave dropship buckets unchanged\"; that no longer applies.\n",
        );
        let mut findings = Findings::default();
        validate_stale_wording_retained(&[doc.as_path()], &mut findings);
        assert_eq!(findings.errors, 1);
        let _ = fs::remove_file(doc);
    }

    #[test]
    fn a_second_retraction_paragraph_repeating_the_same_claim_is_not_a_survivor() {
        // Two retraction paragraphs (2, 3) both quote the same claim, so both
        // cite paragraph 1 (the Objective) as a survivor -- but paragraph 3
        // must never itself be reported as a survivor, since restating a
        // retraction is not leaving the claim in force.
        let doc = scratch_doc(
            "no-survivor",
            "## Objective\n\nleave dropship buckets unchanged\n\n## First correction\n\nAn earlier version of this said \"leave dropship buckets unchanged\"; that no longer applies.\n\n## Second correction, same claim repeated\n\nAn earlier version of this row also said 'leave dropship buckets unchanged', which finding AR-08 recorded as wrong and superseded here.\n",
        );
        let mut findings = Findings::default();
        validate_stale_wording_retained(&[doc.as_path()], &mut findings);
        assert_eq!(
            findings.errors, 2,
            "paragraphs 2 and 3 both cite paragraph 1 as the survivor"
        );
        let messages = findings
            .messages
            .iter()
            .map(|message| message.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !messages.contains("but paragraph 3 still carries it"),
            "paragraph 3 (a retraction itself) must never be reported as a survivor: {messages}"
        );
        assert!(
            messages.contains("but paragraph 1 still carries it"),
            "the Objective paragraph (1) must be reported as a survivor: {messages}"
        );
        let _ = fs::remove_file(doc);
    }

    #[test]
    fn a_contractions_apostrophe_is_not_misread_as_a_quote() {
        let doc = scratch_doc(
            "contraction",
            "## Notes\n\nAn earlier version of this said the unit's own instructions were unclear.\n",
        );
        let mut findings = Findings::default();
        validate_stale_wording_retained(&[doc.as_path()], &mut findings);
        assert_eq!(findings.errors, 0);
        let _ = fs::remove_file(doc);
    }

    #[test]
    fn the_following_n_things_with_no_member_is_flagged() {
        let doc = scratch_doc(
            "enumeration",
            "## Handoff\n\nThe following four steps each add one public method.\n",
        );
        let mut findings = Findings::default();
        validate_countable_enumeration(&[doc.as_path()], &mut findings);
        assert_eq!(findings.errors, 1);
        let _ = fs::remove_file(doc);
    }

    #[test]
    fn the_following_n_things_with_an_explicit_member_is_silent() {
        let doc = scratch_doc(
            "enumeration-ok",
            "## Handoff\n\nThe following four steps (W01, W02, W03, W04) each add one public method.\n",
        );
        let mut findings = Findings::default();
        validate_countable_enumeration(&[doc.as_path()], &mut findings);
        assert_eq!(findings.errors, 0);
        let _ = fs::remove_file(doc);
    }

    #[test]
    fn a_universal_count_is_not_the_following_n_shape() {
        let doc = scratch_doc(
            "universal",
            "## Overview\n\nAll twelve goals are complete.\n",
        );
        let mut findings = Findings::default();
        validate_countable_enumeration(&[doc.as_path()], &mut findings);
        assert_eq!(findings.errors, 0);
        let _ = fs::remove_file(doc);
    }
}
