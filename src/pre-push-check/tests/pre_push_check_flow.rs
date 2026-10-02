// MODE: DEV
// Real-subprocess integration coverage for the compiled pre-push-check
// binary, mirroring ci-failures's own established convention: a real,
// throwaway git repository rather than a stubbed one, since this binary's
// entire job is measuring a real change set against it. AI_SKILLS_PREPUSH_IN_NIX=1
// is set on every invocation so the binary never tries its own nix re-exec
// inside the test's own environment (which may or may not already be a nix
// shell); the re-exec path itself is covered by unit tests in reexec.rs and
// by manual invocation (W84's own acceptance run), not here.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

impl Repo {
    fn new(tag: &str) -> Self {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "pre-push-check-flow-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "work"]);
        Repo { dir }
    }

    fn commit(&self, message: &str) {
        git(&self.dir, &["add", "-A"]);
        git(&self.dir, &["commit", "-q", "-m", message]);
    }

    /// A stub generate-portability.sh that always succeeds, so an
    /// otherwise-empty scratch repo can reach an all-gates-passing run: the
    /// real script does not exist outside this repository's own tree.
    fn stub_portability_ok(&self) {
        write_executable(
            &self.dir.join("generate-portability.sh"),
            "#!/usr/bin/env bash\nexit 0\n",
        );
    }

