// MODE: DEV
// PACKAGE: PROD
//! ci-test-scope — decide which shell tests and crate tests a CI run has to
//! execute. Rust port of ci-test-scope.sh, reproducing its exact observable
//! behavior: the same git plumbing ci-scope.sh already uses, the real
//! `run-tests.sh --list-only` as the canonical item list, and the same
//! COVERS-marker matching and fail-open-to-full default on every branch
//! that cannot prove a smaller scope is correct.

mod covers;
mod decide;
mod git;
mod global;
mod list_items;
mod repo_root;

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const PROGRAM: &str = "ci-test-scope.sh";

// Captured verbatim via `bash ci-test-scope.sh --help`.
const USAGE: &str = "ci-test-scope.sh — decide which shell tests and crate tests a CI run has to
execute, the same idea ci-scope.sh already proves for crate BUILDS, applied
to the shell suite ci-scope.sh does not touch (T116).

Prints:
  scope=full|selective
  reason=<one line saying why>
  tests=<space-separated repo-relative test paths and crate dirs to run,
         meaningful only when scope=selective; empty under scope=full,
         where run-tests.sh's own unfiltered discovery is what runs>

THE DEFAULT IS ALWAYS full, for the same reason ci-scope.sh's is: every
branch that cannot prove a smaller scope correct returns full, including
every error path. A selector that narrows when it is confused is worse than
none, because the green tick then means \"we did not look\" while reading as
\"we looked\".

SELECTION. Each test may declare what it covers with a COVERS marker within
the first few header lines (right after the shebang and the MODE marker), a
comment line reading:

  COVERS: <path> <path> ...

A changed path \"hits\" a COVERS entry when it equals the entry or begins with
\"<entry>/\" — a directory entry covers everything under it, a file entry
covers only itself. A test with NO COVERS marker is UNDECLARED, and an
undeclared test ALWAYS runs: selection only ever narrows a test that opted
in, on grounds that test itself stated, never a test nobody has annotated
yet. That is what keeps marking the rest of the suite a pure optimisation
rather than a hazard — an unmarked test costs nothing in speed but nothing
in safety either.

The canonical test/crate list comes from `run-tests.sh --list-only`, not a
second copy of its suites array here: two lists of \"what counts as a test\"
drift, and this selector deciding what NOT to run is exactly the place a
stale list would fail silently.

Usage:
  ci-test-scope.sh [--base REF] [--files-from FILE]
  ci-test-scope.sh --push-to BRANCH
  ci-test-scope.sh --help

  --base REF        what to diff against (default: origin/master, then master)
  --files-from FILE  read the change set from FILE instead of git; one path
                     per line. For tests, so every branch is reachable
                     without inventing commits.
  --push-to BRANCH  this run is a push to BRANCH, not a pull request: decide
                    full and stop, matching ci-scope.sh's own reasoning (a
                    push to master has an empty diff against itself, and
                    selection is a pull-request feature).

Exit codes: 0 always, unless usage is wrong (64).
";

struct Args {
    base: Option<String>,
    files_from: Option<String>,
    push_to: Option<String>,
}

enum Action {
    Help,
    Run(Args),
}

fn parse_args(args: &[String]) -> Result<Action, i32> {
    let mut base = None;
    let mut files_from = None;
    let mut push_to = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--base" => {
                if i + 1 >= args.len() {
                    return Err(64);
                }
                base = Some(args[i + 1].clone());
                i += 2;
            }
            "--files-from" => {
                if i + 1 >= args.len() {
                    return Err(64);
                }
                files_from = Some(args[i + 1].clone());
                i += 2;
            }
            "--push-to" => {
                if i + 1 >= args.len() {
                    return Err(64);
                }
                push_to = Some(args[i + 1].clone());
                i += 2;
            }
            "-h" | "--help" => return Ok(Action::Help),
            other => {
                eprintln!("{PROGRAM}: unknown argument: {other}");
                return Err(64);
            }
        }
    }
    Ok(Action::Run(Args {
        base,
        files_from,
        push_to,
    }))
}

fn non_blank_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect()
}

