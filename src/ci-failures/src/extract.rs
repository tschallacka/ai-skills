// MODE: DEV
// PACKAGE: PROD
use regex::Regex;
use std::sync::OnceLock;

fn ansi_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap())
}

fn timestamp_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:.]+Z ?").unwrap())
}

fn fail_paren_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"FAIL[ \t]*[:(]").unwrap())
}

fn fail_colon_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[ \t]*(FAIL|portability):").unwrap())
}

fn error_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^error(\[|:)").unwrap())
}

fn failed_colon_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[ \t]*Failed:").unwrap())
}

/// A line is a "detail continuation" line when its first four characters
/// are all whitespace -- matching awk's own
/// `/^[[:space:]][[:space:]][[:space:]][[:space:]]/` (four repetitions of a
/// single-whitespace-char class, not an anchored "exactly four" count).
fn has_four_space_indent(line: &str) -> bool {
    let mut chars = line.chars();
    (0..4).all(|_| chars.next().is_some_and(|c| c.is_whitespace()))
}

/// Strips ANSI CSI color sequences and a single trailing CR from one raw
/// line, then strips a single leading GitHub-style ISO-8601 timestamp (a
/// no-op on a GitLab trace line, which carries none) -- mirroring ci-
/// failures.sh's own extract() awk script exactly: first $0 itself is
/// cleaned of escape codes/CR, then a separate `line` variable additionally
/// drops the timestamp, and every pattern rule below matches against that
/// fully-cleaned `line`.
fn clean_line(raw: &str) -> String {
    let no_ansi = ansi_re().replace_all(raw, "");
    let no_cr = no_ansi.strip_suffix('\r').unwrap_or(&no_ansi);
    timestamp_re().replace(no_cr, "").into_owned()
}

/// Mirrors the `sed -e "s/${esc}\[[0-9;]*[a-zA-Z]//g" -e 's/\r$//'` pipeline
/// gh_print_job/glab_print_job run over a job's raw log before writing it to
/// --raw DIR: every ANSI CSI sequence removed (all occurrences per line,
/// matching sed's own `g` flag), then one trailing CR stripped per line.
/// Unlike clean_line (used only for pattern-matching), this does NOT strip a
/// leading timestamp -- the raw log file is meant to be the de-escaped
/// original, not the pattern-matcher's own further-cleaned view of it.
pub fn strip_ansi_and_cr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let no_ansi = ansi_re().replace_all(line, "");
        let no_cr = no_ansi.strip_suffix('\r').unwrap_or(&no_ansi);
        out.push_str(no_cr);
        out.push('\n');
    }
    out
}

