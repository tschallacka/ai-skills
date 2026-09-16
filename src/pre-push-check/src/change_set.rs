// MODE: DEV
// PACKAGE: PROD

//! The change set: committed work on this branch plus whatever is still in
//! the worktree or the index, measured against master (or the merge base with
//! it, so a master that has moved ahead does not show its own commits as part
//! of this branch's diff). Falls back to the tracking upstream, and then to
//! worktree-only mode with an empty base, mirroring pre-push-check.sh's own
//! `base`/`base_label` resolution exactly -- including the real bash CODE,
//! not its own header comment, which incorrectly claims the no-upstream case
//! exits 65. It does not; it continues with an empty base.

use crate::report::Report;
use regex::Regex;
use std::env;
use std::path::Path;
use std::process::Command;

pub struct Base {
    pub base: Option<String>,
    pub label: Option<String>,
}

fn git_status_ok(repo_root: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn git_stdout(repo_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

pub fn resolve_base(repo_root: &Path) -> Base {
    for r in ["origin/master", "master"] {
        if git_status_ok(repo_root, &["rev-parse", "--verify", r]) {
            let base =
                git_stdout(repo_root, &["merge-base", r, "HEAD"]).unwrap_or_else(|| r.to_string());
            return Base {
                base: Some(base),
                label: Some(r.to_string()),
            };
        }
    }
    let upstream = git_stdout(
        repo_root,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    );
    Base {
        base: upstream.clone(),
        label: upstream,
    }
}

pub fn current_branch(repo_root: &Path) -> String {
    git_stdout(repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_else(|| "HEAD".to_string())
}

/// PRE_PUSH_SKIP_FETCH exists for two callers only: a test driving this
/// script in a throwaway clone whose origin is a local path, and diagnosis
/// with no network. Returns the failing-summary message (already printed) as
/// an error when the fetch itself fails and was not skipped, matching bash's
/// own immediate `exit 1`.
pub fn fetch_master(repo_root: &Path, report: &mut Report) -> Result<(), i32> {
    if env::var("PRE_PUSH_SKIP_FETCH").as_deref() == Ok("1") {
        report.note("PRE_PUSH_SKIP_FETCH=1: master not refreshed, the change set may be stale");
        return Ok(());
    }
    let has_git_dir = git_status_ok(repo_root, &["rev-parse", "--git-dir"]);
    let has_origin = git_status_ok(repo_root, &["remote", "get-url", "origin"]);
    if !(has_git_dir && has_origin) {
        report.note("no origin remote; master cannot be refreshed and the change set may be stale");
        return Ok(());
    }
    if git_status_ok(repo_root, &["fetch", "--quiet", "origin", "master"]) {
        report.ok("fetched origin master, so the change set is measured against it");
        Ok(())
    } else {
        report.bad(
            "could not fetch origin master; the change set would be measured against a stale ref",
        );
        report.note("retry first -- a dropped fetch is usually the network, not the remote");
        report.note(
            "to check an unrelated gate meanwhile: PRE_PUSH_SKIP_FETCH=1 ./pre-push-check.sh",
        );
        println!("pre-push-check: 1 failure(s) - master could not be refreshed");
        Err(1)
    }
}

fn diff_name_only(repo_root: &Path, extra_args: &[&str]) -> Vec<String> {
    let mut args = vec!["diff", "--name-only"];
    args.extend_from_slice(extra_args);
    Command::new("git")
        .args(&args)
        .current_dir(repo_root)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Unions the branch diff (when a base resolved), the worktree diff and the
/// index diff, deduplicated and sorted, filtered by `pattern` (an extended
/// regular expression, matching bash's own `grep -E`/`grep` filters).
pub fn changed(repo_root: &Path, base: Option<&str>, pattern: &str) -> Vec<String> {
    let mut files = Vec::new();
    if let Some(base) = base {
        let range = format!("{base}..HEAD");
        files.extend(diff_name_only(repo_root, &[&range]));
    }
    files.extend(diff_name_only(repo_root, &[]));
    files.extend(diff_name_only(repo_root, &["--cached"]));
    files.sort();
    files.dedup();
    let re = Regex::new(pattern).expect("pre-push-check: internal error: invalid pattern");
    files.into_iter().filter(|f| re.is_match(f)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn git_init(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    fn scratch_repo(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "pre-push-check-change-set-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        git_init(&dir, &["init", "-q", "-b", "work"]);
        dir
    }

    #[test]
    fn changed_unions_worktree_and_index_and_filters_by_pattern() {
        let dir = scratch_repo("union");
        fs::write(dir.join("README.md"), "hi\n").unwrap();
        git_init(&dir, &["add", "-A"]);
        git_init(
            &dir,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.com",
                "commit",
                "-q",
                "-m",
                "initial",
            ],
        );
        fs::write(dir.join("a.sh"), "#!/bin/sh\n").unwrap();
        fs::write(dir.join("b.json"), "{}\n").unwrap();
        git_init(&dir, &["add", "a.sh", "b.json"]);

        let files = changed(&dir, None, r"\.sh$");
        assert_eq!(files, vec!["a.sh".to_string()]);
    }

    #[test]
    fn resolve_base_falls_back_to_worktree_only_when_nothing_resolves() {
        let dir = scratch_repo("no-base");
        fs::write(dir.join("README.md"), "hi\n").unwrap();
        git_init(&dir, &["add", "-A"]);
        git_init(
            &dir,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.com",
                "commit",
                "-q",
                "-m",
                "initial",
            ],
        );
        let resolved = resolve_base(&dir);
        assert!(resolved.base.is_none());
        assert!(resolved.label.is_none());
    }
}
