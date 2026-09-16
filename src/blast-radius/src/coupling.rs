// MODE: DEV
// PACKAGE: PROD
use crate::finding::{fail, ok, warn, Line};
use crate::globs;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Reads `registry` as TSV rows of `glob\tlevel\tconsequence\tcheck`,
/// skipping blank lines and lines whose glob starts with `#`. For each row
/// with a hit against `changed` (first match), either runs the row's check
/// command or reports a static fail/warn.
pub fn run_pass(repo_root: &Path, registry: &Path, changed: &[String]) -> Vec<Line> {
    let contents = std::fs::read_to_string(registry)
        .unwrap_or_else(|e| panic!("reading {}: {e}", registry.display()));
    let mut lines = Vec::new();
    // AR-69: a HashSet exact-match dedup is a deliberate, documented
    // simplification of bash's own whitespace-delimited SUBSTRING
    // `ran_checks` match -- see goal section 8.1.
    let mut ran_checks: HashSet<String> = HashSet::new();

    for row in contents.lines() {
        let mut fields = row.splitn(4, '\t');
        let glob = fields.next().unwrap_or("");
        if glob.is_empty() || glob.starts_with('#') {
            continue;
        }
        let level = fields.next().unwrap_or("");
        let consequence = fields.next().unwrap_or("");
        let check = fields.next().unwrap_or("");

        let hit = changed
            .iter()
            .find(|path| !path.is_empty() && globs::matches(path, glob));
        let Some(hit) = hit else { continue };

        if !check.is_empty() {
            if ran_checks.contains(check) {
                continue;
            }
            ran_checks.insert(check.to_string());
            let (succeeded, output) = run_check(repo_root, check);
            if succeeded {
                lines.push(ok(consequence));
            } else {
                let reason = extract_reason(&output);
                lines.push(fail(format!("{consequence} — `{check}`: {reason}")));
            }
        } else if level == "fail" {
            lines.push(fail(format!("{hit}: {consequence}")));
        } else {
            lines.push(warn(format!("{hit}: {consequence}")));
        }
    }
    lines
}

/// AR-72: this spawns `check` in a FRESH subprocess, unlike real bash's
/// `eval`, which runs it in-process inside blast-radius.sh's own shell
/// (visible to that shell's own local variables/functions). No current
/// coupling.tsv check relies on shell-local state, so this divergence is
/// explicitly out of scope for byte-for-byte parity beyond exported
/// environment variables. The `( check ) 2>&1` wrapping reproduces
/// `eval "$check" 2>&1`'s merged-stream semantics: a single real
/// redirection inside the spawned bash, not two separately-captured Rust
/// pipes, which could not reconstruct the true interleaving order.
fn run_check(repo_root: &Path, check: &str) -> (bool, String) {
    let script = format!("( {check} ) 2>&1");
    let output = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(repo_root)
        .output()
        .unwrap_or_else(|e| panic!("spawning bash -c for check {check:?}: {e}"));
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
    )
}

/// The exact three-tier heuristic: a line matching `^[A-Za-z0-9._/-]+\.sh: `,
/// else the last non-blank line, else the literal string.
fn extract_reason(output: &str) -> String {
    if let Some(line) = output.lines().find(|line| has_sh_prefix(line)) {
        return line.to_string();
    }
    if let Some(line) = output.lines().rev().find(|line| !line.trim().is_empty()) {
        return line.to_string();
    }
    "check failed with no output".to_string()
}

/// `^[A-Za-z0-9._/-]+\.sh: ` -- the identifier class includes `.`, so a
/// greedy scan must backtrack: try the longest run of identifier bytes down
/// to length 1, looking for the first cut point where what remains starts
/// with the literal `.sh: ` (a naive non-backtracking scan over-consumes the
/// line's own `.sh` into the `+` run and never finds the suffix).
fn has_sh_prefix(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut max_run = 0usize;
    while max_run < bytes.len() && is_ident_byte(bytes[max_run]) {
        max_run += 1;
    }
    (1..=max_run)
        .rev()
        .any(|len| bytes[len..].starts_with(b".sh: "))
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'/' | b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sh_prefix_line_is_recognized() {
        assert!(has_sh_prefix(
            "build-plan-libs.sh: No such file or directory"
        ));
        assert!(!has_sh_prefix("no prefix here"));
        assert!(!has_sh_prefix(".sh: leading dot has no identifier"));
    }

    #[test]
    fn extract_reason_prefers_the_sh_prefixed_line() {
        let output = "some noise\nbuild-plan-libs.sh: boom\nmore trailing noise\n";
        assert_eq!(extract_reason(output), "build-plan-libs.sh: boom");
    }

    #[test]
    fn extract_reason_falls_back_to_the_last_non_blank_line() {
        let output = "first\nsecond\n\n";
        assert_eq!(extract_reason(output), "second");
    }

    #[test]
    fn extract_reason_falls_back_to_the_literal_string_when_blank() {
        assert_eq!(extract_reason(""), "check failed with no output");
        assert_eq!(extract_reason("\n\n"), "check failed with no output");
    }
}
