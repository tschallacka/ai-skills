// MODE: DEV
// Real-subprocess integration coverage for the compiled setup-dev-env
// binary, mirroring pre-push-check/run-tests's own established convention:
// a small synthetic scratch tree built to look like a repository this
// binary can build and stage crates against, rather than exercising the
// real (much larger, slower) ai-skills workspace. Every test points
// PLANNING_SKILL_ROOT at its own scratch fixture -- repo_root is never
// allowed to default to the real ai-skills repository, since this binary
// writes marker files, stages binaries, and runs `git config
// core.hooksPath hooks` against whatever repo_root resolves to.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

// `bash_program()`: on Windows a bare `Command::new("bash")` finds System32's
// WSL launcher before Git for Windows' bash.
#[path = "../../../tests/rust-support/script_stub.rs"]
mod script_stub;

/// A binary's file name in the staged tree: the crate's binary plus the
/// platform's executable suffix (`.exe` on Windows).
fn staged_name(binary: &str) -> String {
    format!("{binary}{}", std::env::consts::EXE_SUFFIX)
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
        "setup-dev-env-flow-{tag}-{}-{:?}",
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
    /// A scratch git repo with a stub planning/scripts/ and stub
    /// generated-artifact scripts (build-plan-libs.sh, generate-reviewer.sh,
    /// generate-portability.sh -- each a plain `exit 0`), so the
    /// generated-artifact section of a real run does not fail outright
    /// against an otherwise-empty tree.
    fn new(tag: &str) -> Self {
        let dir = unique_dir(tag);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q"]);
        // A root virtual workspace (members = ["src/*"]) -- without it,
        // cargo treats each src/<crate>/Cargo.toml as a standalone package
        // and places target/ INSIDE the crate's own directory rather than
        // at repo_root/target, which is where a build's output is expected.
        write_file(
            &dir.join("Cargo.toml"),
            "[workspace]\nmembers = [\"src/*\"]\nresolver = \"2\"\n",
        );
        fs::create_dir_all(dir.join("planning/scripts")).unwrap();
        write_executable(
            &dir.join("planning/scripts/build-plan-libs.sh"),
            "#!/usr/bin/env bash\nexit 0\n",
        );
        write_executable(
            &dir.join("planning/scripts/generate-reviewer.sh"),
            "#!/usr/bin/env bash\nexit 0\n",
        );
        write_executable(
            &dir.join("generate-portability.sh"),
            "#!/usr/bin/env bash\nexit 0\n",
        );
        Repo { dir }
    }

    /// A trivial crate under src/<name>, `ok` controlling whether its own
    /// source actually compiles.
    fn add_crate(&self, name: &str, ok: bool) {
        let crate_dir = self.dir.join("src").join(name);
        write_file(
            &crate_dir.join("Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        );
        let body = if ok {
            "fn main() { println!(\"ok\"); }\n".to_string()
        } else {
            "fn main( this does not parse as rust\n".to_string()
        };
        write_file(&crate_dir.join("src/main.rs"), &body);
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_setup-dev-env"))
            .args(args)
            .env("SETUP_DEV_ENV_IN_NIX", "1")
            .env("PLANNING_SKILL_ROOT", &self.dir)
            .current_dir(&self.dir)
            .output()
            .unwrap()
    }

    fn hooks_path(&self) -> Option<String> {
        let out = Command::new("git")
            .args(["config", "core.hooksPath"])
            .current_dir(&self.dir)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Extracts the "host target: <triple>" line every --list/--check run
/// prints first, so tests do not duplicate this crate's own triple
/// resolution logic.
fn host_triple_from(output: &Output) -> String {
    stdout_of(output)
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("host target: "))
        .expect("no host target line in output")
        .to_string()
}

fn real_repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

// The build loop is driven by iterating plan::plan()'s own fixed table: a
// directory under src/ that is NOT one of the plan's own rows is simply
// never visited by this loop, even though it is a perfectly valid cargo
// workspace member otherwise. So a "dummy crate" usable by the build loop
// must reuse one of the table's own real crate names -- their PRODUCTION
// identity is irrelevant here, since every test below builds a totally
// isolated fixture repository under its own scratch directory.
const OK_CRATE: &str = "add-goal";
const FAILING_CRATE: &str = "cleanup-plans";
// A real plan-table crate name that also triggers the skill-dir extra-copy
// branch (bug-report/todo/interactive-shell), for the print-ordering
// regression test below.
const SKILL_DIR_CRATE: &str = "todo";

#[test]
fn list_includes_every_planned_row_including_dummy_crates_and_self() {
    let repo = Repo::new("list");
    repo.add_crate(OK_CRATE, true);
    let output = repo.run(&["--list"]);
    assert!(output.status.success());
    let text = stdout_of(&output);
    assert!(text.contains(OK_CRATE), "missing dummy crate row:\n{text}");
    assert!(
        text.contains("setup-dev-env"),
        "missing the self-hosting row:\n{text}"
    );
}

#[test]
fn check_reports_missing_before_a_build_and_present_after() {
    let repo = Repo::new("check-before-after");
    repo.add_crate(OK_CRATE, true);

    let before = repo.run(&["--check"]);
    assert!(before.status.success());
    let before_text = stdout_of(&before);
    let row = before_text
        .lines()
        .find(|l| l.contains(OK_CRATE) && !l.contains("planning/scripts"))
        .expect("no row in --check output");
    assert!(row.contains("MISSING"), "expected MISSING: {row}");

    let build = repo.run(&[]);
    assert!(
        build.status.success(),
        "build failed: {}",
        stderr_of(&build)
    );

    let after = repo.run(&["--check"]);
    assert!(after.status.success());
    let after_text = stdout_of(&after);
    let row = after_text
        .lines()
        .find(|l| l.contains(OK_CRATE) && !l.contains("planning/scripts"))
        .expect("no row in --check output");
    assert!(row.contains("present"), "expected present: {row}");
}

#[test]
fn a_full_run_builds_and_stages_the_dummy_crate_binary() {
    let repo = Repo::new("full-run");
    repo.add_crate(OK_CRATE, true);

    let output = repo.run(&[]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    let triple = host_triple_of_check(&repo);
    let staged = repo
        .dir
        .join("bin")
        .join(&triple)
        .join(staged_name(OK_CRATE));
    assert!(staged.is_file(), "binary was not staged at {staged:?}");
    assert!(repo.dir.join(".setup-dev-env.finished").is_file());
}

/// Regression test: a real bug found during goal 17's own regression sweep.
/// stage_extras (the planning/scripts sibling and skill-dir copies) used to
/// run and print its own "   -> ..." lines BEFORE stage_primary's "ok ->
/// bin/..." line, the wrong order. Assert the exact line order for a crate
/// that triggers the skill-dir branch.
#[test]
fn the_ok_line_prints_before_the_skill_dir_extra_copy_line() {
    let repo = Repo::new("print-order");
    repo.add_crate(SKILL_DIR_CRATE, true);

    let output = repo.run(&[]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    let text = stdout_of(&output);
    let ok_pos = text
        .find("ok -> bin/")
        .expect("no 'ok -> bin/...' line in output");
    let extra_pos = text
        .find(&format!("-> {SKILL_DIR_CRATE}/bin/"))
        .expect("no skill-dir extra-copy line in output");
    assert!(
        ok_pos < extra_pos,
        "expected 'ok -> bin/...' before the skill-dir extra-copy line, got:\n{text}"
    );
}

fn host_triple_of_check(repo: &Repo) -> String {
    host_triple_from(&repo.run(&["--check"]))
}

#[test]
fn a_failing_crate_is_reported_and_the_run_still_processes_the_rest() {
    let repo = Repo::new("failing-crate");
    repo.add_crate(OK_CRATE, true);
    repo.add_crate(FAILING_CRATE, false);

    let output = repo.run(&[]);
    assert!(!output.status.success(), "expected a non-zero exit");
    let combined = format!("{}{}", stdout_of(&output), stderr_of(&output));
    assert!(combined.contains("FAILED"), "no FAILED marker:\n{combined}");
    assert!(
        combined.contains(FAILING_CRATE),
        "failing crate not named:\n{combined}"
    );

    let triple = host_triple_of_check(&repo);
    let ok_staged = repo
        .dir
        .join("bin")
        .join(&triple)
        .join(staged_name(OK_CRATE));
    assert!(
        ok_staged.is_file(),
        "the other, valid crate should still have built and staged"
    );
    assert!(
        !repo.dir.join(".setup-dev-env.finished").is_file(),
        ".finished must be absent after a failing crate"
    );
}

#[test]
fn git_hooks_path_is_set_on_the_fixture_never_the_real_repository() {
    let repo = Repo::new("hooks-path");
    repo.add_crate(OK_CRATE, true);

    let real_before = git_config_hooks_path(&real_repo_root());
    let output = repo.run(&[]);
    assert!(output.status.success(), "{}", stderr_of(&output));

    assert_eq!(repo.hooks_path().as_deref(), Some("hooks"));
    let real_after = git_config_hooks_path(&real_repo_root());
    assert_eq!(
        real_before, real_after,
        "the real ai-skills repository's own core.hooksPath must be unaffected by this test run"
    );
}

fn git_config_hooks_path(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["config", "core.hooksPath"])
        .current_dir(dir)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[test]
fn rebuilding_and_restaging_the_same_crate_succeeds_via_write_then_rename() {
    let repo = Repo::new("rebuild");
    repo.add_crate(OK_CRATE, true);

    let first = repo.run(&[]);
    assert!(first.status.success(), "{}", stderr_of(&first));

    // Simulate a source edit and rebuild the same crate a second time --
    // the write-then-rename staging path must replace the existing staged
    // binary cleanly.
    repo.add_crate(OK_CRATE, true);
    let second = repo.run(&[]);
    assert!(second.status.success(), "{}", stderr_of(&second));

    let triple = host_triple_of_check(&repo);
    let staged = repo
        .dir
        .join("bin")
        .join(&triple)
        .join(staged_name(OK_CRATE));
    assert!(staged.is_file());
}

/// B94/AR-58: this crate's own plan.rs is a second source of truth
/// alongside setup-dev-env-lib.sh's real plan() -- diff the two against
/// this repository's own real copy of setup-dev-env-lib.sh (read-only, a
/// fixture read; never executed against a live repo_root) to catch the
/// exact silent-degradation drift this whole bootstrapper exists to cure.
#[test]
fn the_embedded_plan_table_matches_the_real_bash_plan_function() {
    let repo_root = real_repo_root();
    let lib = repo_root.join("setup-dev-env-lib.sh");
    assert!(lib.is_file(), "expected {lib:?} to exist");

    // Forward slashes: bash does not reliably read a backslash path.
    let script = format!(
        "set -euo pipefail\nsource {:?}\nplan\n",
        lib.to_string_lossy().replace('\\', "/")
    );
    let output = Command::new(script_stub::bash_program())
        .arg("-c")
        .arg(&script)
        .output()
        .expect("failed to run bash plan()");
    assert!(output.status.success(), "{}", stderr_of(&output));
    let mut bash_rows: Vec<(String, String)> = stdout_of(&output)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut parts = line.splitn(2, '\t');
            (
                parts.next().unwrap().to_string(),
                parts.next().unwrap().to_string(),
            )
        })
        .collect();
    bash_rows.sort();

    // Build a throwaway fixture whose only job is to make --list resolve a
    // host triple without requiring nix; the plan TABLE itself does not
    // depend on the fixture's own contents.
    let repo = Repo::new("drift-check");
    let list = repo.run(&["--list"]);
    assert!(list.status.success());
    let mut rust_rows: Vec<(String, String)> = stdout_of(&list)
        .lines()
        .skip(1)
        .filter(|line| line.starts_with("  ") && line.contains("->"))
        .filter_map(|line| {
            let line = line.trim_start();
            let mut parts = line.splitn(2, "->");
            let crate_name = parts.next()?.trim().to_string();
            let dest = parts.next()?.trim();
            if dest.starts_with("planning/scripts/") {
                return None; // the sibling-copy line, not a plan row of its own
            }
            // The plan's own rows name the binary without the platform's
            // suffix; --list prints the staged name.
            let staged = dest.rsplit('/').next()?;
            let binary = staged
                .strip_suffix(std::env::consts::EXE_SUFFIX)
                .unwrap_or(staged)
                .to_string();
            Some((crate_name, binary))
        })
        .collect();
    rust_rows.sort();

    assert_eq!(
        bash_rows, rust_rows,
        "the embedded plan.rs table has drifted from setup-dev-env-lib.sh's real plan()"
    );
}
