// MODE: DEV
// PACKAGE: PROD
//! blast-radius — report what a change set touches beyond the files it
//! edits. Rust port of blast-radius.sh, reproducing its exact observable
//! behavior: four passes (coupling-registry checks, a PACKAGE-MANIFEST
//! registry check, a base-drift check, and its own argument parsing).

mod changeset;
mod coupling;
mod drift;
mod finding;
mod git;
mod globs;
mod manifest;
mod shell;

use finding::{Level, Line};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

const PROGRAM: &str = "blast-radius.sh";

// Captured verbatim via `bash blast-radius.sh --help`.
const USAGE: &str = "blast-radius.sh — report what a change set touches beyond the files it edits.

This is an integration-safety report, not a correctness check. It catches the
mistakes that come from a file's couplings: a generated artifact left stale, a
new registry that never ships, a branch whose base moved under it. It will not
find a logic bug, and it is not a substitute for exercising the changed path.

Four passes:
  1. freshness  — generated artifacts whose sources moved (fails)
  2. registry   — a new file under planning/ with no manifest row (fails for a
                  runtime registry, warns for anything else)
  3. drift      — commits that touched these files since the base ref, which
                  is how a stale branch silently reverts someone else's fix
  4. contracts  — couplings recorded in coupling.tsv that a human must honour

Usage:
  blast-radius.sh [--base <ref>] [<path> ...]
  blast-radius.sh --help

With no paths, the working tree's own changes are used (staged, unstaged and
untracked). --base defaults to master and only affects the drift pass.

Exit codes: 0 clean, 1 findings, 64 bad invocation, 69 not a git work tree.
";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    ExitCode::from(run(&args) as u8)
}

fn run(args: &[String]) -> i32 {
    if let Some(first) = args.first() {
        if first == "-h" || first == "--help" {
            print!("{USAGE}");
            return 0;
        }
    }

    let (base, paths) = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };

    let repo_root = match git::discover_repo_root() {
        Some(root) => PathBuf::from(root),
        None => {
            eprintln!("{PROGRAM}: not a git work tree");
            return 69;
        }
    };

    let registry = repo_root.join("coupling.tsv");
    if !registry.is_file() {
        eprintln!("{PROGRAM}: coupling.tsv not found");
        return 69;
    }

    let changed = changeset::changed_paths(&repo_root, &paths);
    let changed_count = changeset::non_blank_count(&changed);
    if changed_count == 0 {
        println!("blast-radius: no changes to analyse");
        return 0;
    }
    println!("blast-radius: {changed_count} changed path(s), base {base}");

    let manifest_path = repo_root.join("planning/PACKAGE-MANIFEST.tsv");

    let mut failures = 0usize;
    let mut warnings = 0usize;

    emit_all(
        coupling::run_pass(&repo_root, &registry, &changed),
        &mut failures,
        &mut warnings,
    );
    emit_all(
        manifest::missing_rows(&repo_root, &manifest_path, &changed),
        &mut failures,
        &mut warnings,
    );
    emit_all(
        drift::base_drift(&repo_root, &base, &changed),
        &mut failures,
        &mut warnings,
    );

    println!("blast-radius: {failures} failure(s), {warnings} warning(s)");
    if failures > 0 {
        1
    } else {
        0
    }
}

fn emit_all(lines: Vec<Line>, failures: &mut usize, warnings: &mut usize) {
    for line in lines {
        if line.to_stderr {
            eprintln!("{}", line.text);
        } else {
            println!("{}", line.text);
        }
        match line.level {
            Some(Level::Fail) => *failures += 1,
            Some(Level::Warn) => *warnings += 1,
            None => {}
        }
    }
}

/// Returns `(base, positional_paths)` on success, or `Err(exit_code)` when
/// argument parsing itself fails (64). Matches bash's own loop exactly:
/// `--base <ref>` / `--base=<ref>` / any other `-*` rejected with 64 (the
/// OPPOSITE of generate-portability.sh's silent-ignore) / bare tokens
/// collected as positional paths.
fn parse_args(args: &[String]) -> Result<(String, Vec<String>), i32> {
    let mut base = "master".to_string();
    let mut paths = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--base" {
            if i + 1 >= args.len() {
                eprintln!("{PROGRAM}: --base needs a ref");
                return Err(64);
            }
            base = args[i + 1].clone();
            i += 2;
        } else if let Some(rest) = arg.strip_prefix("--base=") {
            base = rest.to_string();
            i += 1;
        } else if let Some(stripped) = arg.strip_prefix('-') {
            let _ = stripped;
            eprintln!("{PROGRAM}: unknown option: {arg}");
            return Err(64);
        } else {
            paths.push(arg.to_string());
            i += 1;
        }
    }
    Ok((base, paths))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_flag_two_argument_form_is_parsed() {
        let (base, paths) =
            parse_args(&["--base".into(), "origin/main".into(), "a.txt".into()]).unwrap();
        assert_eq!(base, "origin/main");
        assert_eq!(paths, vec!["a.txt".to_string()]);
    }

    #[test]
    fn base_flag_equals_form_is_parsed() {
        let (base, paths) = parse_args(&["--base=origin/main".into()]).unwrap();
        assert_eq!(base, "origin/main");
        assert!(paths.is_empty());
    }

    #[test]
    fn base_with_no_following_argument_is_rejected() {
        assert_eq!(parse_args(&["--base".into()]), Err(64));
    }

    #[test]
    fn an_unknown_option_is_rejected() {
        assert_eq!(parse_args(&["-x".into()]), Err(64));
    }

    #[test]
    fn default_base_is_master_with_no_base_flag() {
        let (base, paths) = parse_args(&["a.txt".into(), "b.txt".into()]).unwrap();
        assert_eq!(base, "master");
        assert_eq!(paths, vec!["a.txt".to_string(), "b.txt".to_string()]);
    }
}
