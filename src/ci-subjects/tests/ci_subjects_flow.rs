// MODE: DEV
// Real-subprocess integration coverage for the compiled ci-subjects binary.
//
// AR-82: every spawned invocation below explicitly controls its own
// GITHUB_OUTPUT environment variable (either removed or pointed at a
// scratch file) rather than inheriting the test process's own ambient
// value -- real CI sets GITHUB_OUTPUT for every step, so a test that
// inherited it unmodified could append stray lines into that job's own
// real output file when this crate's own test suite runs inside actual CI.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ci-subjects"))
        .args(args)
        .env_remove("GITHUB_OUTPUT")
        .output()
        .unwrap()
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn combined_of(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn help_prints_the_embedded_text_and_exits_zero() {
    let output = run(&["--help"]);
    assert!(output.status.success(), "{}", combined_of(&output));
    assert!(
        stdout_of(&output).contains("turn ci-scope.sh's crate list into per-subject build flags")
    );
}

#[test]
fn an_unknown_flag_exits_64() {
    let output = run(&["--nonsense"]);
    assert_eq!(output.status.code(), Some(64));
    assert!(
        combined_of(&output).contains("unknown argument: --nonsense"),
        "{}",
        combined_of(&output)
    );
}

#[test]
fn scope_with_no_following_value_exits_64() {
    let output = run(&["--scope"]);
    assert_eq!(output.status.code(), Some(64));
}

#[test]
fn crates_with_no_following_value_exits_64() {
    let output = run(&["--crates"]);
    assert_eq!(output.status.code(), Some(64));
}

#[test]
fn github_output_is_appended_when_set() {
    let scratch = std::env::temp_dir().join(format!(
        "ci-subjects-flow-github-output-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&scratch);
    let output = Command::new(env!("CARGO_BIN_EXE_ci-subjects"))
        .args(["--scope", "selective", "--crates", "rjq"])
        .env("GITHUB_OUTPUT", &scratch)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", combined_of(&output));
    let appended = fs::read_to_string(&scratch).expect("GITHUB_OUTPUT should have been written");
    assert_eq!(appended, stdout_of(&output));
    let _ = fs::remove_file(&scratch);
}

#[test]
fn no_github_output_set_leaves_stdout_unaffected() {
    let output = run(&["--scope", "full"]);
    assert!(output.status.success(), "{}", combined_of(&output));
    assert!(stdout_of(&output).contains("rjq=true"));
}

#[test]
fn an_unwritable_github_output_still_exits_zero() {
    // AR-84: real bash's own unconditional `exit 0` means a GITHUB_OUTPUT
    // write failure never changes the exit code.
    let bad_path = "/definitely/does/not/exist/output-file";
    let output = Command::new(env!("CARGO_BIN_EXE_ci-subjects"))
        .args(["--scope", "full"])
        .env("GITHUB_OUTPUT", bad_path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", combined_of(&output));
    assert!(
        stdout_of(&output).contains("rjq=true"),
        "stdout must still be printed"
    );
}

/// Read-only against the actual ai-skills repository, never mutating it:
/// reproduces every scenario `.github/tests/test-ci-subjects.sh`'s own
/// `check()` calls exercise.
#[test]
fn matches_the_real_bash_original_for_every_real_test_scenario() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("src/ci-subjects is two levels below the repo root")
        .to_path_buf();
    let script = repo_root.join(".github/ci-subjects.sh");
    assert!(script.is_file(), "{} not found", script.display());

    let scenarios: Vec<Vec<&str>> = vec![
        vec!["--scope", "full"],
        vec!["--scope", "wat"],
        vec!["--scope", "none"],
        vec!["--scope", "selective", "--crates", "rjq"],
        vec!["--scope", "selective", "--crates", "plan-crypt"],
        vec!["--scope", "selective", "--crates", "chat-proto"],
        vec![
            "--scope",
            "selective",
            "--crates",
            "chat-proto chat-server-rs chat-client-rs",
        ],
        vec!["--scope", "selective", "--crates", "ai-text-editor-mcp"],
        vec!["--scope", "selective", "--crates", "installer"],
        vec![
            "--scope",
            "selective",
            "--crates",
            "installer installer-platform installer-release",
        ],
        vec!["--scope", "selective", "--crates", "planning-core"],
        vec!["--scope", "selective", "--crates", "some-new-crate"],
        vec![
            "--scope",
            "selective",
            "--crates",
            "rjq chat-proto plan-overview installer",
        ],
        vec!["--scope", "selective", "--crates", ""],
        vec!["--help"],
    ];

    for args in scenarios {
        let bash_output = Command::new("bash")
            .arg(&script)
            .args(&args)
            .env_remove("GITHUB_OUTPUT")
            .current_dir(&repo_root)
            .output()
            .unwrap();
        let binary_output = Command::new(env!("CARGO_BIN_EXE_ci-subjects"))
            .args(&args)
            .env_remove("GITHUB_OUTPUT")
            .output()
            .unwrap();
        assert_eq!(
            bash_output.status.code(),
            binary_output.status.code(),
            "exit codes differ for {args:?}"
        );
        assert_eq!(
            combined_of(&bash_output),
            combined_of(&binary_output),
            "output differs for {args:?}"
        );
    }
}
