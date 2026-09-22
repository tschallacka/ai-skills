// MODE: DEV
// PACKAGE: PROD
//! ci-scope — decide how much of the workspace a CI run has to build. Rust
//! port of ci-scope.sh, reproducing its exact observable behavior: the same
//! git plumbing, the same cargo-metadata-derived reverse-dependency closure,
//! the same derived threshold, and the same fail-open-to-full default on
//! every branch that cannot prove a smaller scope is correct.

mod closure;
mod crates;
mod decide;
mod git;
mod global;
mod metadata;
mod repo_root;
mod threshold;

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const PROGRAM: &str = "ci-scope.sh";

// Captured verbatim via `bash ci-scope.sh --help`.
const USAGE: &str = "ci-scope.sh — decide how much of the workspace a CI run has to build.

Prints three lines, and writes the same to $GITHUB_OUTPUT when it is set:

  scope=full|selective|none
  reason=<one line saying why>
  crates=<space-separated crate names, empty unless scope=selective>

full       build and test every workspace member on every target
selective  build and test only the named crates
none       no crate needs building; the shell and packaging gates still run

THE DEFAULT IS ALWAYS full. Every branch that cannot prove a smaller scope is
correct returns full, including every error path: no merge base, a shallow
clone, cargo metadata failing, an unreadable change set. A selector that
narrows the run when it is confused is worse than no selector, because the
green tick then means \"we did not look\" while reading as \"we looked\".

Selection is crate changes UNIONED WITH THEIR DEPENDENTS. Building only the
changed crate is wrong: chat-proto has two dependents, planning-core has
twenty-four, and a library change that compiles in isolation can still break
every consumer. The reverse edges come from cargo metadata, so the graph is
the real one rather than a list someone maintains by hand.

Usage:
  ci-scope.sh [--base REF] [--files-from FILE] [--threshold N]
  ci-scope.sh --push-to BRANCH
  ci-scope.sh --help

  --base REF        what to diff against (default: origin/master, then master)
  --files-from FILE  read the change set from FILE instead of git; one path
                     per line. For tests, so every branch is reachable
                     without inventing commits.
  --threshold N     override the derived threshold (see below)
  --push-to BRANCH  this run is a push to BRANCH, not a pull request: decide
                    full and stop. On a push to master, HEAD *is*
                    origin/master, so the merge base is HEAD and the diff is
                    empty -- the selector reported `scope=none` and master
                    went green having compiled nothing. Selection is a pull
                    request feature; an integration branch stays exhaustive.

THE THRESHOLD IS DERIVED, NOT A CONSTANT. A closure bigger than a quarter of
the workspace goes full: ceil(members / 4), with a floor of 5 so a small
workspace does not end up with a threshold of 1.

A fixed number would rot. When this was written the workspace had 78 members
and the closure sizes were 25, 13, 10, 10, 7, 6, 5, 3, then 2 for eleven more
libraries and 1 for the 69 leaf crates. Any hand-picked value between 10 and
13 behaved identically, and so did anything from 13 to 24 — so the number
looked meaningful while being arbitrary inside a gap, and would have silently
changed meaning as crates were added.

The obvious alternative is to find the widest gap in that distribution and
split there, which is what a person does by eye. It is rejected deliberately:
the widest gap MOVES. One new crate with a mid-sized closure relocates it, and
the policy flips with no edit and no announcement. A ratio is monotonic — it
only ever moves when the workspace size moves, and it moves predictably.

The ratio is a cap on RISK as much as on cost. Pure economics would put the
crossover much higher, since the fixed overhead of a run is paid either way;
but a large closure means a broad change, and a broad change is exactly where
selection is most likely to miss a path cargo cannot see — a shell script, a
generated library, a packaged file. Capping well below the cost crossover
buys back that uncertainty.

Exit codes: 0 always, unless usage is wrong (64). A decision is not a failure.
";

struct Args {
    base: Option<String>,
    files_from: Option<String>,
    threshold: Option<String>,
    push_to: Option<String>,
}

enum Action {
    Help,
    Run(Args),
}

