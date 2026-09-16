// MODE: DEV
// PACKAGE: PROD
use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};

/// One shell leg: a label for the report, and a function BUILDING (not yet
/// spawning) its own real `Command` given the worktree to run in.
pub struct Leg {
    pub label: &'static str,
    pub build: Box<dyn Fn(&Path) -> Command>,
}

/// AR-77 (refined during implementation to a more robust design than a
/// shell-string wrapper): reproduces `>"$log" 2>&1`'s merged-stream
/// semantics NATIVELY -- both stdout and stderr are redirected to clones of
/// the SAME open file, so they share one underlying kernel file description
/// and interleave in true chronological order exactly like a real shell
/// redirect, with no shell-string quoting risk. The log file is durable on
/// disk incrementally, which also matters if a signal interrupts mid-leg.
pub fn run_leg_to_file(command: &mut Command, log_path: &Path) -> std::io::Result<()> {
    let out_file = File::create(log_path)?;
    let err_file = out_file.try_clone()?;
    command
        .stdout(Stdio::from(out_file))
        .stderr(Stdio::from(err_file));
    let _status = command.status()?;
    Ok(())
}

/// AR-81: the real leg-2 argv as a PURE function, separate from the
/// `Command` built from it, so its exact shape is unit-testable with zero
/// subprocess cost.
pub fn nix_leg_argv(src: &Path) -> Vec<String> {
    vec![
        "nix".to_string(),
        "develop".to_string(),
        src.display().to_string(),
        "--command".to_string(),
        "bash32-run-tests".to_string(),
    ]
}

/// The two real production legs: bash 5.3 (the invoking shell's own
/// `./run-tests.sh`) and bash 3.2 (via `nix develop <src> --command
/// bash32-run-tests`).
pub fn real_legs(src: &Path) -> [Leg; 2] {
    let src_owned = src.to_path_buf();
    [
        Leg {
            label: "bash 5.3",
            build: Box::new(|wt: &Path| {
                let mut c = Command::new("./run-tests.sh");
                c.current_dir(wt);
                c
            }),
        },
        Leg {
            label: "bash 3.2",
            build: Box::new(move |wt: &Path| {
                let argv = nix_leg_argv(&src_owned);
                let mut c = Command::new(&argv[0]);
                c.args(&argv[1..]).current_dir(wt);
                c
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nix_leg_argv_matches_the_real_command_shape() {
        let src = Path::new("/repo/root");
        assert_eq!(
            nix_leg_argv(src),
            vec![
                "nix",
                "develop",
                "/repo/root",
                "--command",
                "bash32-run-tests"
            ]
        );
    }

    #[test]
    fn run_leg_to_file_preserves_real_chronological_stdout_stderr_interleaving() {
        let log_path = std::env::temp_dir().join(format!(
            "verify-both-shells-merge-test-{}.log",
            std::process::id()
        ));
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg("printf 'out1\\n'; printf 'err1\\n' >&2; printf 'out2\\n'");
        run_leg_to_file(&mut command, &log_path).unwrap();
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert_eq!(content, "out1\nerr1\nout2\n");
        let _ = std::fs::remove_file(&log_path);
    }
}