fn run(args: Args, repo_root: &Path) -> ExitCode {
    if let Some(push_to) = &args.push_to {
        return decide::decide_full(&format!(
            "push to {push_to}: an integration branch always runs the full suite"
        ));
    }

    if !repo_root.is_dir() {
        return decide::decide_full("cannot enter the repository root; refusing to narrow");
    }

    let mut base_ref = args.base.clone();
    let changed_text: String;
    if let Some(files_from) = &args.files_from {
        let resolved = if Path::new(files_from).is_relative() {
            repo_root.join(files_from)
        } else {
            Path::new(files_from).to_path_buf()
        };
        match fs::read_to_string(&resolved) {
            Ok(text) => changed_text = text,
            Err(_) => {
                return decide::decide_full(&format!(
                    "cannot read the change set from {files_from}"
                ))
            }
        }
    } else {
        if !git::git_dir_exists(repo_root) {
            return decide::decide_full("not a git repository");
        }
        if base_ref.is_none() {
            for candidate in ["origin/master", "master"] {
                if git::ref_resolves(repo_root, candidate) {
                    base_ref = Some(candidate.to_string());
                    break;
                }
            }
        }
        let Some(base) = base_ref.clone() else {
            return decide::decide_full("no base ref resolves; refusing to narrow");
        };
        if !git::ref_resolves(repo_root, &base) {
            return decide::decide_full(&format!("base ref {base} does not resolve"));
        }
        let Some(merge_base) = git::merge_base(repo_root, &base) else {
            return decide::decide_full(&format!(
                "no merge base with {base} (shallow clone?); refusing to narrow"
            ));
        };
        match git::diff_name_only(repo_root, &merge_base) {
            Some(text) => changed_text = text,
            None => {
                return decide::decide_full(&format!(
                    "cannot diff {merge_base}..HEAD; refusing to narrow"
                ))
            }
        }
    }

    let changed = non_blank_lines(&changed_text);
    let base_label = base_ref.clone().unwrap_or_else(|| "the base".to_string());
    if changed.is_empty() {
        // Unlike ci-scope.sh (which answers `none` here), this script has no
        // `none` scope at all: nothing to diff against goes full, not
        // selective-on-nothing.
        return decide::decide_full(&format!(
            "nothing differs from {base_label}; nothing to narrow against"
        ));
    }

    if let Some(hit) = global::find_global_hit(&changed) {
        return decide::decide_full(&format!(
            "{hit} changed, which the whole suite execution depends on"
        ));
    }

    if changed.len() > 100 {
        return decide::decide_full(&format!(
            "{} files changed, past the point where selecting pays",
            changed.len()
        ));
    }

    let items = match list_items::list_items(repo_root) {
        Ok(items) => items,
        Err(list_items::ListError::CommandFailed) => {
            return decide::decide_full(
                "run-tests.sh --list-only failed; cannot read the canonical test list",
            )
        }
        Err(list_items::ListError::Empty) => {
            return decide::decide_full(
                "run-tests.sh --list-only listed nothing; refusing to narrow",
            )
        }
    };

    let selection = covers::select(repo_root, &items, &changed);
    if selection.selected.is_empty() {
        return decide::decide_full(
            "selection excluded every test; refusing to trust an empty run",
        );
    }

    decide::decide_selective(
        &format!(
            "{} changed file(s); {} declared test(s) excluded on their own stated grounds",
            changed.len(),
            selection.excluded_count
        ),
        &selection.selected,
    )
}

fn main() -> ExitCode {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let action = match parse_args(&raw_args) {
        Ok(action) => action,
        Err(code) => {
            // Matches goal 21's own AR-85 fix, applied proactively here:
            // bash's own usage() prints the full embedded usage text to
            // stdout on EVERY exit path (a missing flag value, an unknown
            // flag, or -h/--help alike) before exiting -- not only on the
            // help path.
            print!("{USAGE}");
            return ExitCode::from(code as u8);
        }
    };
    let args = match action {
        Action::Help => {
            print!("{USAGE}");
            return ExitCode::from(0);
        }
        Action::Run(args) => args,
    };
    let repo_root = repo_root::discover_repo_root_or_sentinel();
    run(args, &repo_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_three_value_flags_are_parsed() {
        let raw = vec![
            "--base".to_string(),
            "origin/main".to_string(),
            "--files-from".to_string(),
            "/tmp/x".to_string(),
            "--push-to".to_string(),
            "master".to_string(),
        ];
        match parse_args(&raw) {
            Ok(Action::Run(args)) => {
                assert_eq!(args.base.as_deref(), Some("origin/main"));
                assert_eq!(args.files_from.as_deref(), Some("/tmp/x"));
                assert_eq!(args.push_to.as_deref(), Some("master"));
            }
            _ => panic!("expected Action::Run"),
        }
    }

    #[test]
    fn help_is_recognized() {
        assert!(matches!(
            parse_args(&["--help".to_string()]),
            Ok(Action::Help)
        ));
    }

    #[test]
    fn a_value_flag_with_no_following_value_is_rejected() {
        for flag in ["--base", "--files-from", "--push-to"] {
            assert_eq!(
                parse_args(&[flag.to_string()]).err(),
                Some(64),
                "flag: {flag}"
            );
        }
    }

    #[test]
    fn an_unknown_flag_is_rejected() {
        assert_eq!(parse_args(&["--nonsense".to_string()]).err(), Some(64));
    }

    #[test]
    fn a_threshold_flag_is_not_recognized_here() {
        // ci-test-scope.sh has no --threshold concept at all, unlike
        // ci-scope.sh -- confirm it is rejected like any other unknown flag.
        assert_eq!(parse_args(&["--threshold".to_string()]).err(), Some(64));
    }

    #[test]
    fn non_blank_lines_drops_whitespace_only_lines() {
        let out = non_blank_lines("a\n\n  \nb\n");
        assert_eq!(out, vec!["a".to_string(), "b".to_string()]);
    }
}