fn parse_args(args: &[String]) -> Result<Action, i32> {
    let mut base = None;
    let mut files_from = None;
    let mut threshold = None;
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
            "--threshold" => {
                if i + 1 >= args.len() {
                    return Err(64);
                }
                threshold = Some(args[i + 1].clone());
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
        threshold,
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
            "push to {push_to}: an integration branch is always built in full"
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
        return decide::decide_none(&format!("nothing differs from {base_label}"));
    }

    if let Some(hit) = global::find_global_hit(&changed) {
        return decide::decide_full(&format!("{hit} changed, which can alter every crate"));
    }

    if changed.len() > 100 {
        return decide::decide_full(&format!(
            "{} files changed, past the point where selecting pays",
            changed.len()
        ));
    }

    let changed_crates = crates::extract_changed_crates(&changed);
    if changed_crates.is_empty() {
        return decide::decide_none(&format!("no crate under src/ differs from {base_label}"));
    }

    let workspace_metadata = match metadata::read(repo_root) {
        Ok(m) => m,
        Err(metadata::MetadataError::CargoNotOnPath) => {
            return decide::decide_full("cargo is not on PATH; cannot compute the dependency graph")
        }
        Err(metadata::MetadataError::CommandFailed) => {
            return decide::decide_full(
                "cargo metadata failed; cannot compute the dependency graph",
            )
        }
        Err(metadata::MetadataError::Empty) => {
            return decide::decide_full(
                "cargo metadata produced nothing; cannot compute the dependency graph",
            )
        }
        Err(metadata::MetadataError::Unparseable) => {
            return decide::decide_full("cannot read the workspace graph from cargo metadata")
        }
    };
    let edges = metadata::build_edges(&workspace_metadata);
    let members_total = workspace_metadata.packages.len() as u64;

    let selected = closure::reverse_closure(&changed_crates, &edges);
    let selected_count = selected.len() as u64;
    let changed_count = changed_crates.len() as u64;

    if members_total == 0 {
        return decide::decide_full("cannot read the workspace member count; refusing to narrow");
    }

    let divisor = threshold::coerce_divisor(&env::var("CI_SCOPE_DIVISOR").unwrap_or_default(), 4);
    let floor = threshold::coerce_floor(&env::var("CI_SCOPE_FLOOR").unwrap_or_default(), 5);
    let resolved = threshold::resolve(
        &env::var("CI_SCOPE_THRESHOLD").unwrap_or_default(),
        args.threshold.as_deref(),
        members_total,
        divisor,
        floor,
    );

    if selected_count > resolved.value {
        return decide::decide_full(&format!(
            "{changed_count} changed crate(s) fan out to {selected_count} of {members_total}, past the threshold ({})",
            resolved.label
        ));
    }

    let selected_vec: Vec<String> = selected.into_iter().collect();
    decide::decide_selective(
        &format!(
            "{changed_count} changed crate(s) plus dependents = {selected_count} of {members_total}, within the threshold ({})",
            resolved.label
        ),
        &selected_vec,
    )
}

fn main() -> ExitCode {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let action = match parse_args(&raw_args) {
        Ok(action) => action,
        Err(code) => {
            // AR-85: the full usage text prints to stdout on EVERY exit
            // path (a missing flag value, an unknown flag, or -h/--help
            // alike) before exiting -- not only on the help path.
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
    fn all_four_value_flags_are_parsed() {
        let raw = vec![
            "--base".to_string(),
            "origin/main".to_string(),
            "--files-from".to_string(),
            "/tmp/x".to_string(),
            "--threshold".to_string(),
            "9".to_string(),
            "--push-to".to_string(),
            "master".to_string(),
        ];
        match parse_args(&raw) {
            Ok(Action::Run(args)) => {
                assert_eq!(args.base.as_deref(), Some("origin/main"));
                assert_eq!(args.files_from.as_deref(), Some("/tmp/x"));
                assert_eq!(args.threshold.as_deref(), Some("9"));
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
        for flag in ["--base", "--files-from", "--threshold", "--push-to"] {
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
    fn non_blank_lines_drops_whitespace_only_lines() {
        let out = non_blank_lines("a\n\n  \nb\n");
        assert_eq!(out, vec!["a".to_string(), "b".to_string()]);
    }
}
