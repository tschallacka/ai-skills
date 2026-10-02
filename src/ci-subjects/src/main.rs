// MODE: DEV
// PACKAGE: PROD
//! ci-subjects — turn ci-scope.sh's crate list into per-subject build
//! flags. Rust port of ci-subjects.sh, reproducing its exact observable
//! behavior: pure, argument-driven decision logic with no git/filesystem
//! dependencies beyond an optional $GITHUB_OUTPUT append.

mod subjects;

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::ExitCode;
use subjects::Subjects;

const PROGRAM: &str = "ci-subjects.sh";

// Captured verbatim via `bash ci-subjects.sh --help`.
const USAGE: &str = "ci-subjects.sh — turn ci-scope.sh's crate list into per-subject build flags.

ci-scope.sh answers \"which crates does this run have to build\". The native
job is organised by SUBJECT, not by crate: one subject owns a group of crates
and a verification the group shares (run the binary, upload one named
artifact). This maps one to the other.

Prints one line per subject, and appends the same to $GITHUB_OUTPUT when set:

  rjq=true|false
  chat=true|false
  plan_crypt=true|false
  planning_commands=true|false
  editor=true|false
  installer=true|false

Usage:
  ci-subjects.sh --scope full|selective|none [--crates \"a b c\"]
  ci-subjects.sh --help

THE DEFAULT IS ALWAYS true, for the same reason ci-scope.sh defaults to full:
a subject wrongly skipped produces a green tick that means \"we did not look\".
So scope=full turns everything on, an unrecognised scope turns everything on,
and only an explicit scope=selective narrows anything. scope=none is the one
case that turns everything off, and ci-scope.sh emits it only when it has
established that no crate changed.

PLANNING COMMANDS IS THE CATCH-ALL. It owns every workspace crate that is not
claimed by a named subject, so a crate added under src/ is covered by default
rather than silently unbuilt until someone remembers to edit this file. That
asymmetry is deliberate: the failure mode of the catch-all is a slower run,
and the failure mode of a whitelist is an untested crate.

Exit codes: 0 always, unless usage is wrong (64).
";

enum Action {
    Help,
    Run { scope: String, crates: String },
}

fn parse_args(args: &[String]) -> Result<Action, i32> {
    let mut scope = String::new();
    let mut crates = String::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scope" => {
                if i + 1 >= args.len() {
                    return Err(64);
                }
                scope = args[i + 1].clone();
                i += 2;
            }
            "--crates" => {
                if i + 1 >= args.len() {
                    return Err(64);
                }
                crates = args[i + 1].clone();
                i += 2;
            }
            "-h" | "--help" => return Ok(Action::Help),
            other => {
                eprintln!("{PROGRAM}: unknown argument: {other}");
                return Err(64);
            }
        }
    }
    Ok(Action::Run { scope, crates })
}

fn print_subjects(subjects: &Subjects) {
    println!("rjq={}", bool_str(subjects.rjq));
    println!("chat={}", bool_str(subjects.chat));
    println!("plan_crypt={}", bool_str(subjects.plan_crypt));
    println!("planning_commands={}", bool_str(subjects.planning_commands));
    println!("editor={}", bool_str(subjects.editor));
    println!("installer={}", bool_str(subjects.installer));
}

fn bool_str(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

fn append_github_output(subjects: &Subjects) {
    let Ok(path) = env::var("GITHUB_OUTPUT") else {
        return;
    };
    if path.is_empty() {
        return;
    }
    // AR-84: a failure to open/write this file never changes the exit
    // code. Fail-open: note the failure on stderr, never propagate it as a
    // non-zero exit or a panic.
    let result = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .and_then(|mut file| {
            writeln!(file, "rjq={}", bool_str(subjects.rjq))?;
            writeln!(file, "chat={}", bool_str(subjects.chat))?;
            writeln!(file, "plan_crypt={}", bool_str(subjects.plan_crypt))?;
            writeln!(
                file,
                "planning_commands={}",
                bool_str(subjects.planning_commands)
            )?;
            writeln!(file, "editor={}", bool_str(subjects.editor))?;
            writeln!(file, "installer={}", bool_str(subjects.installer))
        });
    if let Err(error) = result {
        eprintln!("{PROGRAM}: could not write to GITHUB_OUTPUT ({path}): {error}");
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let action = match parse_args(&args) {
        Ok(action) => action,
        Err(code) => return ExitCode::from(code as u8),
    };
    let (scope, crates) = match action {
        Action::Help => {
            print!("{USAGE}");
            return ExitCode::from(0);
        }
        Action::Run { scope, crates } => (scope, crates),
    };
    let subjects = subjects::decide(&scope, &crates);
    print_subjects(&subjects);
    append_github_output(&subjects);
    ExitCode::from(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_and_crates_flags_are_parsed() {
        let args = vec![
            "--scope".to_string(),
            "selective".to_string(),
            "--crates".to_string(),
            "rjq".to_string(),
        ];
        match parse_args(&args) {
            Ok(Action::Run { scope, crates }) => {
                assert_eq!(scope, "selective");
                assert_eq!(crates, "rjq");
            }
            _ => panic!("expected Action::Run"),
        }
    }

    #[test]
    fn scope_with_no_following_value_is_rejected() {
        let args = vec!["--scope".to_string()];
        assert_eq!(parse_args(&args).err(), Some(64));
    }

    #[test]
    fn an_unknown_flag_is_rejected() {
        let args = vec!["--nonsense".to_string()];
        assert_eq!(parse_args(&args).err(), Some(64));
    }

    #[test]
    fn help_is_recognized() {
        let args = vec!["--help".to_string()];
        assert!(matches!(parse_args(&args), Ok(Action::Help)));
    }
}
