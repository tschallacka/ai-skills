// MODE: DEV
// Real-subprocess integration coverage for the compiled ci-test-scope
// binary: scratch git repos with a synthetic `run-tests.sh --list-only`
// stub for the branches this crate's own logic decides, plus a real-tree
// parity test running every one of `.github/tests/test-ci-test-scope.sh`'s
// own scenarios against both bash and the compiled binary. AR-82: every
// spawn explicitly sets or removes GITHUB_OUTPUT rather than inheriting the
// test process's own ambient value.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_dir(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "ci-test-scope-flow-{tag}-{}-{:?}",
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

    fn files_from(&self, name: &str, lines: &[&str]) -> PathBuf {
        let path = self.dir.join(name);
        let mut content = lines.join("\n");
        if !lines.is_empty() {
            content.push('\n');
        }
        fs::write(&path, content).unwrap();
        path
    }

    /// A synthetic `run-tests.sh --list-only` stub: unconditionally prints
    /// `items`, one per line, regardless of its own arguments -- the real
    /// crate always calls it with `--list-only`, so a scratch double does
    /// not need to branch on that itself.
    fn stub_run_tests(&self, items: &[&str]) {
        let mut script = String::from("#!/usr/bin/env bash\n");
        for item in items {
            script.push_str("printf '%s\\n' '");
            script.push_str(&item.replace('\'', "'\\''"));
            script.push_str("'\n");
        }
        self.write("run-tests.sh", &script);
    }

    /// A synthetic `run-tests.sh` that lists nothing at all.
    fn stub_run_tests_empty(&self) {
        self.write("run-tests.sh", "#!/usr/bin/env bash\n");
    }

    /// Spawns the real compiled binary with `PLANNING_SKILL_ROOT` pointed at
    /// this scratch repo and GITHUB_OUTPUT explicitly removed (AR-82).
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ci-test-scope"))
            .args(args)
            .current_dir(&self.dir)
            .env("PLANNING_SKILL_ROOT", &self.dir)
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

fn tests_line(output: &Output) -> String {
    stdout_of(output)
        .lines()
        .find_map(|line| line.strip_prefix("tests="))
        .unwrap_or("")
        .to_string()
}

// ---- usage --------------------------------------------------------------

#[test]
fn help_prints_the_embedded_text_and_exits_0() {
    let repo = Repo::new("help");
    let output = repo.run(&["--help"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).starts_with("ci-test-scope.sh"));
    repo.cleanup();
}

