// MODE: DEV
// PACKAGE: PROD
mod error;
mod extract;
mod forge;
mod gh;
mod glab;
mod target;

use error::CliError;
use forge::{detect_forge, Forge, RealProbe};
use std::env;
use std::process::{Command, ExitCode};

const DEFAULT_REPO_SLUG: &str = "tschallacka/ai-skills";

// The usage block above `set -euo pipefail` in ci-failures.sh's own header
// comment, verbatim (the bash script derives this at runtime via an awk
// one-liner stripping the leading `# `; this port embeds the identical text
// as a literal constant instead of re-deriving it from a comment, per W73's
// own Instructions).
const USAGE: &str = r#"ci-failures - what actually failed in a CI run or pipeline, from a run/
pipeline id, a PR/MR number, or a branch. Works against GitHub (gh) and
GitLab (glab), detecting which one this repository's remote calls for.

Usage:
  ci-failures.sh                  # the newest run/pipeline for the current branch
  ci-failures.sh 33894205595      # a run/pipeline id
  ci-failures.sh 47               # a PR/MR number
  ci-failures.sh pr/47            # a PR/MR number, unambiguously
  ci-failures.sh fix/some-branch  # the newest run/pipeline for a branch
  ci-failures.sh <target> --raw DIR  # keep the whole log of each failing job
  ci-failures.sh <target> --all   # every job, not only the failing ones

It prints, per failing job, the lines that identify the failure: a suite's
own `Failed:` summary, cargo's `test result:`, every panic with the lines
after it, GitHub's `##[error]` annotations, and this repository's own FAIL/
portability findings. `--raw DIR` additionally writes each job's full,
de-escaped log to DIR so a long screen dump can be read whole.

WHY THIS EXISTS. Reading a failing run by hand is six steps -- list the jobs,
find the failing ids, fetch each log through the API, allow the escape
sequences, strip the CR and the ANSI, then search for the interesting lines
-- and on GitHub it was done eight times in one session before anyone wrote
it down. None of that is specific to one repository or one forge, which is
why this is a skill rather than a maintainer note: which API answers while a
run is still going, that GitHub's raw-log endpoint refuses a body carrying
colour codes without an explicit flag, that the codes then need stripping
with a literal ESC because the escape form is a GNU sed extension, and which
lines in a log actually identify a failure as opposed to merely mentioning
one.

FORGE DETECTION. The remote this repository's origin points at decides gh or
glab, not a flag: a github.com remote uses gh, a gitlab.com or self-hosted
GitLab remote uses glab, and CI_FAILURES_FORGE=gh|glab overrides the guess
outright for a remote neither pattern recognises. Either way the tool chosen
is named on the first line of output: a silent choice between two APIs with
different failure vocabularies is exactly the kind of guess this script
exists to make instead of a person, and the person reading the output still
needs to know which guess was made.

BOTH FORGES, ONE VOCABULARY. GitHub calls it a run containing jobs, with a
logs endpoint; GitLab calls it a pipeline containing jobs, with a trace
endpoint instead, and its statuses and failure markup are not GitHub's. The
<target> forms above (a bare number, pr/N, a branch, nothing) mean the same
thing on both: pr/N addresses a pull request on GitHub and a merge request
on GitLab, and a bare number is read the same way, disambiguated by
magnitude (see resolve_run below), on both.

VERIFIED DIFFERENTLY. The gh path is exercised against this repository's own
real GitHub Actions runs. The glab path is written against GitLab's
documented REST API v4 (pipelines, jobs, trace) -- the same stable,
versioned surface the gh path prefers over gh's own formatted subcommands,
and for the same reason -- but this repository has no GitLab remote to run
it against, so it is exercised in tests against a stubbed glab rather than a
live one. Treat a first real run against a GitLab project as the one
still-missing verification step, not as settled.
"#;

#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    help: bool,
    target: String,
    raw_dir: Option<String>,
    want_all: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum ParsedArgs {
    Help,
    BadUsage(String),
    Run(Args),
}

/// Pure argument parsing: never exits the process, so tests can assert on
/// the parse result directly. Mirrors ci-failures.sh's own parsing exactly,
/// including its own real quirk (verified directly against the bash source,
/// not assumed): position 1 is ALWAYS taken as the target, whatever it is --
/// `target="${1:-}"` -- with the sole exception of literal -h/--help. This
/// means `ci-failures.sh --all` with no real target sets target="--all" (a
/// bogus value) and leaves want_all false, since the flag loop only sees
/// $2 onward; it is not an error in bash (--all is never reached as a flag)
/// and is not "fixed" here, since this port's scope is behavioral parity.
fn parse_args(argv: &[String]) -> ParsedArgs {
    let mut it = argv.iter();
    let mut args = Args::default();

    let target = it.next().cloned().unwrap_or_default();
    if target == "-h" || target == "--help" {
        return ParsedArgs::Help;
    }
    args.target = target;

    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--raw" => match it.next() {
                Some(value) => args.raw_dir = Some(value.clone()),
                None => return ParsedArgs::BadUsage("ci-failures: --raw needs a directory".into()),
            },
            _ if arg.starts_with("--raw=") => {
                args.raw_dir = Some(arg["--raw=".len()..].to_string());
            }
            "--all" => args.want_all = true,
            other => return ParsedArgs::BadUsage(format!("ci-failures: unknown option: {other}")),
        }
    }
    ParsedArgs::Run(args)
}

