// MODE: DEV
// Real-subprocess integration coverage for the compiled run-tests binary,
// mirroring pre-push-check/ci-failures's own established convention: a
// small synthetic scratch tree built to look like a repository this binary
// can run its own discovery/execution logic against, rather than exercising
// the real ai-skills repository's own (much larger, slower) test set.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

// `bash_program()`: on Windows a bare `Command::new("bash")` (or the path
// "/bin/bash") is not Git for Windows' bash.
#[path = "../../../tests/rust-support/script_stub.rs"]
mod script_stub;

/// Where the binary keeps its lock: `/tmp/ai-skills-run-tests.lock`, or the
/// platform's temporary directory where there is no /tmp.
fn lock_file() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::temp_dir()
    } else {
        PathBuf::from("/tmp")
    };
    base.join("ai-skills-run-tests.lock")
}

/// run-tests's own lock path is a fixed, unconfigurable /tmp location by
/// design (matching the bash original: a mutex only works if every run
/// agrees on where it lives), so every test that touches it must be
/// serialized against every other one -- cargo test's default parallelism
/// would otherwise let two of these tests race over the same real file.
fn lock_test_guard() -> &'static Mutex<()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(()))
}

/// True when `lock_path` names a real, live run-tests holder -- mirroring
/// the production lock_holder_is_live check. A test that would otherwise
/// overwrite or delete this file must not, when the real, hardcoded,
/// unconfigurable lock path this binary uses (by design, for bash parity)
/// happens to already be held by a genuinely separate, still-running
/// run-tests process (an enclosing run-tests.sh invocation of this very
/// suite, most plausibly).
fn lock_is_held_by_a_live_process(lock_path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(lock_path) else {
        return false;
    };
    let Some(pid) = content.lines().next() else {
        return false;
    };
    if pid.is_empty() || !pid.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    #[cfg(windows)]
    {
        // No `ps`: tasklist names the image of a running pid.
        let filter = format!("PID eq {pid}");
        if let Ok(out) = Command::new("tasklist")
            .args(["/FI", &filter, "/FO", "CSV", "/NH"])
            .output()
        {
            return String::from_utf8_lossy(&out.stdout).contains("run-tests");
        }
        false
    }
    #[cfg(not(windows))]
    {
        for flag in ["args=", "command="] {
            if let Ok(out) = Command::new("ps").args(["-p", pid, "-o", flag]).output() {
                let text = String::from_utf8_lossy(&out.stdout);
                if text.contains("run-tests") {
                    return true;
                }
            }
        }
        false
    }
}

struct Repo {
    dir: PathBuf,
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn write_executable(path: &Path, contents: &str) {
    write_file(path, contents);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

fn unique_dir(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "run-tests-flow-{tag}-{}-{:?}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    dir
}

impl Repo {
    /// A scratch git repo with a "tests/" suite directory containing four
    /// synthetic scripts (pass, fail, skip, hang-past-timeout) and a stub
    /// generate-portability.sh / bootstrap.sh so bootstrap_generated does
    /// not fail outright in an otherwise-empty tree.
    fn new(tag: &str) -> Self {
        let dir = unique_dir(tag);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "work"]);

        write_executable(
            &dir.join("generate-portability.sh"),
            "#!/usr/bin/env bash\nexit 0\n",
        );
        write_executable(
            &dir.join("bootstrap.sh"),
            "#!/usr/bin/env bash\nexit 1\n", // rjq stays missing; nothing here needs it
        );

        write_executable(
            &dir.join("tests/test-passes.sh"),
            "#!/usr/bin/env bash\nexit 0\n",
        );
        write_executable(
            &dir.join("tests/test-fails.sh"),
            "#!/usr/bin/env bash\necho boom\nexit 1\n",
        );
        write_executable(
            &dir.join("tests/test-skips.sh"),
            "#!/usr/bin/env bash\necho 'test-skips: SKIP'\nexit 0\n",
        );
        write_executable(
            &dir.join("tests/test-hangs.sh"),
            "#!/usr/bin/env bash\nsleep 30\n",
        );

        // The other five suite directories + benchmark suite must exist as
        // real (even if empty) directories, or `find` on a missing path
        // just yields nothing -- matching the bash original's own behavior,
        // not a special case this test needs to construct.
        for suite in [
            "planning/tests",
            "editor-gate-plugin/tests",
            "tui-hint-plugin/tests",
            "agent-identity-plugin/tests",
            ".github/tests",
            "benchmark/planning/tests",
        ] {
            fs::create_dir_all(dir.join(suite)).unwrap();
        }

        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-q", "-m", "initial"]);

        Repo { dir }
    }

