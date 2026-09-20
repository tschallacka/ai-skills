// MODE: DEV
// PACKAGE: PROD
//! planning-client -- the CLI-over-protocol client for planning-server,
//! mirroring ai-text-editor's own CLI/server split. Every guarded
//! subcommand needs the document's current revision: pass --revision
//! explicitly (matching ai-text-editor's own explicit-revision-on-mutating-
//! call convention), or omit it and this client reads the target document
//! itself first to obtain it -- one extra round trip, but no separate
//! "read" step the caller must remember to run first.

use planning_server::protocol::{decode_response, encode_request, Request, Response};
use planning_server::transport::Stream;
use std::io::{BufRead, BufReader, Write};
use std::process::ExitCode;

const USAGE: &str = "planning-client -- CLI-over-protocol client for planning-server\n\nUsage:\n  planning-client read-plan-document <plan-dir> <document-id> [--view VIEW]\n  planning-client read-work-unit <plan-dir> <unit-id>\n  planning-client update-step <plan-dir> <goal> <step> <status> [--revision REV]\n  planning-client add-work-unit <plan-dir> <id> <type> <file> <scope> <subscope> <change> <depends-on> <goal> <step> [--revision REV]\n  planning-client set-review-status <plan-dir> <status> [--revision REV]\n  planning-client set-testing-requirement <plan-dir> <goal> <yes|no> <rationale> [--revision REV]\n  planning-client validate-plan <plan-dir> [--complete]\n  planning-client --help\n\nA guarded subcommand (update-step, add-work-unit, set-review-status,\nset-testing-requirement) reads the document's current revision itself when\n--revision is not given. Exit codes: 0 success, 64 bad usage, 65 stale\nrevision (re-read and retry), 69 cannot connect to the server, 70 the\nserver reported an error.\n";

fn usage(code: u8) -> ExitCode {
    if code == 0 {
        print!("{USAGE}");
    } else {
        eprint!("{USAGE}");
    }
    ExitCode::from(code)
}

fn send(request: &Request) -> Result<Response, String> {
    let socket_path = planning_server::endpoint::socket_path();
    let mut stream = Stream::connect(&socket_path).map_err(|error| {
        format!(
            "cannot connect to planning-server at {}: {error}",
            socket_path.display()
        )
    })?;
    let line = encode_request(request);
    writeln!(stream, "{line}").map_err(|error| format!("cannot write to server: {error}"))?;
    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .map_err(|error| format!("cannot read from server: {error}"))?;
    if response_line.is_empty() {
        return Err("server closed the connection with no response".to_string());
    }
    decode_response(response_line.trim_end())
}

/// Reads a document's current revision through the server itself (a plain
/// ReadPlanDocument call), so a caller that did not supply --revision still
/// submits a real, current guard rather than an empty or invented one.
fn current_revision(plan_dir: &str, document_id: &str) -> Result<String, String> {
    match send(&Request::ReadPlanDocument {
        plan_dir: plan_dir.to_string(),
        document_id: document_id.to_string(),
        view: Some("full".to_string()),
    })? {
        Response::Document { revision, .. } => Ok(revision),
        Response::Error { message } => Err(message),
        other => Err(format!(
            "expected a Document response while reading the current revision, got {other:?}"
        )),
    }
}

fn take_flag_value(args: &mut Vec<String>, flag: &str) -> Option<String> {
    let position = args.iter().position(|arg| arg == flag)?;
    if position + 1 >= args.len() {
        return None;
    }
    args.remove(position); // the flag itself
    Some(args.remove(position)) // its value, now at the same index
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    if let Some(position) = args.iter().position(|arg| arg == flag) {
        args.remove(position);
        true
    } else {
        false
    }
}

