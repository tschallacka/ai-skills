// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::process::ExitCode;
use verify_both_shells::{discover_repo_root, parse_args, process, run, Action, PROGRAM, USAGE};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let action = match parse_args(&args) {
        Ok(action) => action,
        Err(code) => return ExitCode::from(code as u8),
    };
    let keep = match action {
        Action::Help => {
            print!("{USAGE}");
            return ExitCode::from(0);
        }
        Action::Run { keep } => keep,
    };
    let src = match discover_repo_root() {
        Ok(root) => root,
        Err(message) => {
            eprintln!("{PROGRAM}: {message}");
            return ExitCode::from(1);
        }
    };
    let status = run(&src, keep, process::real_legs(&src)).status as u8;
    verify_both_shells::signal::wait_for_signal_exit();
    ExitCode::from(status)
}
