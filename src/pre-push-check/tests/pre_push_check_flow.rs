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

#[test]
fn registers_branch_gate_is_bypassed_on_the_registers_branch_itself() {
    let repo = Repo::new("registers-branch-ok");
    write_file(&repo.dir.join("README.md"), "hello\n");
    repo.commit("initial");
    git(&repo.dir, &["checkout", "-q", "-b", "registers"]);
    repo.stub_portability_ok();

    write_file(&repo.dir.join("BUGS.json"), "[]\n");
    git(&repo.dir, &["add", "BUGS.json"]);

    let output = repo.run(&[]);
    let out = stdout(&output);
    assert!(!out.contains("a register is modified outside"));
    assert!(out.contains("regenerated PORTABILITY.md"));
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
