// MODE: DEV
// PACKAGE: PROD
//! The single exit point every decision funnels through: print the
//! three-line `scope=`/`reason=`/`crates=` shape to stdout, optionally
//! append the same to `$GITHUB_OUTPUT`, then exit 0 -- always. AR-84
//! (applied proactively, from goal 20's own finding): the real script's own
//! final statement is an unconditional `exit 0`; a failure to write
//! `$GITHUB_OUTPUT` must never change the exit code or panic, only
//! optionally note the failure on stderr.

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::ExitCode;

const PROGRAM: &str = "ci-scope.sh";

pub fn decide_full(reason: &str) -> ExitCode {
    decide("full", reason, &[])
}

pub fn decide_none(reason: &str) -> ExitCode {
    decide("none", reason, &[])
}

pub fn decide_selective(reason: &str, crates: &[String]) -> ExitCode {
    decide("selective", reason, crates)
}

fn decide(scope: &str, reason: &str, crates: &[String]) -> ExitCode {
    let crates_joined = crates.join(" ");
    println!("scope={scope}");
    println!("reason={reason}");
    println!("crates={crates_joined}");
    append_github_output(scope, reason, &crates_joined);
    ExitCode::from(0)
}

fn append_github_output(scope: &str, reason: &str, crates_joined: &str) {
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
            writeln!(file, "crates={crates_joined}")
        });
    if let Err(error) = result {
        eprintln!("{PROGRAM}: could not write to GITHUB_OUTPUT ({path}): {error}");
    }
}
