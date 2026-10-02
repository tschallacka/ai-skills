// MODE: DEV
// PACKAGE: PROD
//! The single exit point every decision funnels through: print the
//! three-line `scope=`/`reason=`/`tests=` shape to stdout, optionally
//! append the same to `$GITHUB_OUTPUT`, then exit 0 -- always. There is no
//! `scope=none` here (unlike ci-scope.sh): only `full` and `selective`
//! exist in the real bash original. AR-84 (applied proactively, from goal
//! 20's own finding): a failure to write `$GITHUB_OUTPUT` must never change
//! the exit code or panic, only optionally note the failure on stderr.

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::ExitCode;

const PROGRAM: &str = "ci-test-scope.sh";

pub fn decide_full(reason: &str) -> ExitCode {
    decide("full", reason, &[])
}

pub fn decide_selective(reason: &str, tests: &[String]) -> ExitCode {
    decide("selective", reason, tests)
}

fn decide(scope: &str, reason: &str, tests: &[String]) -> ExitCode {
    let tests_joined = tests.join(" ");
    println!("scope={scope}");
    println!("reason={reason}");
    println!("tests={tests_joined}");
    append_github_output(scope, reason, &tests_joined);
    ExitCode::from(0)
}

fn append_github_output(scope: &str, reason: &str, tests_joined: &str) {
    let Ok(path) = env::var("GITHUB_OUTPUT") else {
        return;
    };
    if path.is_empty() {
        return;
    }
    let result = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .and_then(|mut file| {
            writeln!(file, "scope={scope}")?;
            writeln!(file, "reason={reason}")?;
            writeln!(file, "tests={tests_joined}")
        });
    if let Err(error) = result {
        eprintln!("{PROGRAM}: could not write to GITHUB_OUTPUT ({path}): {error}");
    }
}
