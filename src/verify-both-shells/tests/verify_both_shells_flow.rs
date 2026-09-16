// MODE: DEV
// Integration coverage for the verify-both-shells crate: library-level
// orchestration tests driving `run()` directly with fake Leg values (no
// real flake.nix/nix develop needed), plus real spawned-subprocess tests
// of the CLI surface that don't need the legs to run at all.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
#[cfg(unix)]
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use verify_both_shells::process::Leg;

fn unique_dir(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "verify-both-shells-flow-{tag}-{}-{:?}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    dir
}

struct Repo {
    dir: PathBuf,
}

impl Repo {
    fn new(tag: &str) -> Self {
        let dir = unique_dir(tag);
        fs::create_dir_all(&dir).unwrap();
        let repo = Repo { dir };
        repo.git(&["init", "-q", "-b", "master"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        repo.git(&["config", "user.name", "Test"]);
        repo
    }

    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(&self.dir)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    fn write(&self, rel_path: &str, content: &str) {
        let path = self.dir.join(rel_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
    }

    fn commit_all(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message]);
    }

    fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn passing_leg(label: &'static str) -> Leg {
    Leg {
        label,
        build: Box::new(|_wt| {
            let mut c = Command::new("bash");
            c.arg("-c")
                .arg("printf 'Total ran: 1   Passed: 1   Failed: 0\\n'");
            c
        }),
    }
}

fn failing_leg(label: &'static str) -> Leg {
    Leg {
        label,
        build: Box::new(|_wt| {
            let mut c = Command::new("bash");
            c.arg("-c").arg(
                "printf 'Total ran: 1   Passed: 0   Failed: 1\\nFailed: test-a\\n  test-a  FAIL\\n    boom\\n'",
            );
            c
        }),
    }
}

#[test]
fn a_successful_run_creates_and_removes_the_worktree_and_its_parent() {
    let repo = Repo::new("success");
    repo.write("marker.txt", "baseline\n");
    repo.commit_all("baseline");

    let legs = [passing_leg("bash 5.3"), passing_leg("bash 3.2")];
    let outcome = verify_both_shells::run(&repo.dir, false, legs);
    assert_eq!(outcome.status, 0);

    let worktrees = verify_both_shells::git::worktree_paths(&repo.dir);
    assert!(
        worktrees.iter().all(|p| !p.contains("/verify-wt.")),
        "worktree was not cleaned up: {worktrees:?}"
    );
    repo.cleanup();
}

#[test]
fn a_failing_leg_sets_exit_1_and_prints_the_failing_block() {
    let repo = Repo::new("failure");
    repo.write("marker.txt", "baseline\n");
    repo.commit_all("baseline");

    let legs = [failing_leg("bash 5.3"), passing_leg("bash 3.2")];
    let outcome = verify_both_shells::run(&repo.dir, false, legs);
    assert_eq!(outcome.status, 1);
    repo.cleanup();
}

#[test]
fn keep_with_a_failure_leaves_the_logs_on_disk() {
    let repo = Repo::new("keep-on-failure");
    repo.write("marker.txt", "baseline\n");
    repo.commit_all("baseline");

    let legs = [failing_leg("bash 5.3"), passing_leg("bash 3.2")];
    let outcome = verify_both_shells::run(&repo.dir, true, legs);
    assert_eq!(outcome.status, 1);

    let log5 = outcome.log5.expect("log5 path should be known");
    let log3 = outcome.log3.expect("log3 path should be known");
    assert!(
        log5.is_file(),
        "log5 should still be on disk (keep+failure)"
    );
    assert!(
        log3.is_file(),
        "log3 should still be on disk (keep+failure)"
    );
    let _ = fs::remove_file(&log5);
    let _ = fs::remove_file(&log3);
    repo.cleanup();
}

#[test]
fn without_keep_a_failure_still_removes_everything() {
    let repo = Repo::new("no-keep-on-failure");
    repo.write("marker.txt", "baseline\n");
    repo.commit_all("baseline");

    let legs = [failing_leg("bash 5.3"), passing_leg("bash 3.2")];
    let outcome = verify_both_shells::run(&repo.dir, false, legs);
    assert_eq!(outcome.status, 1);

    let log5 = outcome.log5.expect("log5 path should be known");
    let log3 = outcome.log3.expect("log3 path should be known");
    assert!(!log5.exists(), "log5 should be removed without --keep");
    assert!(!log3.exists(), "log3 should be removed without --keep");
    repo.cleanup();
}

#[test]
fn a_stale_leftover_worktree_from_a_dead_pid_is_swept() {
    let repo = Repo::new("sweep-dead");
    repo.write("marker.txt", "baseline\n");
    repo.commit_all("baseline");

    let parent = repo.dir.join("scratch/verify-wt.deadleftover");
    let wt = parent.join("tree");
    fs::create_dir_all(&parent).unwrap();
    repo.git(&[
        "worktree",
        "add",
        "--detach",
        "--quiet",
        wt.to_str().unwrap(),
        "HEAD",
    ]);
    let mut child = Command::new("true").spawn().unwrap();
    let dead_pid = child.id();
    let _ = child.wait();
    fs::write(parent.join("harness.pid"), dead_pid.to_string()).unwrap();

    let legs = [passing_leg("bash 5.3"), passing_leg("bash 3.2")];
    let outcome = verify_both_shells::run(&repo.dir, false, legs);
    assert_eq!(outcome.status, 0);
    assert!(!wt.exists(), "the stale worktree should have been swept");
    assert!(
        !parent.exists(),
        "the stale worktree's own parent should also be gone (AR-79)"
    );
    repo.cleanup();
}

#[test]
fn a_stale_leftover_worktree_from_a_live_pid_is_left_alone() {
    let repo = Repo::new("sweep-live");
    repo.write("marker.txt", "baseline\n");
    repo.commit_all("baseline");

    let parent = repo.dir.join("scratch/verify-wt.liveleftover");
    let wt = parent.join("tree");
    fs::create_dir_all(&parent).unwrap();
    repo.git(&[
        "worktree",
        "add",
        "--detach",
        "--quiet",
        wt.to_str().unwrap(),
        "HEAD",
    ]);
    fs::write(parent.join("harness.pid"), std::process::id().to_string()).unwrap();

    let legs = [passing_leg("bash 5.3"), passing_leg("bash 3.2")];
    let outcome = verify_both_shells::run(&repo.dir, false, legs);
    assert_eq!(outcome.status, 0);
    assert!(
        wt.exists(),
        "a worktree owned by a live process must not be swept"
    );

    repo.git(&["worktree", "remove", "--force", wt.to_str().unwrap()]);
    let _ = fs::remove_dir_all(&parent);
    repo.cleanup();
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
    let output = Command::new(env!("CARGO_BIN_EXE_verify-both-shells"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", combined_of(&output));
    assert!(stdout_of(&output).contains("run the suite on the working tree under both shells"));
}

#[test]
fn an_unknown_argument_exits_64() {
    let output = Command::new(env!("CARGO_BIN_EXE_verify-both-shells"))
        .arg("-x")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(64));
    assert!(
        combined_of(&output).contains("unknown argument: -x"),
        "{}",
        combined_of(&output)
    );
}

/// Signal delivery needs a real, separately spawned process; leg 1 always
/// runs before leg 2's real `nix develop` call, so a signal delivered
/// during a deliberately slow fixture `run-tests.sh` never reaches leg 2.
/// Shared by the SIGINT/SIGTERM/SIGHUP tests below. Unix only: SIGINT/
/// SIGTERM/SIGHUP and libc::kill have no Windows equivalent, and
/// verify-both-shells.sh itself has no meaningful bash-comparison workflow
/// there anyway (no bash to compare) -- the crate still builds and its
/// other tests still run on every platform, matching src/verify-both-shells
/// own #[cfg(unix)] signal-handling split.
#[cfg(unix)]
fn assert_signal_removes_worktree(tag: &str, signal: libc::c_int, expected_exit: i32) {
    let repo = Repo::new(tag);
    repo.write("run-tests.sh", "#!/usr/bin/env bash\nsleep 30\n");
    fs::set_permissions(
        repo.dir.join("run-tests.sh"),
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )
    .unwrap();
    repo.commit_all("slow fixture run-tests.sh");

    let mut child = Command::new(env!("CARGO_BIN_EXE_verify-both-shells"))
        .current_dir(&repo.dir)
        .env("PLANNING_SKILL_ROOT", &repo.dir)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    // Wait for the "worktree ..." line to confirm the worktree exists
    // before signaling: a background thread reads lines and hands the
    // matching one back over a channel, bounded by a timeout.
    use std::io::{BufRead, BufReader};
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if line.starts_with("worktree ") {
                let _ = tx.send(line);
                return;
            }
        }
    });
    let worktree_line = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("did not see a worktree line in time");
    let wt_path = worktree_line
        .trim_start_matches("worktree ")
        .split(" (base")
        .next()
        .unwrap()
        .trim()
        .to_string();
    assert!(Path::new(&wt_path).exists());