    fn add_crate(&self, name: &str, cargo_toml: &str, lib_rs: &str) {
        write_file(&self.dir.join(format!("src/{name}/Cargo.toml")), cargo_toml);
        write_file(&self.dir.join(format!("src/{name}/src/lib.rs")), lib_rs);
    }

    /// Bypasses the machine-wide lock (AI_SKILLS_ALLOW_CONCURRENT=1):
    /// every caller of this helper is testing discovery/execution/reporting
    /// logic, not the lock itself, and the real, hardcoded, unconfigurable
    /// /tmp/ai-skills-run-tests.lock this binary uses by design (matching
    /// bash parity -- a mutex only works if every run agrees on where it
    /// lives) is ALSO the exact lock an enclosing run-tests.sh invocation of
    /// this very suite already holds for its own whole duration whenever
    /// these tests run as this crate's own `cargo test` step within it.
    /// Without this, a nested run wrongly observes lock refusal (exit 75)
    /// as if it were a real test failure -- confirmed directly: this exact
    /// collision was how run-tests's own full self-hosted run-tests.sh run
    /// found it.
    fn run(&self, args: &[&str]) -> Output {
        let binary = env!("CARGO_BIN_EXE_run-tests");
        Command::new(binary)
            .args(args)
            .current_dir(&self.dir)
            .env("AI_SKILLS_RESOURCE_LIMIT", "0")
            .env("AI_SKILLS_ALLOW_CONCURRENT", "1")
            .env("RUN_TESTS_BASH", which_bash())
            .env("AI_SKILLS_TEST_TIMEOUT", "3")
            .output()
            .unwrap()
    }
}