fn origin_remote_url() -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!url.is_empty()).then_some(url)
}

fn run() -> Result<(), CliError> {
    let argv: Vec<String> = env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        ParsedArgs::Help => {
            print!("{USAGE}");
            return Ok(());
        }
        ParsedArgs::BadUsage(message) => return Err(CliError::bad_usage(message)),
        ParsedArgs::Run(args) => args,
    };

    let repo_override = env::var("CI_FAILURES_REPO").ok();
    let forge_override = env::var("CI_FAILURES_FORGE").ok();
    let remote_url = origin_remote_url();

    let forge = detect_forge(forge_override.as_deref(), remote_url.as_deref(), &RealProbe)?;

    match forge {
        Forge::Gh => {
            let repo_slug = repo_override.unwrap_or_else(|| DEFAULT_REPO_SLUG.to_string());
            gh::run(
                &repo_slug,
                &args.target,
                args.raw_dir.as_deref(),
                args.want_all,
            )
        }
        Forge::Glab => glab::run(
            repo_override.as_deref(),
            remote_url.as_deref(),
            &args.target,
            args.raw_dir.as_deref(),
            args.want_all,
        ),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error.message);
            ExitCode::from(error.code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_arguments_means_the_current_branch_with_no_raw_dir_or_all() {
        assert_eq!(
            parse_args(&args(&[])),
            ParsedArgs::Run(Args {
                help: false,
                target: String::new(),
                raw_dir: None,
                want_all: false,
            })
        );
    }

    #[test]
    fn help_is_recognized_in_either_spelling() {
        assert_eq!(parse_args(&args(&["--help"])), ParsedArgs::Help);
        assert_eq!(parse_args(&args(&["-h"])), ParsedArgs::Help);
    }

    #[test]
    fn a_bare_positional_is_the_target() {
        assert_eq!(
            parse_args(&args(&["pr/47"])),
            ParsedArgs::Run(Args {
                help: false,
                target: "pr/47".to_string(),
                raw_dir: None,
                want_all: false,
            })
        );
    }

    #[test]
    fn raw_and_all_are_accepted_after_an_explicit_target() {
        assert_eq!(
            parse_args(&args(&["47", "--raw", "/tmp/logs", "--all"])),
            ParsedArgs::Run(Args {
                help: false,
                target: "47".to_string(),
                raw_dir: Some("/tmp/logs".to_string()),
                want_all: true,
            })
        );
    }

    // A --raw=DIR or --all with no real target is a real bash quirk, verified
    // directly against the source, not assumed: position 1 is ALWAYS the
    // target (target="${1:-}"), so the flag itself becomes a bogus target
    // string and the flag loop never runs at all (nothing left after shift).
    #[test]
    fn raw_equals_form_with_no_explicit_target_becomes_the_bogus_target() {
        assert_eq!(
            parse_args(&args(&["--raw=/tmp/logs"])),
            ParsedArgs::Run(Args {
                help: false,
                target: "--raw=/tmp/logs".to_string(),
                raw_dir: None,
                want_all: false,
            })
        );
    }

    #[test]
    fn raw_missing_its_value_is_refused() {
        match parse_args(&args(&["47", "--raw"])) {
            ParsedArgs::BadUsage(message) => assert!(message.contains("--raw needs a directory")),
            other => panic!("expected BadUsage, got {other:?}"),
        }
    }

    #[test]
    fn an_unknown_flag_is_refused_by_name() {
        match parse_args(&args(&["47", "--bogus"])) {
            ParsedArgs::BadUsage(message) => assert!(message.contains("unknown option: --bogus")),
            other => panic!("expected BadUsage, got {other:?}"),
        }
    }

    #[test]
    fn all_with_no_explicit_target_becomes_the_bogus_target_and_want_all_stays_false() {
        assert_eq!(
            parse_args(&args(&["--all"])),
            ParsedArgs::Run(Args {
                help: false,
                target: "--all".to_string(),
                raw_dir: None,
                want_all: false,
            })
        );
    }
}
