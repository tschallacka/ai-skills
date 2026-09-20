// MODE: DEV
// Real-subprocess integration coverage for the compiled ci-scope binary:
// scratch git repos and synthetic cargo workspaces for the branches this
// crate's own logic decides, plus a real-tree parity test running every one
// of `.github/tests/test-ci-scope.sh`'s own scenarios against both bash and
// the compiled binary. AR-82: every spawn explicitly sets or removes
// GITHUB_OUTPUT rather than inheriting the test process's own ambient value.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

// `bash_program()`/`bash_script()`: on Windows a bare `Command::new("bash")`
// finds System32's WSL launcher before Git for Windows' bash.
#[path = "../../../tests/rust-support/script_stub.rs"]
mod script_stub;

fn unique_dir(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "ci-scope-flow-{tag}-{}-{:?}",
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
        Repo { dir }
    }

    fn git_init(tag: &str) -> Self {
        let repo = Self::new(tag);
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

    fn git_output(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.dir)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?} failed");
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    fn write(&self, rel_path: &str, content: &str) {
        let path = self.dir.join(rel_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
    }

    fn commit_all(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message]);
    }

    /// A minimal virtual cargo workspace with path-dependency edges, so
    /// `cargo metadata` (and this crate's own reverse-dependency closure) has
    /// something real to compute against without any network access:
    /// `edges` are `(dependent, dependency)` pairs. Members live under
    /// `src/<name>/`, matching this real repository's own convention that
    /// the crate-extraction logic (`$1 == "src"`) hardcodes.
    fn workspace(&self, members: &[&str], edges: &[(&str, &str)]) {
        let member_list = members
            .iter()
            .map(|m| format!("\"src/{m}\""))
            .collect::<Vec<_>>()
            .join(", ");
        self.write(
            "Cargo.toml",
            &format!("[workspace]\nresolver = \"2\"\nmembers = [{member_list}]\n"),
        );
        for member in members {
            let deps: String = edges
                .iter()
                .filter(|(dependent, _)| dependent == member)
                .map(|(_, dependency)| format!("{dependency} = {{ path = \"../{dependency}\" }}\n"))
                .collect();
            self.write(
                &format!("src/{member}/Cargo.toml"),
                &format!("[package]\nname = \"{member}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{deps}"),
            );
            self.write(&format!("src/{member}/src/lib.rs"), "// empty\n");
        }
    }

    fn files_from(&self, name: &str, lines: &[&str]) -> PathBuf {
        let path = self.dir.join(name);
        let mut content = lines.join("\n");
        if !lines.is_empty() {
            content.push('\n');
        }
        fs::write(&path, content).unwrap();
        path
    }

    /// Spawns the real compiled binary with `PLANNING_SKILL_ROOT` pointed at
    /// this scratch repo and GITHUB_OUTPUT explicitly removed (AR-82).
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ci-scope"))
            .args(args)
            .current_dir(&self.dir)
            .env("PLANNING_SKILL_ROOT", &self.dir)
            .env_remove("GITHUB_OUTPUT")
            .env_remove("CI_SCOPE_THRESHOLD")
            .env_remove("CI_SCOPE_DIVISOR")
            .env_remove("CI_SCOPE_FLOOR")
            .output()
            .unwrap()
    }

    fn run_with_path(&self, args: &[&str], path: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ci-scope"))
            .args(args)
            .current_dir(&self.dir)
            .env("PLANNING_SKILL_ROOT", &self.dir)
            .env("PATH", path)
            .env_remove("GITHUB_OUTPUT")
            .output()
            .unwrap()
    }

    fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn scope_of(output: &Output) -> String {
    stdout_of(output)
        .lines()
        .find_map(|line| line.strip_prefix("scope="))
        .unwrap_or("")
        .to_string()
}

// ---- usage --------------------------------------------------------------

#[test]
fn help_prints_the_embedded_text_and_exits_0() {
    let repo = Repo::new("help");
    let output = repo.run(&["--help"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).starts_with("ci-scope.sh"));
    repo.cleanup();
}

