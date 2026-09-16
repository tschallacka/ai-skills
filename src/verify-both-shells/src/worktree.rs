// MODE: DEV
// PACKAGE: PROD
use std::path::PathBuf;
use std::process::Command;

/// AR-78: the real sed pattern `^worktree \(.*/verify-wt\..*\)$` is a plain,
/// UNANCHORED substring match for the literal text `/verify-wt.` occurring
/// anywhere in the porcelain path (with arbitrary text before and after) --
/// not a "last two path components equal verify-wt.<suffix>/tree" check.
pub fn is_stale_candidate(path: &str) -> bool {
    path.contains("/verify-wt.")
}

/// Shells to real `mktemp -d "<dir>/verify-wt.XXXXXX"`, matching bash's own
/// template exactly -- both for cross-implementation naming fidelity (a
/// leftover created by one implementation must be sweepable by the other)
/// and for `mktemp`'s own atomic-creation collision-avoidance guarantee.
pub fn mktemp_scratch_dir(base: &str) -> Option<PathBuf> {
    let template = format!("{base}/verify-wt.XXXXXX");
    let output = Command::new("mktemp")
        .args(["-d", &template])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

/// Real bash creates `log5`/`log3` as INDEPENDENT `mktemp` files directly
/// under `${TMPDIR:-/tmp}` (`verify-log5.XXXXXX`/`verify-log3.XXXXXX`) --
/// siblings of, not nested inside, the worktree's own parent directory.
/// `cleanup()`'s own real order removes the worktree's parent
/// unconditionally and decides the logs' own keep-or-delete fate entirely
/// separately, so the two must never share a directory.
pub fn mktemp_file(base: &str, prefix: &str) -> Option<PathBuf> {
    let template = format!("{base}/{prefix}.XXXXXX");
    let output = Command::new("mktemp").arg(&template).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_containing_the_literal_substring_is_a_stale_candidate() {
        assert!(is_stale_candidate("/tmp/verify-wt.abc123/tree"));
        assert!(is_stale_candidate(
            "/tmp/some-prefix/verify-wt.abc123-suffix/tree/nested"
        ));
    }

    #[test]
    fn a_path_without_the_substring_is_not_a_stale_candidate() {
        assert!(!is_stale_candidate("/tmp/other-wt.abc123/tree"));
        assert!(!is_stale_candidate("/tmp/unrelated/path"));
    }
}