fn print_response(response: &Response) -> ExitCode {
    match response {
        Response::Document { content, revision } => {
            println!("{content}");
            eprintln!("revision: {revision}");
            ExitCode::SUCCESS
        }
        Response::Written { revision } => {
            println!("written; new revision: {revision}");
            ExitCode::SUCCESS
        }
        Response::Validated { passed, report } => {
            print!("{report}");
            if *passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Response::Stale { expected, actual } => {
            eprintln!(
                "planning-client: stale revision: the document is at revision {actual}; this request supplied {expected} - re-read and retry with the current revision"
            );
            ExitCode::from(65)
        }
        Response::Error { message } => {
            eprintln!("planning-client: {message}");
            ExitCode::from(70)
        }
        Response::Unimplemented => {
            eprintln!("planning-client: the server has no handler for this operation yet");
            ExitCode::from(70)
        }
    }
}

fn run() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        return usage(0);
    }
    if args.is_empty() {
        return usage(64);
    }
    let subcommand = args.remove(0);

    let request = match subcommand.as_str() {
        "read-plan-document" => {
            let view = take_flag_value(&mut args, "--view");
            if args.len() != 2 {
                return usage(64);
            }
            Request::ReadPlanDocument {
                plan_dir: args[0].clone(),
                document_id: args[1].clone(),
                view,
            }
        }
        "read-work-unit" => {
            if args.len() != 2 {
                return usage(64);
            }
            Request::ReadWorkUnit {
                plan_dir: args[0].clone(),
                unit_id: args[1].clone(),
            }
        }
        "update-step" => {
            // --revision is stripped BEFORE the positional count check: it is
            // an optional flag anywhere on the line, not one of the four
            // required positionals, and checking the count first (counting
            // the flag and its value as if they were positionals) refused
            // every call that supplied --revision at all (found by manual
            // end-to-end testing, not a unit test -- fixed here rather than
            // left as a real, user-visible bug).
            let explicit_revision = take_flag_value(&mut args, "--revision");
            if args.len() != 4 {
                return usage(64);
            }
            let (plan_dir, goal, step, status) = (
                args[0].clone(),
                args[1].clone(),
                args[2].clone(),
                args[3].clone(),
            );
            // update-step's own write target is the goal's progress.md, not
            // the step document itself (confirmed directly against the real
            // standalone binary) -- the guard must read/check that file.
            let document_id = format!("goal-progress:{goal}");
            let revision = match explicit_revision {
                Some(revision) => revision,
                None => match current_revision(&plan_dir, &document_id) {
                    Ok(revision) => revision,
                    Err(message) => {
                        eprintln!("planning-client: {message}");
                        return ExitCode::from(70);
                    }
                },
            };
            Request::UpdateStep {
                plan_dir,
                goal,
                step,
                status,
                revision,
            }
        }
        "add-work-unit" => {
            let explicit_revision = take_flag_value(&mut args, "--revision");
            if args.len() != 10 {
                return usage(64);
            }
            let plan_dir = args[0].clone();
            let revision = match explicit_revision {
                Some(revision) => revision,
                None => match current_revision(&plan_dir, "inventory") {
                    Ok(revision) => revision,
                    Err(message) => {
                        eprintln!("planning-client: {message}");
                        return ExitCode::from(70);
                    }
                },
            };
            Request::AddWorkUnit {
                plan_dir,
                id: args[1].clone(),
                unit_type: args[2].clone(),
                file: args[3].clone(),
                scope: args[4].clone(),
                subscope: args[5].clone(),
                change: args[6].clone(),
                depends_on: args[7].clone(),
                goal: args[8].clone(),
                step: args[9].clone(),
                revision,
            }
        }
        "set-review-status" => {
            let explicit_revision = take_flag_value(&mut args, "--revision");
            if args.len() != 2 {
                return usage(64);
            }
            let plan_dir = args[0].clone();
            let revision = match explicit_revision {
                Some(revision) => revision,
                None => match current_revision(&plan_dir, "plan") {
                    Ok(revision) => revision,
                    Err(message) => {
                        eprintln!("planning-client: {message}");
                        return ExitCode::from(70);
                    }
                },
            };
            Request::SetReviewStatus {
                plan_dir,
                status: args[1].clone(),
                revision,
            }
        }
        "set-testing-requirement" => {
            let explicit_revision = take_flag_value(&mut args, "--revision");
            if args.len() != 4 {
                return usage(64);
            }
            let (plan_dir, goal, required_text, rationale) = (
                args[0].clone(),
                args[1].clone(),
                args[2].clone(),
                args[3].clone(),
            );
            let required = match required_text.as_str() {
                "yes" => true,
                "no" => false,
                _ => return usage(64),
            };
            let document_id = format!("goal:{goal}");
            let revision = match explicit_revision {
                Some(revision) => revision,
                None => match current_revision(&plan_dir, &document_id) {
                    Ok(revision) => revision,
                    Err(message) => {
                        eprintln!("planning-client: {message}");
                        return ExitCode::from(70);
                    }
                },
            };
            Request::SetTestingRequirement {
                plan_dir,
                goal,
                required,
                rationale,
                revision,
            }
        }
        "validate-plan" => {
            let complete = take_flag(&mut args, "--complete");
            if args.len() != 1 {
                return usage(64);
            }
            Request::ValidatePlan {
                plan_dir: args[0].clone(),
                complete,
            }
        }
        _ => return usage(64),
    };

    match send(&request) {
        Ok(response) => print_response(&response),
        Err(message) => {
            eprintln!("planning-client: {message}");
            ExitCode::from(69)
        }
    }
}

fn main() -> ExitCode {
    run()
}