#[test]
fn an_unknown_flag_exits_64() {
    let repo = Repo::new("unknown-flag");
    let output = repo.run(&["--nonsense"]);
    assert_eq!(output.status.code(), Some(64));
    // Matching goal 21's own AR-85 lesson, applied proactively: bash's own
    // usage() prints the full usage text to stdout on EVERY exit path, not
    // only -h/--help.
    assert!(stdout_of(&output).starts_with("ci-test-scope.sh"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown argument: --nonsense"));
    repo.cleanup();
}

#[test]
fn missing_value_flags_exit_64_and_print_usage_to_stdout() {
    let repo = Repo::new("missing-value");
    for flag in ["--base", "--files-from", "--push-to"] {
        let output = repo.run(&[flag]);
        assert_eq!(output.status.code(), Some(64), "flag: {flag}");
        assert!(
            stdout_of(&output).starts_with("ci-test-scope.sh"),
            "flag: {flag}, stdout: {}",
            stdout_of(&output)
        );
    }
    repo.cleanup();
}

#[test]
fn a_threshold_flag_is_rejected_as_unknown() {
    // ci-test-scope.sh has no --threshold concept at all, unlike
    // ci-scope.sh -- confirm the real bash script rejects it too, and that
    // the compiled binary matches.
    let repo = Repo::new("threshold-unknown");
    let output = repo.run(&["--threshold", "5"]);
    assert_eq!(output.status.code(), Some(64));
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
    // Matching AR-90's lesson from goal 21, applied proactively: bash `cd`s
    // into repo_root BEFORE reading a relative --files-from path, so it
    // resolves against repo_root, not the caller's original working
    // directory. Prove that genuinely, with a process cwd DIFFERENT from
    // PLANNING_SKILL_ROOT and a relative filename that exists only under
    // the latter.
    let root = Repo::new("relative-root");
    let elsewhere = Repo::new("relative-elsewhere");
    root.stub_run_tests(&[]);
    root.files_from("changed.txt", &["src/rjq/src/main.rs"]);
    let output = Command::new(env!("CARGO_BIN_EXE_ci-test-scope"))
        .args(["--files-from", "changed.txt"])
        .current_dir(&elsewhere.dir)
        .env("PLANNING_SKILL_ROOT", &root.dir)
        .env_remove("GITHUB_OUTPUT")
        .output()
        .unwrap();
    let out = stdout_of(&output);
    assert!(
        !out.contains("cannot read the change set from changed.txt"),
        "the relative --files-from path was not found under PLANNING_SKILL_ROOT: {out}"
    );
    root.cleanup();
    elsewhere.cleanup();
}

#[test]
fn no_changes_at_all_goes_full_not_selective_on_nothing() {
    // Unlike ci-scope.sh (which answers `none` on an empty diff),
    // ci-test-scope.sh has no `none` scope at all: nothing to narrow
    // against goes full.
    let repo = Repo::new("no-changes");
    let empty = repo.files_from("empty.txt", &[]);
    let output = repo.run(&["--files-from", empty.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "full");
    assert!(stdout_of(&output).contains("nothing to narrow against"));
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
fn run_tests_sh_itself_changing_forces_full() {
    let repo = Repo::new("global-run-tests");
    let f = repo.files_from("changed.txt", &["run-tests.sh"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "full");
    repo.cleanup();
}

#[test]
fn lib_test_sh_changing_forces_full() {
    let repo = Repo::new("global-lib-test");
    let f = repo.files_from("changed.txt", &["planning/tests/lib-test.sh"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "full");
    repo.cleanup();
}

#[test]
fn the_new_self_protection_arm_forces_full_for_its_own_source() {
    let repo = Repo::new("self-protection");
    let f = repo.files_from("changed.txt", &["src/ci-test-scope/src/main.rs"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "full");
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

// ---- run-tests.sh --list-only failure paths ------------------------------

#[test]
fn run_tests_list_only_failing_forces_full() {
    // No run-tests.sh at all in this scratch repo -> the shelled-to `bash
    // <missing path> --list-only` fails, matching the real
    // "run-tests.sh --list-only failed" branch.
    let repo = Repo::new("no-run-tests");
    let f = repo.files_from("changed.txt", &["docs/x.md"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "full");
    assert!(stdout_of(&output).contains("run-tests.sh --list-only failed"));
    repo.cleanup();
}

#[test]
fn run_tests_list_only_listing_nothing_forces_full() {
    let repo = Repo::new("empty-list");
    repo.stub_run_tests_empty();
    let f = repo.files_from("changed.txt", &["docs/x.md"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "full");
    assert!(stdout_of(&output).contains("run-tests.sh --list-only listed nothing"));
    repo.cleanup();
}

#[test]
fn run_tests_list_only_is_invoked_with_lc_all_c_regardless_of_the_ambient_locale() {
    // A real bug found while writing this goal's own real-tree parity test:
    // run-tests.sh's own shell-test discovery (`find ... | sort`) is a BARE
    // `sort` that inherits whatever locale is ambient in ITS caller's
    // environment, rather than forcing C collation itself. The real bash
    // ci-test-scope.sh already exports LC_ALL=C at its own top before
    // shelling to run-tests.sh, so that bare sort always inherits C there --
    // but this crate's own process is not guaranteed to run under LC_ALL=C
    // itself (a real CI runner, or a developer's own shell, may set any
    // locale), so list_items::list_items must set LC_ALL=C explicitly on
    // the run-tests.sh subprocess rather than letting it inherit whatever
    // is ambient. Proven directly here: a stub run-tests.sh echoes its own
    // $LC_ALL, and the ci-test-scope process is deliberately started under
    // a DIFFERENT ambient locale to prove the subprocess still sees "C".
    let repo = Repo::new("lc-all-forced");
    repo.write(
        "run-tests.sh",
        "#!/usr/bin/env bash\nprintf '%s\\n' \"$LC_ALL\"\n",
    );
    let f = repo.files_from("changed.txt", &["docs/x.md"]);
    let output = Command::new(env!("CARGO_BIN_EXE_ci-test-scope"))
        .args(["--files-from", f.to_str().unwrap()])
        .current_dir(&repo.dir)
        .env("PLANNING_SKILL_ROOT", &repo.dir)
        .env("LC_ALL", "en_US.UTF-8")
        .env_remove("GITHUB_OUTPUT")
        .output()
        .unwrap();
    assert_eq!(scope_of(&output), "selective");
    assert_eq!(tests_line(&output), "C");
    repo.cleanup();
}

// ---- COVERS-marker selection ---------------------------------------------

#[test]
fn an_undeclared_test_always_runs() {
    let repo = Repo::new("undeclared");
    repo.write("tests/plain.sh", "#!/usr/bin/env bash\necho hi\n");
    repo.stub_run_tests(&["tests/plain.sh"]);
    let f = repo.files_from("changed.txt", &["docs/unrelated.md"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "selective");
    assert_eq!(tests_line(&output), "tests/plain.sh");
    repo.cleanup();
}

#[test]
fn a_declared_test_is_kept_when_its_own_covered_path_changed() {
    let repo = Repo::new("declared-hit");
    repo.write(
        "tests/declared.sh",
        "#!/usr/bin/env bash\n# COVERS: src/foo\necho hi\n",
    );
    repo.stub_run_tests(&["tests/declared.sh"]);
    let f = repo.files_from("changed.txt", &["src/foo/main.rs"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "selective");
    assert_eq!(tests_line(&output), "tests/declared.sh");
    repo.cleanup();
}

#[test]
fn a_declared_test_is_excluded_when_unrelated_to_the_change() {
    let repo = Repo::new("declared-miss");
    repo.write("tests/kept.sh", "#!/usr/bin/env bash\necho hi\n");
    repo.write(
        "tests/excluded.sh",
        "#!/usr/bin/env bash\n# COVERS: src/bar\necho hi\n",
    );
    repo.stub_run_tests(&["tests/kept.sh", "tests/excluded.sh"]);
    let f = repo.files_from("changed.txt", &["src/foo/main.rs"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "selective");
    let tests = tests_line(&output);
    assert!(tests.contains("tests/kept.sh"));
    assert!(!tests.contains("tests/excluded.sh"));
    assert!(stdout_of(&output).contains("1 declared test(s) excluded"));
    repo.cleanup();
}

#[test]
fn a_directory_prefix_covers_entry_matches_a_file_beneath_it() {
    let repo = Repo::new("directory-prefix");
    repo.write(
        "chat/tests/test-chat.sh",
        "#!/usr/bin/env bash\n# COVERS: chat\necho hi\n",
    );
    repo.stub_run_tests(&["chat/tests/test-chat.sh"]);
    let f = repo.files_from("changed.txt", &["chat/SKILL.md"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "selective");
    assert_eq!(tests_line(&output), "chat/tests/test-chat.sh");
    repo.cleanup();
}

#[test]
fn a_crate_directory_item_is_always_undeclared() {
    let repo = Repo::new("crate-dir-item");
    fs::create_dir_all(repo.dir.join("src/some-crate")).unwrap();
    repo.stub_run_tests(&["src/some-crate"]);
    let f = repo.files_from("changed.txt", &["docs/unrelated.md"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "selective");
    assert_eq!(tests_line(&output), "src/some-crate");
    repo.cleanup();
}

#[test]
fn selection_excluding_every_item_falls_back_to_full() {
    let repo = Repo::new("all-excluded");
    repo.write(
        "tests/only.sh",
        "#!/usr/bin/env bash\n# COVERS: src/bar\necho hi\n",
    );
    repo.stub_run_tests(&["tests/only.sh"]);
    let f = repo.files_from("changed.txt", &["docs/unrelated.md"]);
    let output = repo.run(&["--files-from", f.to_str().unwrap()]);
    assert_eq!(scope_of(&output), "full");
    assert!(stdout_of(&output).contains("selection excluded every test"));
    repo.cleanup();
}

// ---- the real git-diff path (no --files-from) ----------------------------

#[test]
fn the_real_git_diff_path_computes_a_selective_scope() {
    let repo = Repo::git_init("real-git");
    repo.write("tests/plain.sh", "#!/usr/bin/env bash\necho hi\n");
    repo.stub_run_tests(&["tests/plain.sh"]);
    repo.commit_all("baseline");
    let base = repo.git_output(&["rev-parse", "HEAD"]);
    repo.write("docs/new.md", "hello\n");
    repo.commit_all("add a doc");
    let output = repo.run(&["--base", &base]);
    assert_eq!(scope_of(&output), "selective");
    assert_eq!(tests_line(&output), "tests/plain.sh");
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
    repo.stub_run_tests(&["tests/plain.sh"]);
    let empty = repo.files_from("empty.txt", &[]);
    let out_path = repo.dir.join("gh-output.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_ci-test-scope"))
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
    assert_eq!(scope_of(&output), "full");
    repo.cleanup();
}

#[test]
fn an_unwritable_github_output_still_exits_zero() {
    let repo = Repo::new("unwritable-github-output");
    let empty = repo.files_from("empty.txt", &[]);
    let output = Command::new(env!("CARGO_BIN_EXE_ci-test-scope"))
        .args(["--files-from", empty.to_str().unwrap()])
        .current_dir(&repo.dir)
        .env("PLANNING_SKILL_ROOT", &repo.dir)
        .env("GITHUB_OUTPUT", "/nonexistent-directory/gh-output.txt")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(scope_of(&output), "full");
    repo.cleanup();
}

// ---- real-tree parity: every scenario the authoritative black-box harness
// (.github/tests/test-ci-test-scope.sh) exercises, bash vs the compiled
// binary --

fn real_repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn real_bash_scope(repo_root: &Path, args: &[&str]) -> Output {
    Command::new("bash")
        .arg(repo_root.join(".github/ci-test-scope.sh"))
        .args(args)
        .current_dir(repo_root)
        .env_remove("GITHUB_OUTPUT")
        .output()
        .unwrap()
}

fn real_compiled_scope(repo_root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ci-test-scope"))
        .args(args)
        .current_dir(repo_root)
        .env("PLANNING_SKILL_ROOT", repo_root)
        .env_remove("GITHUB_OUTPUT")
        .output()
        .unwrap()
}

#[test]
fn real_tree_parity_matches_bash_for_every_test_ci_test_scope_scenario() {
    let repo_root = real_repo_root();
    let work = unique_dir("parity");
    fs::create_dir_all(&work).unwrap();

    let scenarios: &[(&str, &[&str])] = &[
        ("the root manifest", &["Cargo.toml"]),
        ("the toolchain file", &["rust-toolchain.toml"]),
        ("the flake", &["flake.nix"]),
        ("a workflow", &[".github/workflows/ci.yml"]),
        ("the selector itself", &[".github/ci-test-scope.sh"]),
        ("run-tests.sh", &["run-tests.sh"]),
        ("lib-test.sh", &["planning/tests/lib-test.sh"]),
        // Deliberately NOT included here: a `src/ci-test-scope/*` scenario.
        // That is the self-protection arm THIS goal adds (see the dedicated
        // scratch-repo test `the_new_self_protection_arm_forces_full_for_its_own_source`
        // above), and the real bash `.github/ci-test-scope.sh` on disk does
        // not gain the matching case arm until step 03 wires it -- comparing
        // against bash here, before that edit lands, would be a genuine
        // mismatch (bash still says selective) rather than a real defect.
        // Matches goal 21's own identical precedent for `ci-scope.sh`.
        ("no files at all", &[]),
        ("a doc-only change", &["README.md"]),
        (
            "a bug-report crate change",
            &["src/bug-report/src/resolve.rs"],
        ),
        ("a file under the chat directory", &["chat/SKILL.md"]),
        ("an arbitrary unrelated change", &["docs/unrelated-file.md"]),
    ];

    for (label, files) in scenarios {
        let list_path = work.join(format!("{}.txt", label.replace(' ', "-")));
        let mut content = files.join("\n");
        if !files.is_empty() {
            content.push('\n');
        }
        fs::write(&list_path, content).unwrap();

        let bash_out = real_bash_scope(&repo_root, &["--files-from", list_path.to_str().unwrap()]);
        let rust_out =
            real_compiled_scope(&repo_root, &["--files-from", list_path.to_str().unwrap()]);
        assert_eq!(
            stdout_of(&bash_out),
            stdout_of(&rust_out),
            "scenario: {label}"
        );
    }

    // The 101-file huge-change-set scenario.
    let huge_list = work.join("huge.txt");
    let huge_content: String = (0..101).map(|i| format!("docs/file-{i}.md\n")).collect();
    fs::write(&huge_list, huge_content).unwrap();
    let bash_huge = real_bash_scope(&repo_root, &["--files-from", huge_list.to_str().unwrap()]);
    let rust_huge = real_compiled_scope(&repo_root, &["--files-from", huge_list.to_str().unwrap()]);
    assert_eq!(
        stdout_of(&bash_huge),
        stdout_of(&rust_huge),
        "scenario: a huge change set"
    );

    // The unknown-flag-rejection scenario.
    let bash_unknown_flag = real_bash_scope(&repo_root, &["--nonsense"]);
    let rust_unknown_flag = real_compiled_scope(&repo_root, &["--nonsense"]);
    assert_eq!(
        stdout_of(&bash_unknown_flag),
        stdout_of(&rust_unknown_flag),
        "scenario: an unknown flag"
    );
    assert_eq!(
        bash_unknown_flag.status.code(),
        rust_unknown_flag.status.code(),
        "scenario: an unknown flag, exit code"
    );

    let _ = fs::remove_dir_all(&work);
}

#[test]
fn real_tree_parity_push_to_matches_bash() {
    let repo_root = real_repo_root();
    for branch in ["master", "nextupdate"] {
        let bash_out = real_bash_scope(&repo_root, &["--push-to", branch]);
        let rust_out = real_compiled_scope(&repo_root, &["--push-to", branch]);
        assert_eq!(
            stdout_of(&bash_out),
            stdout_of(&rust_out),
            "branch: {branch}"
        );
    }
    // And it must win over an empty change set.
    let bash_out = real_bash_scope(
        &repo_root,
        &["--push-to", "master", "--files-from", "/dev/null"],
    );
    let rust_out = real_compiled_scope(
        &repo_root,
        &["--push-to", "master", "--files-from", "/dev/null"],
    );
    assert_eq!(stdout_of(&bash_out), stdout_of(&rust_out));
}

#[test]
fn real_tree_parity_help_matches_bash() {
    let repo_root = real_repo_root();
    let bash_out = Command::new("bash")
        .arg(repo_root.join(".github/ci-test-scope.sh"))
        .arg("--help")
        .current_dir(&repo_root)
        .output()
        .unwrap();
    let rust_out = Command::new(env!("CARGO_BIN_EXE_ci-test-scope"))
        .arg("--help")
        .output()
        .unwrap();
    assert_eq!(stdout_of(&bash_out), stdout_of(&rust_out));
}