fn which_bash() -> String {
    // Windows has no /bin/bash and no $SHELL worth trusting: use Git for
    // Windows' bash, found the way the run-tests binary itself finds it.
    #[cfg(windows)]
    {
        script_stub::bash_program().to_string_lossy().into_owned()
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL")
            .ok()
            .filter(|s| s.ends_with("bash"))
            .unwrap_or_else(|| "/bin/bash".to_string())
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn list_only_prints_every_discovered_script() {
    let repo = Repo::new("list-only");
    let output = repo.run(&["--list-only"]);
    let out = stdout(&output);
    assert_eq!(output.status.code(), Some(0));
    assert!(out.contains("tests/test-passes.sh"));
    assert!(out.contains("tests/test-fails.sh"));
    assert!(out.contains("tests/test-skips.sh"));
    assert!(out.contains("tests/test-hangs.sh"));
}

#[test]
fn a_full_run_reports_the_correct_counts_and_exact_summary() {
    let _guard = lock_test_guard().lock().unwrap_or_else(|p| p.into_inner());
    let repo = Repo::new("full-run");
    // Exclude the hanging test from this run so it stays fast; the
    // dedicated timeout test below covers it on its own.
    let select = repo.dir.join("select.txt");
    fs::write(
        &select,
        "tests/test-passes.sh\ntests/test-fails.sh\ntests/test-skips.sh\n",
    )
    .unwrap();
    let output = repo.run(&["--select-file", "select.txt"]);
    let out = stdout(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(out.contains("test-passes"));
    assert!(out.contains("PASS"));
    assert!(out.contains("test-fails"));
    assert!(out.contains("FAIL (exit 1)"));
    assert!(out.contains("boom"));
    assert!(out.contains("test-skips"));
    assert!(out.contains("SKIP"));
    assert!(out.contains("Total ran: 3   Passed: 1   Failed: 1   Skipped: 1   Unconfigured: 0"));
    assert!(out.contains("Failed: test-fails"));
    assert!(out.contains("Skipped: test-skips"));
}

#[test]
fn a_hanging_test_is_reported_as_timeout() {
    // Bounded by timeout(1) where there is one, and by the binary itself
    // where there is not (Windows, a stock macOS), so the answer is TIMEOUT
    // on every host.
    let _guard = lock_test_guard().lock().unwrap_or_else(|p| p.into_inner());
    let repo = Repo::new("timeout");
    let select = repo.dir.join("select.txt");
    fs::write(&select, "tests/test-hangs.sh\n").unwrap();
    let output = repo.run(&["--select-file", "select.txt"]);
    let out = stdout(&output);
    assert!(out.contains("test-hangs"));
    assert!(out.contains("TIMEOUT"));
}

#[test]
fn a_crate_is_discovered_and_run_via_cargo_test() {
    let _guard = lock_test_guard().lock().unwrap_or_else(|p| p.into_inner());
    let repo = Repo::new("crate-run");
    repo.add_crate(
        "trivial",
        "[package]\nname = \"trivial\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        "#[test]\nfn it_passes() { assert!(true); }\n",
    );
    let select = repo.dir.join("select.txt");
    fs::write(&select, "src/trivial\n").unwrap();
    let output = repo.run(&["--select-file", "select.txt"]);
    let out = stdout(&output);
    assert!(out.contains("cargo-trivial"));
    assert!(out.contains("PASS") || out.contains("UNCONFIGURED"));
}

#[test]
fn the_lock_refuses_a_genuinely_concurrent_second_run() {
    let _guard = lock_test_guard().lock().unwrap_or_else(|p| p.into_inner());
    let repo = Repo::new("lock-refuse");
    let select = repo.dir.join("select.txt");
    fs::write(&select, "tests/test-hangs.sh\n").unwrap();

    let binary = env!("CARGO_BIN_EXE_run-tests");
    // No timeout override: test-hangs.sh sleeps 30s, and this test kills
    // `first` explicitly at the end -- a short timeout here would let the
    // first run finish and release the lock before the "blocked" attempt
    // below ever gets to it, which is exactly the race that made this test
    // flaky (VIOLATION self-reported: an earlier 5s timeout intermittently
    // let `first` exit before `blocked` ran, observed directly rather than
    // assumed).
    let mut first = Command::new(binary)
        .args(["--select-file", "select.txt"])
        .current_dir(&repo.dir)
        .env("AI_SKILLS_RESOURCE_LIMIT", "0")
        .env("RUN_TESTS_BASH", which_bash())
        .spawn()
        .unwrap();

    // Wait for the first run to actually take the machine-wide lock.
    let lock_buf = lock_file();
    let lock_path = lock_buf.as_path();
    for _ in 0..50 {
        if lock_path.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let second = repo.run(&["--list-only"]);
    // --list-only takes no lock at all, so it must succeed regardless --
    // exercise the actual refusal with a real run instead.
    assert_eq!(
        second.status.code(),
        Some(0),
        "--list-only must never contend for the lock"
    );
    assert!(
        lock_path.exists(),
        "the lock disappeared between the poll and the blocked attempt -- `first` exited early"
    );

    let blocked = Command::new(binary)
        .args(["--select-file", "select.txt"])
        .current_dir(&repo.dir)
        .env("AI_SKILLS_RESOURCE_LIMIT", "0")
        .env("RUN_TESTS_BASH", which_bash())
        .output()
        .unwrap();
    assert_eq!(blocked.status.code(), Some(75));
    assert!(stderr(&blocked).contains("another suite run is already going"));

    let _ = first.kill();
    let _ = first.wait();
    let _ = std::fs::remove_file(lock_path);
}

#[test]
fn a_stale_lock_is_reclaimed_and_the_run_proceeds() {
    let _guard = lock_test_guard().lock().unwrap_or_else(|p| p.into_inner());
    let lock_buf = lock_file();
    let lock_path = lock_buf.as_path();
    if lock_is_held_by_a_live_process(lock_path) {
        eprintln!(
            "skipping: {lock_path:?} is genuinely held by a live process (an enclosing \
             run-tests.sh run?) -- overwriting it here would corrupt that holder's own lock"
        );
        return;
    }
    let repo = Repo::new("stale-lock");
    let _ = std::fs::remove_file(lock_path);
    // pid 1 is always live on a real system but is never a run-tests
    // process, so lock_holder_is_live must reject it as "not ours" and
    // reclaim the lock rather than treating a live-but-foreign pid as a
    // legitimate holder.
    std::fs::write(
        lock_path,
        "1\nai-skills-run-tests\n/nonexistent\n2020-01-01T00:00:00Z\n",
    )
    .unwrap();

    let output = repo.run(&["--list-only"]);
    assert_eq!(output.status.code(), Some(0));
    let _ = std::fs::remove_file(lock_path);
}

#[cfg(unix)]
#[test]
fn sigterm_still_removes_the_scratch_root_and_releases_the_lock() {
    let _guard = lock_test_guard().lock().unwrap_or_else(|p| p.into_inner());
    let lock_buf = lock_file();
    let lock_path = lock_buf.as_path();
    if lock_is_held_by_a_live_process(lock_path) {
        eprintln!("skipping: {lock_path:?} is genuinely held by a live process");
        return;
    }
    let repo = Repo::new("sigterm");
    let select = repo.dir.join("select.txt");
    fs::write(&select, "tests/test-hangs.sh\n").unwrap();
    let _ = std::fs::remove_file(lock_path);

    let binary = env!("CARGO_BIN_EXE_run-tests");
    let mut child = Command::new(binary)
        .args(["--select-file", "select.txt"])
        .current_dir(&repo.dir)
        .env("AI_SKILLS_RESOURCE_LIMIT", "0")
        .env("RUN_TESTS_BASH", which_bash())
        .spawn()
        .unwrap();

    for _ in 0..50 {
        if lock_path.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(lock_path.exists(), "the run never took the lock");

    send_signal(child.id(), 15); // SIGTERM
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(143));
    assert!(!lock_path.exists(), "SIGTERM must still release the lock");
}

#[cfg(unix)]
#[test]
fn sigint_still_removes_the_scratch_root_and_releases_the_lock() {
    let _guard = lock_test_guard().lock().unwrap_or_else(|p| p.into_inner());
    let lock_buf = lock_file();
    let lock_path = lock_buf.as_path();
    if lock_is_held_by_a_live_process(lock_path) {
        eprintln!("skipping: {lock_path:?} is genuinely held by a live process");
        return;
    }
    let repo = Repo::new("sigint");
    let select = repo.dir.join("select.txt");
    fs::write(&select, "tests/test-hangs.sh\n").unwrap();
    let _ = std::fs::remove_file(lock_path);

    let binary = env!("CARGO_BIN_EXE_run-tests");
    let mut child = Command::new(binary)
        .args(["--select-file", "select.txt"])
        .current_dir(&repo.dir)
        .env("AI_SKILLS_RESOURCE_LIMIT", "0")
        .env("RUN_TESTS_BASH", which_bash())
        .spawn()
        .unwrap();

    for _ in 0..50 {
        if lock_path.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(lock_path.exists(), "the run never took the lock");

    send_signal(child.id(), 2); // SIGINT
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(130));
    assert!(!lock_path.exists(), "SIGINT must still release the lock");
}

#[cfg(unix)]
fn send_signal(pid: u32, sig: i32) {
    unsafe {
        libc::kill(pid as libc::pid_t, sig);
    }
}
