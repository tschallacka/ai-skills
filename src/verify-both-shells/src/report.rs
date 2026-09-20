// MODE: DEV
// PACKAGE: PROD

pub struct ReportResult {
    pub text: String,
    pub is_failure: bool,
}

/// A leg the host cannot have at all (see `Leg::skip_reason`). Not a failure:
/// nothing was attempted, and the reason says so in the report.
pub fn skipped(label: &str, reason: &str) -> ReportResult {
    ReportResult {
        text: format!("=== {label} -- SKIPPED: {reason} ===\n"),
        is_failure: false,
    }
}

/// Reproduces `report()`'s exact real semantics: a log with no `Total ran`
/// substring anywhere means the leg did not run at all (its own last 20
/// lines are shown for diagnosis); otherwise every `Total ran`/`^Failed:`
/// line is shown, and if any line starts with `Failed:`, the failing-test
/// blocks are extracted via the AWK-equivalent state machine below.
pub fn report(label: &str, log_text: &str) -> ReportResult {
    if !log_text.contains("Total ran") {
        let mut text = format!("=== {label} -- NO SUMMARY, the leg did not run ===\n");
        text.push_str(&tail_lines(log_text, 20));
        return ReportResult {
            text,
            is_failure: true,
        };
    }

    let mut text = format!("=== {label} ===\n");
    for line in log_text.lines() {
        if line.contains("Total ran") || line.starts_with("Failed:") {
            text.push_str(line);
            text.push('\n');
        }
    }

    let has_failed_line = log_text.lines().any(|l| l.starts_with("Failed:"));
    if !has_failed_line {
        return ReportResult {
            text,
            is_failure: false,
        };
    }

    text.push_str(&format!("--- {label}: what each failing test said ---\n"));
    text.push_str(&extract_failing_blocks(log_text));
    ReportResult {
        text,
        is_failure: true,
    }
}

fn tail_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(n);
    let mut out = String::new();
    for line in &lines[start..] {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// `/^  [^ ].* (PASS|FAIL|UNCONFIGURED)/ { inblock = ($0 ~ /FAIL/) }`
/// `inblock { print }` -- a marker line toggles `in_block` to whether the
/// WHOLE marker line contains the substring `FAIL` (unanchored, independent
/// of which status word actually satisfied the marker condition); every
/// line while `in_block` is true, including the marker line itself, is
/// included up to (not including) the next marker line.
fn extract_failing_blocks(log_text: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;
    for line in log_text.lines() {
        if is_marker_line(line) {
            in_block = line.contains("FAIL");
        }
        if in_block {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Exactly two leading spaces, then a non-space character, then a literal
/// space followed by `PASS`, `FAIL`, or `UNCONFIGURED` appearing anywhere
/// later in the line.
fn is_marker_line(line: &str) -> bool {
    let bytes = line.as_bytes();
    if bytes.len() < 3 || bytes[0] != b' ' || bytes[1] != b' ' || bytes[2] == b' ' {
        return false;
    }
    let rest = &line[2..];
    rest.contains(" PASS") || rest.contains(" FAIL") || rest.contains(" UNCONFIGURED")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_total_ran_substring_anywhere_is_reported_as_no_summary() {
        let result = report("bash 5.3", "some crash output\nwith no summary at all\n");
        assert!(result.is_failure);
        assert!(result.text.contains("NO SUMMARY, the leg did not run"));
    }

    #[test]
    fn a_total_ran_with_no_failed_line_is_success() {
        let log = "running things\nTotal ran: 5   Passed: 5\n";
        let result = report("bash 5.3", log);
        assert!(!result.is_failure);
        assert!(result.text.contains("Total ran: 5"));
        assert!(!result.text.contains("what each failing test said"));
    }

    #[test]
    fn a_failed_line_extracts_only_the_failing_blocks() {
        let log = "\
Total ran: 3   Passed: 2   Failed: 1
Failed: test-b
  test-a  PASS
  test-b  FAIL
    detail line one
    detail line two
  test-c  UNCONFIGURED
";
        let result = report("bash 5.3", log);
        assert!(result.is_failure);
        assert!(result.text.contains("  test-b  FAIL"));
        assert!(result.text.contains("detail line one"));
        assert!(result.text.contains("detail line two"));
        assert!(!result.text.contains("  test-a  PASS"));
        assert!(!result.text.contains("  test-c  UNCONFIGURED"));
    }

    #[test]
    fn a_pass_marker_line_that_also_contains_the_literal_fail_substring_still_opens_a_block() {
        // Real bash's own $0 ~ /FAIL/ is unanchored and whole-line: it does
        // not care which status word actually matched the marker condition.
        let log = "\
Total ran: 1   Passed: 0   Failed: 1
Failed: test-a
  test-a-FAIL-in-name  PASS
    should still be captured
  test-b  UNCONFIGURED
";
        let result = report("bash 5.3", log);
        assert!(result.text.contains("test-a-FAIL-in-name"));
        assert!(result.text.contains("should still be captured"));
        assert!(!result.text.contains("  test-b  UNCONFIGURED"));
    }
}