#[test]
fn an_unknown_flag_exits_64() {
    let repo = Repo::new("unknown-flag");
    let output = repo.run(&["--nonsense"]);
    assert_eq!(output.status.code(), Some(64));
    // AR-85: bash's own usage() prints the full usage text to stdout on
    // EVERY exit path, not only -h/--help -- confirm the Rust port does too,
    // not just that it exits 64.
    assert!(stdout_of(&output).starts_with("ci-scope.sh"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown argument: --nonsense"));
    repo.cleanup();
}

#[test]
fn missing_value_flags_exit_64_and_print_usage_to_stdout() {
    let repo = Repo::new("missing-value");
    for flag in ["--base", "--files-from", "--threshold", "--push-to"] {
        let output = repo.run(&[flag]);
        assert_eq!(output.status.code(), Some(64), "flag: {flag}");
        // AR-85: matching bash's own usage() dumping the full text to
        // stdout for a missing flag value too, not only for --help.
        assert!(
            stdout_of(&output).starts_with("ci-scope.sh"),
            "flag: {flag}, stdout: {}",
            stdout_of(&output)
        );
    }
    repo.cleanup();
}

// ---- push-to --------------------------------------------------------------

#[test]
fn push_to_forces_full_even_with_an_empty_change_set() {
    let repo = Repo::new("push-to");
    let empty = repo.files_from("empty.txt", &[]);
    let output = repo.run(&[
        "--push-to",
        "master",
        "--files-from",
        empty.to_str().unwrap(),
    ]);
    assert_eq!(scope_of(&output), "full");
    repo.cleanup();
}

// ---- the change set ---------------------------------------------------

#[test]
fn files_from_an_unreadable_path_forces_full() {
    let repo = Repo::new("unreadable");
    let output = repo.run(&["--files-from", "does-not-exist.txt"]);
    assert_eq!(scope_of(&output), "full");
    repo.cleanup();
}

#[test]
fn a_relative_files_from_path_resolves_against_planning_skill_root_not_process_cwd() {
    // AR-90: bash `cd`s into repo_root BEFORE reading a relative
    // --files-from path, so it resolves against repo_root, not the
    // caller's original working directory. Prove that genuinely, with a
    // process cwd DIFFERENT from PLANNING_SKILL_ROOT and a relative
    // filename that exists only under the latter.
    let root = Repo::new("relative-root");
    let elsewhere = Repo::new("relative-elsewhere");
    root.files_from("changed.txt", &["src/rjq/src/main.rs"]);
    let output = Command::new(env!("CARGO_BIN_EXE_ci-scope"))
        .args(["--files-from", "changed.txt"])
        .current_dir(&elsewhere.dir)
        .env("PLANNING_SKILL_ROOT", &root.dir)
        .env_remove("GITHUB_OUTPUT")
        .output()
        .unwrap();
    // `root.dir` has no Cargo.toml, so once the file IS found the flow
    // reaches (and fails at) the cargo-metadata step -- a different, later
    // reason than "cannot read the change set", which is exactly what a
    // process-cwd-relative (wrong) resolution would produce instead.
    let out = stdout_of(&output);
    assert!(
        !out.contains("cannot read the change set from changed.txt"),
        "the relative --files-from path was not found under PLANNING_SKILL_ROOT: {out}"
    );
    assert!(
        out.contains("cargo metadata failed") || out.contains("cargo is not on PATH"),
        "expected the flow to reach the cargo-metadata step, got: {out}"
    );
    root.cleanup();
    elsewhere.cleanup();
}

#[test]
fn no_changes_at_all_goes_none() {
    let repo = Repo::new("no-changes");
    let empty = repo.files_from("empty.txt", &[]);
    let output = repo.run(&["--files-from", empty.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "none");
    repo.cleanup();
}

// ---- global inputs ------------------------------------------------------

#[test]
fn a_root_manifest_change_forces_full() {
    let repo = Repo::new("global-manifest");
    let f = repo.files_from("changed.txt", &["Cargo.toml"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "full");
    repo.cleanup();
}

#[test]
fn a_dot_github_change_forces_full() {
    let repo = Repo::new("global-github");
    let f = repo.files_from("changed.txt", &[".github/workflows/ci.yml"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "full");
    repo.cleanup();
}

#[test]
fn the_new_self_protection_arm_forces_full_for_its_own_source() {
    let repo = Repo::new("self-protection");
    let f = repo.files_from("changed.txt", &["src/ci-scope/src/main.rs"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "full");
    repo.cleanup();
}

#[test]
fn b300_exempt_paths_need_no_crate_rebuild() {
    let repo = Repo::new("b300");
    for path in ["installer/src/50-manifest.sh", "install.sh", "package.json"] {
        let f = repo.files_from("changed.txt", &[path]);
        let output = repo.run(&["--files-from", f.to_str().unwrap()]);
        assert_eq!(scope_of(&output), "none", "path: {path}");
    }
    repo.cleanup();
}

#[test]
fn a_doc_only_change_goes_none() {
    let repo = Repo::new("doc-only");
    let f = repo.files_from("changed.txt", &["README.md"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "none");
    repo.cleanup();
}

#[test]
fn a_huge_change_set_is_not_trusted_to_selection() {
    let repo = Repo::new("huge");
    let files: Vec<String> = (0..101).map(|i| format!("docs/file-{i}.md")).collect();
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let f = repo.files_from("changed.txt", &refs);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "full");
    repo.cleanup();
}

#[test]
fn an_unusable_threshold_still_yields_a_decision() {
    let repo = Repo::new("junk-threshold");
    let empty = repo.files_from("empty.txt", &[]);
    let output = repo.run(&[
        "--files-from",
        empty.to_str().unwrap(),
        "--threshold",
        "abc",
    ]);
    assert!(output.status.success());
    assert_eq!(scope_of(&output), "none");
    repo.cleanup();
}

// ---- the reverse-dependency closure -------------------------------------

#[test]
fn a_leaf_crate_change_goes_selective() {
    let repo = Repo::new("leaf");
    repo.workspace(&["leaf"], &[]);
    let f = repo.files_from("changed.txt", &["src/leaf/src/lib.rs"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "selective");
    assert!(stdout_of(&output).contains("crates=leaf"));
    repo.cleanup();
}

#[test]
fn a_dependency_change_pulls_in_its_transitive_dependents() {
    let repo = Repo::new("closure");
    // c depends on b, b depends on a: changing a must select a, b, and c.
    repo.workspace(&["a", "b", "c"], &[("b", "a"), ("c", "b")]);
    let f = repo.files_from("changed.txt", &["src/a/src/lib.rs"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap(), "--threshold", "10"]);
    assert_eq!(scope_of(&output), "selective");
    let out = stdout_of(&output);
    let crates_line = out.lines().find(|l| l.starts_with("crates=")).unwrap();
    assert_eq!(crates_line, "crates=a b c");
    repo.cleanup();
}

#[test]
fn selected_count_exactly_at_the_threshold_does_not_force_full() {
    let repo = Repo::new("at-threshold");
    repo.workspace(&["a", "b"], &[("b", "a")]);
    let f = repo.files_from("changed.txt", &["src/a/src/lib.rs"]);
    // selected = {a, b} = 2; threshold pinned to exactly 2.
    let output = repo.run(&["--files-from", f.to_str().unwrap(), "--threshold", "2"]);
    assert_eq!(scope_of(&output), "selective");
    repo.cleanup();
}

#[test]
fn selected_count_over_the_threshold_forces_full() {
    let repo = Repo::new("over-threshold");
    repo.workspace(&["a", "b"], &[("b", "a")]);
    let f = repo.files_from("changed.txt", &["src/a/src/lib.rs"]);
    // selected = {a, b} = 2; threshold pinned to 1, so 2 > 1.
    let output = repo.run(&["--files-from", f.to_str().unwrap(), "--threshold", "1"]);
    assert_eq!(scope_of(&output), "full");
    repo.cleanup();
}

#[test]
fn cargo_not_on_path_forces_full() {
    let repo = Repo::new("no-cargo");
    repo.workspace(&["leaf"], &[]);
    let f = repo.files_from("changed.txt", &["src/leaf/src/lib.rs"]);
    // A PATH with only the directories git itself needs, and none holding
    // a `cargo` binary -- proof the fail-open branch is genuinely reached
    // rather than assumed.
    let output = repo.run_with_path(&["--files-from", f.to_str().unwrap()], "/usr/bin:/bin");
    assert_eq!(scope_of(&output), "full");
    assert!(stdout_of(&output).contains("cargo is not on PATH"));
    repo.cleanup();
}

// ---- the real git-diff path (no --files-from) ----------------------------

#[test]
fn the_real_git_diff_path_computes_a_selective_scope() {
    let repo = Repo::git_init("real-git");
    repo.workspace(&["a", "b"], &[("b", "a")]);
    repo.commit_all("baseline");
    let base = repo.git_output(&["rev-parse", "HEAD"]);
    repo.write("src/a/src/lib.rs", "// changed\n");
    repo.commit_all("change a");
    let output = repo.run(&["--base", &base, "--threshold", "10"]);
    assert_eq!(scope_of(&output), "selective");
    let out = stdout_of(&output);
    let crates_line = out.lines().find(|l| l.starts_with("crates=")).unwrap();
    assert_eq!(crates_line, "crates=a b");
    repo.cleanup();
}

#[test]
fn no_merge_base_forces_full() {
    let repo = Repo::git_init("no-merge-base");
    repo.write("README.md", "hello\n");
    repo.commit_all("only commit");
    let output = repo.run(&["--base", "refs/does-not-exist"]);
    assert_eq!(scope_of(&output), "full");
    assert!(stdout_of(&output).contains("does not resolve"));
    repo.cleanup();
}

#[test]
fn not_a_git_repository_forces_full() {
    let repo = Repo::new("not-a-repo");
    let output = repo.run(&[]);
    assert_eq!(scope_of(&output), "full");
    assert!(stdout_of(&output).contains("not a git repository"));
    repo.cleanup();
}

// ---- GITHUB_OUTPUT (AR-82/AR-84, applied proactively) --------------------

#[test]
fn github_output_is_appended_when_set() {
    let repo = Repo::new("github-output");
    let empty = repo.files_from("empty.txt", &[]);
    let out_path = repo.dir.join("gh-output.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_ci-scope"))
        .args(["--files-from", empty.to_str().unwrap()])
        .current_dir(&repo.dir)
        .env("PLANNING_SKILL_ROOT", &repo.dir)
        .env("GITHUB_OUTPUT", &out_path)
        .output()
        .unwrap();
    let appended = fs::read_to_string(&out_path).unwrap();
    assert_eq!(appended, stdout_of(&output));
    repo.cleanup();
}

#[test]
fn no_github_output_leaves_stdout_unaffected() {
    let repo = Repo::new("no-github-output");
    let empty = repo.files_from("empty.txt", &[]);
    let output = repo.run(&["--files-from", empty.to_str().unwrap()]);
    assert!(output.status.success());
    assert_eq!(scope_of(&output), "none");
    repo.cleanup();
}

#[test]
fn an_unwritable_github_output_still_exits_zero() {
    let repo = Repo::new("unwritable-github-output");
    let empty = repo.files_from("empty.txt", &[]);
    let output = Command::new(env!("CARGO_BIN_EXE_ci-scope"))
        .args(["--files-from", empty.to_str().unwrap()])
        .current_dir(&repo.dir)
        .env("PLANNING_SKILL_ROOT", &repo.dir)
        .env("GITHUB_OUTPUT", "/nonexistent-directory/gh-output.txt")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(scope_of(&output), "none");
    repo.cleanup();
}

// ---- real-tree exec fidelity and the missing-binary fallback (W122: the
// bash reimplementation body is gone, so there is nothing left to compare it
// against -- what remains to prove is that invoking .github/ci-scope.sh, which
// execs the compiled binary via its own wiring block, produces byte-identical
// output to invoking the compiled binary directly, and that the wiring's own
// safe-default fires when no compiled binary can be found) --

fn real_repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// A scratch bin directory holding exactly the ci-scope this test binary was
/// built alongside (`CARGO_BIN_EXE_ci-scope`), pinned onto AI_SKILLS_BIN_ROOT
/// (tier 1) for the wrapper invocation below. That is the very binary this
/// test asserts fidelity against, and staging it here means the test needs no
/// `./setup-dev-env.sh` output: it used to read `<repo>/bin/<triple>/ci-scope`,
/// which a fresh CI checkout running `cargo test --workspace` does not have, so
/// it failed there with "bin/ directory (run ./setup-dev-env.sh)". A stale or
/// partial shared install under ~/.config/tsch-ai-skills/bin (tier 2) still
/// cannot shadow it, for the same reason as before.
///
/// The directory has to hold ONLY that binary, so it is not the build
/// directory itself (which carries whatever else was once built there), and it
/// is created beside the built binary rather than under the temp directory, so
/// `cargo clean` removes it and repeated runs leave nothing behind in $TMPDIR.
fn staged_bin_dir(_repo_root: &Path) -> PathBuf {
    static STAGED: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    STAGED
        .get_or_init(|| {
            let built = Path::new(env!("CARGO_BIN_EXE_ci-scope"));
            let dir = built
                .parent()
                .expect("a built binary lives in a directory")
                .join("ci-scope-staged-bin");
            fs::create_dir_all(&dir).unwrap();
            // The wrapper looks the binary up as `<dir>/ci-scope`; on Windows
            // the file is `ci-scope.exe`, which Git bash resolves the same way.
            fs::copy(
                built,
                dir.join(format!("ci-scope{}", std::env::consts::EXE_SUFFIX)),
            )
            .unwrap();
            dir
        })
        .clone()
}

fn wrapper_scope(repo_root: &Path, args: &[&str]) -> Output {
    script_stub::bash_script(&repo_root.join(".github/ci-scope.sh"))
        .args(args)
        .current_dir(repo_root)
        // Forward slashes: a path bash reads out of its environment is safest
        // in the form bash itself would write.
        .env(
            "AI_SKILLS_BIN_ROOT",
            staged_bin_dir(repo_root)
                .to_string_lossy()
                .replace('\\', "/"),
        )
        .env_remove("GITHUB_OUTPUT")
        .output()
        .unwrap()
}

fn direct_scope(repo_root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ci-scope"))
        .args(args)
        .current_dir(repo_root)
        .env("PLANNING_SKILL_ROOT", repo_root)
        .env_remove("GITHUB_OUTPUT")
        .output()
        .unwrap()
}

#[test]
fn exec_fidelity_matches_the_compiled_binary_for_every_test_ci_scope_scenario() {
    let repo_root = real_repo_root();
    let work = unique_dir("exec-fidelity");
    fs::create_dir_all(&work).unwrap();

    let scenarios: &[(&str, &[&str])] = &[
        ("the root manifest", &["Cargo.toml"]),
        ("the lock file", &["Cargo.lock"]),
        ("the toolchain file", &["rust-toolchain.toml"]),
        ("the flake", &["flake.nix"]),
        ("a workflow", &[".github/workflows/ci.yml"]),
        ("the installer", &["installer/src/50-manifest.sh"]),
        ("the generated install", &["install.sh"]),
        ("package.json", &["package.json"]),
        ("the selector itself", &[".github/ci-scope.sh"]),
        ("the subject mapper", &[".github/ci-subjects.sh"]),
        ("its own tests", &[".github/tests/test-ci-scope.sh"]),
        ("no files at all", &[]),
        ("a doc-only change", &["README.md"]),
        ("a skill-only change", &["chat/SKILL.md"]),
        ("a leaf crate", &["src/rjq/src/main.rs"]),
    ];

    for (label, files) in scenarios {
        let list_path = work.join(format!("{}.txt", label.replace(' ', "-")));
        let mut content = files.join("\n");
        if !files.is_empty() {
            content.push('\n');
        }
        fs::write(&list_path, content).unwrap();

        let wrapper_out = wrapper_scope(&repo_root, &["--files-from", list_path.to_str().unwrap()]);
        let direct_out = direct_scope(&repo_root, &["--files-from", list_path.to_str().unwrap()]);
        assert_eq!(
            stdout_of(&wrapper_out),
            stdout_of(&direct_out),
            "scenario: {label}"
        );
    }

    let huge_list = work.join("huge.txt");
    let huge_content: String = (0..101).map(|i| format!("docs/file-{i}.md\n")).collect();
    fs::write(&huge_list, huge_content).unwrap();
    let wrapper_huge = wrapper_scope(&repo_root, &["--files-from", huge_list.to_str().unwrap()]);
    let direct_huge = direct_scope(&repo_root, &["--files-from", huge_list.to_str().unwrap()]);
    assert_eq!(
        stdout_of(&wrapper_huge),
        stdout_of(&direct_huge),
        "scenario: a huge change set"
    );

    let empty_list = work.join("empty-for-threshold.txt");
    fs::write(&empty_list, "").unwrap();
    let wrapper_junk_threshold = wrapper_scope(
        &repo_root,
        &[
            "--files-from",
            empty_list.to_str().unwrap(),
            "--threshold",
            "abc",
        ],
    );
    let direct_junk_threshold = direct_scope(
        &repo_root,
        &[
            "--files-from",
            empty_list.to_str().unwrap(),
            "--threshold",
            "abc",
        ],
    );
    assert_eq!(
        stdout_of(&wrapper_junk_threshold),
        stdout_of(&direct_junk_threshold),
        "scenario: a junk threshold"
    );

    let wrapper_unknown_flag = wrapper_scope(&repo_root, &["--nonsense"]);
    let direct_unknown_flag = direct_scope(&repo_root, &["--nonsense"]);
    assert_eq!(
        stdout_of(&wrapper_unknown_flag),
        stdout_of(&direct_unknown_flag),
        "scenario: an unknown flag"
    );
    assert_eq!(
        wrapper_unknown_flag.status.code(),
        direct_unknown_flag.status.code(),
        "scenario: an unknown flag, exit code"
    );

    for branch in ["master", "nextupdate"] {
        let wrapper_out = wrapper_scope(&repo_root, &["--push-to", branch]);
        let direct_out = direct_scope(&repo_root, &["--push-to", branch]);
        assert_eq!(
            stdout_of(&wrapper_out),
            stdout_of(&direct_out),
            "branch: {branch}"
        );
    }

    let wrapper_help = wrapper_scope(&repo_root, &["--help"]);
    let direct_help = direct_scope(&repo_root, &["--help"]);
    assert_eq!(stdout_of(&wrapper_help), stdout_of(&direct_help));

    let _ = fs::remove_dir_all(&work);
}

// AR-100: this must not mutate the real, shared planning/scripts/plan-core-lib.sh
// in place -- moving it aside races with any other test in this same binary
// that runs concurrently and expects it present. Instead, copy ci-scope.sh into
// a per-test scratch tree whose planning/scripts/ has no plan-core-lib.sh, so
// the wiring's own [ -f .../plan-core-lib.sh ] check is false there with zero
// shared mutable state touched.
#[test]
fn missing_binary_falls_back_to_the_scope_full_safe_default() {
    let real_repo_root = real_repo_root();
    let scratch = unique_dir("missing-binary");
    fs::create_dir_all(scratch.join(".github")).unwrap();
    fs::create_dir_all(scratch.join("planning/scripts")).unwrap();
    fs::copy(
        real_repo_root.join(".github/ci-scope.sh"),
        scratch.join(".github/ci-scope.sh"),
    )
    .unwrap();

    let output = script_stub::bash_script(&scratch.join(".github/ci-scope.sh"))
        .arg("--files-from")
        .arg("/dev/null")
        .current_dir(&scratch)
        .env_remove("GITHUB_OUTPUT")
        .output()
        .unwrap();

    assert!(output.status.success());
    let out = stdout_of(&output);
    assert!(out.contains("scope=full"), "stdout: {out}");
    assert!(
        out.contains("reason=ci-scope binary not found; run ./setup-dev-env.sh to build it"),
        "stdout: {out}"
    );
    assert!(
        out.contains("crates=\n") || out.ends_with("crates=\n"),
        "stdout: {out}"
    );

    let help = script_stub::bash_script(&scratch.join(".github/ci-scope.sh"))
        .arg("--help")
        .current_dir(&scratch)
        .output()
        .unwrap();
    let real_help = script_stub::bash_script(&real_repo_root.join(".github/ci-scope.sh"))
        .arg("--help")
        .current_dir(&real_repo_root)
        .output()
        .unwrap();
    assert_eq!(stdout_of(&help), stdout_of(&real_help));

    let _ = fs::remove_dir_all(&scratch);
}
