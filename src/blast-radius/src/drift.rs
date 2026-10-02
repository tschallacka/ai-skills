// MODE: DEV
// PACKAGE: PROD
use crate::finding::{note, warn, Line};
use crate::git;
use std::path::Path;

/// AR-75: the `%s changed path(s), base %s` banner is printed by `run()`
/// BEFORE this pass ever runs, and must always show the RAW `base` argument
/// (or the default `master`), never a resolved value -- the `origin/<base>`
/// fallback below is purely internal to this pass; only this pass's own
/// `note:` lines (and its own resolve-failure warning) show the resolved
/// value.
pub fn base_drift(repo_root: &Path, base: &str, changed: &[String]) -> Vec<Line> {
    let resolved_base = resolve_base(repo_root, base);
    let mut lines = Vec::new();

    match git::merge_base(repo_root, &resolved_base) {
        Some(merge_base) => {
            let range = format!("{merge_base}..HEAD");
            let mut drifted = 0usize;
            for path in changed {
                if path.is_empty() {
                    continue;
                }
                if !repo_root.join(path).exists() {
                    continue;
                }
                let commits = git::log_oneline_count(repo_root, &range, path);
                if commits == 0 {
                    continue;
                }
                drifted += 1;
                lines.push(note(format!(
                    "{path} changed in {commits} commit(s) since {resolved_base}"
                )));
            }
            if drifted > 0 {
                lines.push(note(
                    "rebase or verify those commits survive before merging",
                ));
            }
        }
        None => {
            lines.push(warn(format!(
                "cannot resolve a merge base with {resolved_base}; drift not checked"
            )));
        }
    }
    lines
}

/// If `base` does not resolve locally but `origin/<base>` does, use that --
/// the fresh-clone/CI fallback (a fresh clone or a shallow CI checkout has
/// `origin/master` but no local `master`).
fn resolve_base(repo_root: &Path, base: &str) -> String {
    if !git::rev_parse_verify_quiet(repo_root, base) {
        let origin_ref = format!("origin/{base}");
        if git::rev_parse_verify_quiet(repo_root, &origin_ref) {
            return origin_ref;
        }
    }
    base.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    /// A scratch git repo with one commit and a fake `origin/<branch>`
    /// remote-tracking ref (a plain local ref under `refs/remotes/origin/`,
    /// no real remote needed) -- enough to exercise the fresh-clone/CI
    /// fallback without a network or a second repository.
    fn scratch_repo_with_origin_ref(tag: &str, branch: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "blast-radius-drift-test-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(&dir)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?} failed");
        };
        git(&["init", "-q", "-b", "unrelated-local-branch"]);
        git(&["config", "user.email", "test@example.com"]);
        git(&["config", "user.name", "Test"]);
        std::fs::write(dir.join("f.txt"), "x").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "initial"]);
        let head = String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&dir)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        git(&[
            "update-ref",
            &format!("refs/remotes/origin/{branch}"),
            head.trim(),
        ]);
        dir
    }

    #[test]
    fn falls_back_to_origin_ref_when_the_bare_ref_does_not_exist_locally() {
        let dir = scratch_repo_with_origin_ref("fallback", "master");
        assert_eq!(resolve_base(&dir, "master"), "origin/master");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_ref_that_resolves_locally_is_used_as_is() {
        let dir = scratch_repo_with_origin_ref("no-fallback", "master");
        assert_eq!(
            resolve_base(&dir, "unrelated-local-branch"),
            "unrelated-local-branch"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_ref_with_no_local_or_origin_match_is_returned_unresolved() {
        let dir = scratch_repo_with_origin_ref("neither", "master");
        assert_eq!(
            resolve_base(&dir, "totally-unknown-ref"),
            "totally-unknown-ref"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