    unsafe {
        libc::kill(child.id() as i32, signal);
    }
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(expected_exit));
    assert!(
        !Path::new(&wt_path).exists(),
        "the worktree should be removed after signal {signal}"
    );

    repo.cleanup();
}

#[cfg(unix)]
#[test]
fn sigint_mid_run_removes_the_worktree_and_exits_128_plus_2() {
    assert_signal_removes_worktree("sigint", libc::SIGINT, 128 + 2);
}

#[cfg(unix)]
#[test]
fn sigterm_mid_run_removes_the_worktree_and_exits_128_plus_15() {
    assert_signal_removes_worktree("sigterm", libc::SIGTERM, 128 + 15);
}

#[cfg(unix)]
#[test]
fn sighup_mid_run_removes_the_worktree_and_exits_128_plus_1() {
    assert_signal_removes_worktree("sighup", libc::SIGHUP, 128 + 1);
}

/// Read-only against the actual ai-skills repository, never mutating it:
/// `--help` and an unknown-argument invocation only -- NOT a full
/// worktree-and-both-legs run, since that would spawn the real, slow
/// `run-tests.sh` twice from within the test suite itself.
#[test]
fn matches_the_real_bash_original_for_help_and_unknown_argument() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("src/verify-both-shells is two levels below the repo root")
        .to_path_buf();
    let script = repo_root.join("verify-both-shells.sh");
    assert!(script.is_file(), "{} not found", script.display());

    for args in [vec!["--help"], vec!["-x"]] {
        let bash_output = Command::new("bash")
            .arg(&script)
            .args(&args)
            .current_dir(&repo_root)
            .output()
            .unwrap();
        let binary_output = Command::new(env!("CARGO_BIN_EXE_verify-both-shells"))
            .args(&args)
            .current_dir(&repo_root)
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
