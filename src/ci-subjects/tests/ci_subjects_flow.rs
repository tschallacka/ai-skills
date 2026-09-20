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

fn real_repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("src/ci-subjects is two levels below the repo root")
        .to_path_buf()
}

/// A scratch bin directory holding exactly the ci-subjects this test binary
/// was built alongside (`CARGO_BIN_EXE_ci-subjects`), pinned onto
/// AI_SKILLS_BIN_ROOT (tier 1) for the wrapper invocation below. That is the
/// very binary this test asserts fidelity against, and staging it here means
/// the test needs no `./setup-dev-env.sh` output: it used to read
/// `<repo>/bin/<triple>/ci-subjects`, which a fresh CI checkout running
/// `cargo test --workspace` does not have. A stale or partial shared install
/// under ~/.config/tsch-ai-skills/bin (tier 2) still cannot shadow it.
///
/// The directory has to hold ONLY that binary, so it is not the build
/// directory itself (which carries whatever else was once built there), and it
/// is created beside the built binary rather than under the temp directory, so
/// `cargo clean` removes it and repeated runs leave nothing behind in $TMPDIR.
fn staged_bin_dir(_repo_root: &Path) -> std::path::PathBuf {
    static STAGED: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    STAGED
        .get_or_init(|| {
            let built = Path::new(env!("CARGO_BIN_EXE_ci-subjects"));
            let dir = built
                .parent()
                .expect("a built binary lives in a directory")
                .join("ci-subjects-staged-bin");
            fs::create_dir_all(&dir).unwrap();
            fs::copy(built, dir.join("ci-subjects")).unwrap();
            dir
        })
        .clone()
}

// ---- real-tree exec fidelity and the missing-binary fallback (W123: the
// bash reimplementation body is gone, so there is nothing left to compare it
// against -- what remains to prove is that invoking .github/ci-subjects.sh,
// which execs the compiled binary via its own wiring block, produces
// byte-identical output to invoking the compiled binary directly, and that
// the wiring's own safe-default fires when no compiled binary can be found) --
#[test]
fn exec_fidelity_matches_the_compiled_binary_for_every_real_test_scenario() {
    let repo_root = real_repo_root();
    let script = repo_root.join(".github/ci-subjects.sh");
    assert!(script.is_file(), "{} not found", script.display());
    let bin_dir = staged_bin_dir(&repo_root);

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
        let wrapper_output = Command::new("bash")
            .arg(&script)
            .args(&args)
            .env("AI_SKILLS_BIN_ROOT", &bin_dir)
            .env_remove("GITHUB_OUTPUT")
            .current_dir(&repo_root)
            .output()
            .unwrap();
        let direct_output = Command::new(env!("CARGO_BIN_EXE_ci-subjects"))
            .args(&args)
            .env_remove("GITHUB_OUTPUT")
            .output()
            .unwrap();
        assert_eq!(
            wrapper_output.status.code(),
            direct_output.status.code(),
            "exit codes differ for {args:?}"
        );
        assert_eq!(
            combined_of(&wrapper_output),
            combined_of(&direct_output),
            "output differs for {args:?}"
        );
    }
}

// AR-100: this must not mutate the real, shared planning/scripts/plan-core-lib.sh
// in place -- copy ci-subjects.sh into a per-test scratch tree whose
// planning/scripts/ has no plan-core-lib.sh, so the wiring's own
// [ -f .../plan-core-lib.sh ] check is false there with zero shared mutable
// state touched.
#[test]
fn missing_binary_falls_back_to_the_all_true_safe_default() {
    let real_repo_root = real_repo_root();
    let scratch = std::env::temp_dir().join(format!(
        "ci-subjects-flow-missing-binary-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(scratch.join(".github")).unwrap();
    fs::create_dir_all(scratch.join("planning/scripts")).unwrap();
    fs::copy(
        real_repo_root.join(".github/ci-subjects.sh"),
        scratch.join(".github/ci-subjects.sh"),
    )
    .unwrap();

    let output = Command::new("bash")
        .arg(scratch.join(".github/ci-subjects.sh"))
        .arg("--scope")
        .arg("full")
        .current_dir(&scratch)
        .env_remove("GITHUB_OUTPUT")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", combined_of(&output));
    let out = stdout_of(&output);
    assert_eq!(
        out, "rjq=true\nchat=true\nplan_crypt=true\nplanning_commands=true\neditor=true\ninstaller=true\n",
        "stdout: {out}"
    );
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("ci-subjects binary not found; run ./setup-dev-env.sh to build it"),
        "stderr: {err}"
    );

    let _ = fs::remove_dir_all(&scratch);
}