    fn run(&self, args: &[&str]) -> Output {
        let binary = env!("CARGO_BIN_EXE_pre-push-check");
        Command::new(binary)
            .args(args)
            .current_dir(&self.dir)
            .env("AI_SKILLS_PREPUSH_IN_NIX", "1")
            .env("PRE_PUSH_SKIP_FETCH", "1")
            .output()
            .unwrap()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn not_a_git_repository_exits_65() {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "pre-push-check-flow-not-a-repo-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();

    let binary = env!("CARGO_BIN_EXE_pre-push-check");
    let output = Command::new(binary)
        .current_dir(&dir)
        .env("AI_SKILLS_PREPUSH_IN_NIX", "1")
        .env("PRE_PUSH_SKIP_FETCH", "1")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(65));
    assert!(stderr(&output).contains("pre-push-check.sh: not a git repository"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn registers_branch_gate_blocks_with_its_own_exact_terminal_line() {
    let repo = Repo::new("registers-gate");
    write_file(&repo.dir.join("README.md"), "hello\n");
    repo.commit("initial");

    write_file(&repo.dir.join("BUGS.json"), "[]\n");
    git(&repo.dir, &["add", "BUGS.json"]);

    let output = repo.run(&[]);
    let out = stdout(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(out.contains("a register is modified outside the registers branch"));
    assert!(out.contains("    BUGS.json"));
    assert!(out.contains("THE TARGET BRANCH IS: registers"));
    assert!(
        out.contains("pre-push-check: 1 failure(s) - registers changed off the registers branch")
    );
    // Gate 0 fails before any other gate runs.
    assert!(!out.contains("git diff --check"));
}

/// A repository already on `registers`, with one committed file to diff from.
fn registers_repo(tag: &str) -> Repo {
    let repo = Repo::new(tag);
    write_file(&repo.dir.join("README.md"), "hello\n");
    write_file(&repo.dir.join("BUGS.json"), "[]\n");
    write_file(&repo.dir.join("TODO.json"), "[]\n");
    repo.commit("initial");
    git(&repo.dir, &["checkout", "-q", "-b", "registers"]);
    repo
}

#[test]
fn registers_branch_passes_on_register_changes_and_runs_no_other_gate() {
    let repo = registers_repo("registers-branch-ok");
    write_file(&repo.dir.join("BUGS.json"), "[ ]\n");
    write_file(&repo.dir.join("TODO.json"), "[ ]\n");
    git(&repo.dir, &["add", "BUGS.json"]);

    let output = repo.run(&[]);
    let out = stdout(&output);

    assert_eq!(output.status.code(), Some(0), "{out}");
    assert!(!out.contains("a register is modified outside"));
    assert!(out.contains("only registers changed on the registers branch (BUGS.json, TODO.json)"));
    assert!(out.ends_with("pre-push-check: PASS\n"));
    // None of the ordinary gates ran: not whitespace, not cargo, not soundness.
    for gate in ["git diff --check", "PORTABILITY.md", "cargo", "soundness"] {
        assert!(
            !out.contains(gate),
            "{gate} ran on the registers branch: {out}"
        );
    }
}

#[test]
fn registers_branch_refuses_any_other_changed_file() {
    let repo = registers_repo("registers-branch-stray");
    write_file(&repo.dir.join("BUGS.json"), "[ ]\n");
    write_file(&repo.dir.join("README.md"), "changed\n");
    write_file(&repo.dir.join("flake.nix"), "{}\n");
    git(&repo.dir, &["add", "-A"]);

    let output = repo.run(&[]);
    let out = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{out}");
    assert!(out.contains("the registers branch may only change BUGS.json and TODO.json"));
    assert!(out.contains("    README.md"));
    assert!(out.contains("    flake.nix"));
    assert!(
        !out.contains("    BUGS.json"),
        "a register was listed as stray: {out}"
    );
    assert!(out.ends_with("pre-push-check: 1 failure(s)\n"));
}

#[test]
fn registers_branch_refuses_an_unstaged_stray_edit_too() {
    let repo = registers_repo("registers-branch-unstaged");
    write_file(&repo.dir.join("README.md"), "edited, not staged\n");

    let output = repo.run(&[]);

    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(stdout(&output).contains("    README.md"));
}

#[test]
fn registers_branch_with_nothing_changed_passes() {
    let repo = registers_repo("registers-branch-empty");

    let output = repo.run(&[]);
    let out = stdout(&output);

    assert_eq!(output.status.code(), Some(0), "{out}");
    assert!(out.contains("nothing differs from master"));
}

/// master is refreshed BEFORE the base is resolved. `registers` is levelled
/// with master by a workflow, so it carries master's own newer commits; with a
/// stale origin/master ref those commits would read as this branch's changes.
/// The clone below has exactly that: origin/master pinned to the first commit
/// while its `registers` branch sits on top of the second and adds one register.
#[test]
fn master_is_fetched_before_the_base_is_resolved() {
    let origin = Repo::new("fetch-first-origin");
    git(&origin.dir, &["checkout", "-q", "-b", "master"]);
    write_file(&origin.dir.join("README.md"), "one\n");
    origin.commit("first");
    let first = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&origin.dir)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    write_file(&origin.dir.join("other.txt"), "master moved on\n");
    origin.commit("second, on master only");

    let mut clone_dir = std::env::temp_dir();
    clone_dir.push(format!(
        "pre-push-check-flow-fetch-first-clone-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let status = Command::new("git")
        .args(["clone", "-q"])
        .arg(&origin.dir)
        .arg(&clone_dir)
        .status()
        .unwrap();
    assert!(status.success());
    git(
        &clone_dir,
        &["update-ref", "refs/remotes/origin/master", &first],
    );
    git(&clone_dir, &["checkout", "-q", "-b", "registers"]);
    write_file(&clone_dir.join("BUGS.json"), "[]\n");
    git(&clone_dir, &["add", "-A"]);
    git(&clone_dir, &["commit", "-q", "-m", "file a bug"]);

    let output = Command::new(env!("CARGO_BIN_EXE_pre-push-check"))
        .current_dir(&clone_dir)
        .env("AI_SKILLS_PREPUSH_IN_NIX", "1")
        .env_remove("PRE_PUSH_SKIP_FETCH")
        .output()
        .unwrap();
    let out = stdout(&output);

    assert_eq!(output.status.code(), Some(0), "{out}");
    assert!(out.contains("fetched origin master"), "{out}");
    assert!(
        out.contains("only registers changed on the registers branch (BUGS.json)"),
        "master's own commit was counted as this branch's: {out}"
    );
    assert!(!out.contains("other.txt"), "{out}");

    let _ = fs::remove_dir_all(&clone_dir);
}

/// The stale master flake's dev shell does not build on Apple Silicon, so a
/// push from `registers` must never reach `nix develop`. A `nix` that records
/// its own call and fails stands in for it, with no marker variable set.
#[cfg(unix)]
#[test]
fn registers_branch_never_re_enters_nix() {
    let repo = registers_repo("registers-branch-no-nix");
    write_file(&repo.dir.join("BUGS.json"), "[ ]\n");
    let fake_bin = repo.dir.join(".fake-bin");
    let marker = repo.dir.join(".nix-was-called");
    write_executable(
        &fake_bin.join("nix"),
        &format!("#!/bin/sh\ntouch '{}'\nexit 97\n", marker.display()),
    );
    let path = format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let output = Command::new(env!("CARGO_BIN_EXE_pre-push-check"))
        .current_dir(&repo.dir)
        .env("PATH", path)
        .env_remove("AI_SKILLS_PREPUSH_IN_NIX")
        .env_remove("IN_NIX_SHELL")
        .env("PRE_PUSH_SKIP_FETCH", "1")
        .output()
        .unwrap();

    assert!(!marker.exists(), "nix was called from the registers branch");
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
}

#[test]
fn whitespace_gate_reports_the_bad_line_and_the_exact_summary_for_a_broken_diff() {
    let repo = Repo::new("whitespace-gate");
    write_file(&repo.dir.join("README.md"), "hello\n");
    repo.commit("initial");
    repo.stub_portability_ok();

    // A trailing-whitespace line is exactly what `git diff --check` flags.
    write_file(&repo.dir.join("trailing.txt"), "line one   \n");
    git(&repo.dir, &["add", "trailing.txt"]);

    let output = repo.run(&[]);
    let out = stdout(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(out.contains("whitespace errors: git diff --check"));
    assert!(out.contains("pre-push-check: 1 failure(s)"));
}

#[test]
fn bash_syntax_gate_reports_bash_n_failure_for_a_broken_script() {
    let repo = Repo::new("bash-syntax-gate");
    write_file(&repo.dir.join("README.md"), "hello\n");
    repo.commit("initial");
    repo.stub_portability_ok();

    write_file(
        &repo.dir.join("broken.sh"),
        "#!/usr/bin/env bash\nif [ 1 -eq 1 ]; then\n",
    );
    git(&repo.dir, &["add", "broken.sh"]);

    let output = repo.run(&[]);
    let out = stdout(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(out.contains("bash -n: broken.sh"));
}

#[test]
fn all_gates_passing_reports_pass_and_exit_zero() {
    let repo = Repo::new("all-passing");
    write_file(&repo.dir.join("README.md"), "hello\n");
    repo.commit("initial");
    repo.stub_portability_ok();

    let output = repo.run(&[]);
    let out = stdout(&output);

    assert_eq!(output.status.code(), Some(0));
    assert!(out.contains("regenerated PORTABILITY.md"));
    assert!(out.ends_with("pre-push-check: PASS\n"));
}

#[test]
fn help_exits_zero_and_never_touches_git() {
    let dir = std::env::temp_dir();
    let binary = env!("CARGO_BIN_EXE_pre-push-check");
    let output = Command::new(binary)
        .arg("--help")
        .current_dir(&dir)
        .env("AI_SKILLS_PREPUSH_IN_NIX", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).starts_with("pre-push-check - the per-change gates"));
}

#[test]
fn unknown_argument_exits_64() {
    let repo = Repo::new("unknown-arg");
    let output = repo.run(&["--bogus"]);
    assert_eq!(output.status.code(), Some(64));
    assert!(stderr(&output).contains("pre-push-check.sh: unknown argument: --bogus"));
}