/// Reproduces ci-failures.sh's own extract() line-by-line, matching every
/// pattern rule in the exact priority order the bash source lists them
/// (lines 110-137 of ci-failures.sh), including the two pieces of state it
/// carries across lines: `detail` (continuing an indented FAIL block) and
/// `after` (continuing N more lines verbatim after panicked-at/timed-out).
/// Every matched line (and its continuation lines) is emitted with the same
/// four-space prefix bash's own `print "    " line` uses.
pub fn extract(input: &str) -> String {
    let mut detail = false;
    let mut after: u32 = 0;
    let mut out = String::new();

    for raw_line in input.lines() {
        let line = clean_line(raw_line);

        if detail && has_four_space_indent(&line) {
            out.push_str("    ");
            out.push_str(&line);
            out.push('\n');
            continue;
        }
        if detail {
            detail = false;
        }
        if after > 0 {
            out.push_str("    ");
            out.push_str(&line);
            out.push('\n');
            after -= 1;
            continue;
        }
        if line.contains("panicked at") {
            out.push_str("    ");
            out.push_str(&line);
            out.push('\n');
            after = 4;
            continue;
        }
        if failed_colon_re().is_match(&line) {
            out.push_str("    ");
            out.push_str(&line);
            out.push('\n');
            continue;
        }
        if line.contains("Total ran:") {
            out.push_str("    ");
            out.push_str(&line);
            out.push('\n');
            continue;
        }
        if line.contains("test result: FAILED") {
            out.push_str("    ");
            out.push_str(&line);
            out.push('\n');
            continue;
        }
        if line.contains("##[error]") {
            out.push_str("    ");
            out.push_str(&line);
            out.push('\n');
            continue;
        }
        if fail_paren_re().is_match(&line) {
            out.push_str("    ");
            out.push_str(&line);
            out.push('\n');
            detail = true;
            continue;
        }
        if fail_colon_re().is_match(&line) {
            out.push_str("    ");
            out.push_str(&line);
            out.push('\n');
            continue;
        }
        if error_re().is_match(&line) {
            out.push_str("    ");
            out.push_str(&line);
            out.push('\n');
            continue;
        }
        if line.contains("timed out waiting") {
            out.push_str("    ");
            out.push_str(&line);
            out.push('\n');
            after = 2;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panicked_at_pulls_four_trailing_lines() {
        let input =
            "before\nthread panicked at src/lib.rs:1\nline1\nline2\nline3\nline4\nline5\nafter\n";
        let out = extract(input);
        assert!(out.contains("panicked at src/lib.rs:1"));
        assert!(out.contains("    line1"));
        assert!(out.contains("    line4"));
        assert!(!out.contains("line5"));
        assert!(!out.contains("after"));
        assert!(!out.contains("before"));
    }

    #[test]
    fn failed_colon_line_is_captured() {
        let out = extract("  Failed: test_foo\n");
        assert!(out.contains("Failed: test_foo"));
    }

    #[test]
    fn total_ran_line_is_captured() {
        let out = extract("Total ran: 10   Passed: 9   Failed: 1\n");
        assert!(out.contains("Total ran:"));
    }

    #[test]
    fn test_result_failed_is_captured() {
        let out = extract("test result: FAILED. 0 passed; 1 failed\n");
        assert!(out.contains("test result: FAILED"));
    }

    #[test]
    fn github_error_annotation_is_captured() {
        let out = extract("##[error]Process completed with exit code 1.\n");
        assert!(out.contains("##[error]"));
    }

    #[test]
    fn fail_with_paren_continues_indented_detail_lines() {
        let input = "  test-foo    FAIL (exit 1)\n    assertion failed here\n    second detail line\nnext test PASS\n";
        let out = extract(input);
        assert!(out.contains("FAIL (exit 1)"));
        assert!(out.contains("    assertion failed here"));
        assert!(out.contains("    second detail line"));
        assert!(!out.contains("next test PASS"));
    }

    #[test]
    fn fail_colon_line_is_captured() {
        let out = extract("FAIL: something went wrong\n");
        assert!(out.contains("FAIL: something went wrong"));
    }

    #[test]
    fn portability_colon_line_is_captured() {
        let out = extract("portability: a finding was reported\n");
        assert!(out.contains("portability: a finding was reported"));
    }

    #[test]
    fn error_bracket_or_colon_prefixed_line_is_captured() {
        assert!(extract("error[E0502]: cannot borrow\n").contains("error[E0502]"));
        assert!(extract("error: something failed\n").contains("error: something failed"));
    }

    #[test]
    fn timed_out_waiting_pulls_two_trailing_lines() {
        let input = "timed out waiting for the process\nline1\nline2\nline3\n";
        let out = extract(input);
        assert!(out.contains("timed out waiting"));
        assert!(out.contains("    line1"));
        assert!(out.contains("    line2"));
        assert!(!out.contains("line3"));
    }

    #[test]
    fn ansi_and_cr_and_timestamp_are_stripped_before_matching() {
        let input = "2026-09-15T12:00:00.123Z \x1b[31mFAIL: colored and timestamped\x1b[0m\r\n";
        let out = extract(input);
        assert!(out.contains("FAIL: colored and timestamped"));
        assert!(!out.contains('\x1b'));
        assert!(!out.contains("2026-09-15"));
    }

    #[test]
    fn a_gitlab_style_line_with_no_timestamp_prefix_is_unaffected() {
        let out = extract("FAIL: no timestamp here at all\n");
        assert!(out.contains("FAIL: no timestamp here at all"));
    }

    #[test]
    fn an_uninteresting_line_produces_no_output() {
        assert_eq!(extract("just an ordinary log line\n"), "");
    }

    #[test]
    fn detail_continuation_stops_when_indentation_drops() {
        let input = "test-foo    FAIL (exit 1)\n    detail one\nnext test PASS\n    unrelated indented text\n";
        let out = extract(input);
        assert!(out.contains("    detail one"));
        assert!(!out.contains("next test PASS"));
        assert!(!out.contains("unrelated indented text"));
    }
}
