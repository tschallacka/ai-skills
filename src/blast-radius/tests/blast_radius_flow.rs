// MODE: DEV
// Real-subprocess integration coverage for the compiled blast-radius binary,
// against a scratch git repo (WITH `git init` -- unlike goal 17's
// generate-portability, this crate is fundamentally git-dependent since its
// own repo_root discovery shells to `git rev-parse --show-toplevel`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_dir(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "blast-radius-flow-{tag}-{}-{:?}",
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

    /// A baseline coupling.tsv/PACKAGE-MANIFEST.tsv and a first commit --
    /// the shape every scenario below starts from before making its own
    /// working-tree change.
    fn seed_baseline(&self) {
        self.write(
            "coupling.tsv",
            "# header comment\nstale.txt\tfail\tstale.txt is generated from source.txt\tbash check.sh\nadvisory.txt\twarn\tadvisory.txt needs a human look\t\n",
        );
        self.write("check.sh", "#!/usr/bin/env bash\nexit 0\n");
        self.write(
            "planning/PACKAGE-MANIFEST.tsv",
            "planning/known.json\tskill\tnotes\n",
        );
        self.write("planning/known.json", "{}\n");
        self.commit_all("baseline");
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_blast-radius"))
            .args(args)
            .current_dir(&self.dir)
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

fn combined_of(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn a_clean_tree_reports_zero_failures() {
    let repo = Repo::new("clean-tree");
    repo.seed_baseline();
    // A trivial, uncoupled change so the change set is non-empty.
    repo.write("untracked.txt", "hello\n");
    let output = repo.run(&[]);
    assert!(output.status.success(), "{}", combined_of(&output));
    assert!(
        combined_of(&output).contains("0 failure(s)"),
        "{}",
        combined_of(&output)
    );
    repo.cleanup();
}

#[test]
fn a_coupling_row_whose_check_fails_is_reported_with_the_extracted_reason() {
    let repo = Repo::new("check-fails");
    repo.write(
        "coupling.tsv",
        "stale.txt\tfail\tstale.txt is generated from source.txt\tbash check.sh\n",
    );
    repo.write(
        "check.sh",
        "#!/usr/bin/env bash\nprintf 'check.sh: boom\\n' >&2\nexit 1\n",
    );
    repo.write("planning/PACKAGE-MANIFEST.tsv", "");
    repo.commit_all("baseline");
    repo.write("stale.txt", "changed\n");
    let output = repo.run(&["stale.txt"]);
    assert_eq!(output.status.code(), Some(1));
    let text = combined_of(&output);
    assert!(
        text.contains("FAIL: stale.txt is generated from source.txt"),
        "{text}"
    );
    assert!(text.contains("check.sh: boom"), "{text}");
    repo.cleanup();
}

#[test]
fn a_coupling_row_whose_check_succeeds_prints_the_exact_ok_line() {
    let repo = Repo::new("check-ok");
    repo.write(
        "coupling.tsv",
        "stale.txt\tfail\tstale.txt is generated from source.txt\tbash check.sh\n",
    );
    repo.write("check.sh", "#!/usr/bin/env bash\nexit 0\n");
    repo.write("planning/PACKAGE-MANIFEST.tsv", "");
    repo.commit_all("baseline");
    repo.write("stale.txt", "changed\n");
    let output = repo.run(&["stale.txt"]);
    assert!(output.status.success(), "{}", combined_of(&output));
    assert!(
        stdout_of(&output).contains("ok:   stale.txt is generated from source.txt"),
        "{}",
        stdout_of(&output)
    );
    repo.cleanup();
}

#[test]
fn a_coupling_row_with_no_check_reports_the_static_fail_or_warn_by_level() {
    let repo = Repo::new("no-check");
    repo.write(
        "coupling.tsv",
        "advisory.txt\twarn\tadvisory.txt needs a human look\t\nlocked.txt\tfail\tlocked.txt must not change\t\n",
    );
    repo.write("planning/PACKAGE-MANIFEST.tsv", "");
    repo.commit_all("baseline");
    repo.write("advisory.txt", "x\n");
    repo.write("locked.txt", "x\n");
    let output = repo.run(&["advisory.txt", "locked.txt"]);
    assert_eq!(output.status.code(), Some(1));
    let text = combined_of(&output);
    assert!(
        text.contains("WARN: advisory.txt: advisory.txt needs a human look"),
        "{text}"
    );
    assert!(
        text.contains("FAIL: locked.txt: locked.txt must not change"),
        "{text}"
    );
    repo.cleanup();
}

#[test]
fn a_new_planning_json_with_no_manifest_row_fails_with_will_not_ship() {
    let repo = Repo::new("unshipped-json");
    repo.seed_baseline();
    repo.write("planning/new-registry.json", "{}\n");
    let output = repo.run(&["planning/new-registry.json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        combined_of(&output).contains("will not ship"),
        "{}",
        combined_of(&output)
    );
    repo.cleanup();
}

#[test]
fn a_new_non_json_planning_file_with_no_manifest_row_warns_but_does_not_fail() {
    let repo = Repo::new("unregistered-test");
    repo.seed_baseline();
    repo.write("planning/tests/new-test.sh", "#!/usr/bin/env bash\n");
    let output = repo.run(&["planning/tests/new-test.sh"]);
    assert!(output.status.success(), "{}", combined_of(&output));
    let text = combined_of(&output);
    assert!(text.contains("no PACKAGE-MANIFEST row"), "{text}");
    assert!(text.contains("1 warning(s)"), "{text}");
    repo.cleanup();
}

#[test]
fn drift_against_an_explicit_base_reports_the_commit_count_and_remedy() {
    let repo = Repo::new("drift");
    repo.seed_baseline();
    let base = repo.git_output(&["rev-parse", "HEAD"]);
    repo.write("drifted.txt", "v1\n");
    repo.commit_all("touch drifted.txt");
    let output = repo.run(&["--base", &base, "drifted.txt"]);
    assert!(output.status.success(), "{}", combined_of(&output));
    let text = combined_of(&output);
    assert!(
        text.contains("drifted.txt changed in 1 commit(s) since"),
        "{text}"
    );
    assert!(text.contains("verify those commits survive"), "{text}");
    repo.cleanup();
}

#[test]
fn the_banner_shows_the_raw_base_even_when_the_origin_fallback_triggers() {
    // AR-75: the banner is printed before the drift pass's own origin/<base>
    // resolution ever runs, so it must always show the raw argument.
    let repo = Repo::new("banner-raw-base");
    repo.seed_baseline();
    let head = repo.git_output(&["rev-parse", "HEAD"]);
    repo.git(&["update-ref", "refs/remotes/origin/master", &head]);
    // Rename the local branch away so "master" no longer resolves locally,
    // forcing the origin/master fallback inside the drift pass.
    repo.git(&["branch", "-m", "master", "renamed-away"]);
    repo.write("untracked.txt", "x\n");
    let output = repo.run(&["--base", "master", "untracked.txt"]);
    assert!(output.status.success(), "{}", combined_of(&output));
    assert!(
        stdout_of(&output).contains("changed path(s), base master"),
        "banner must show the raw --base value, not origin/master:\n{}",
        stdout_of(&output)
    );
    repo.cleanup();
}

#[test]
fn help_prints_the_embedded_text_and_exits_zero() {
    let repo = Repo::new("help");
    repo.seed_baseline();
    let output = repo.run(&["--help"]);
    assert!(output.status.success(), "{}", combined_of(&output));
    assert!(
        stdout_of(&output).contains("integration-safety report"),
        "{}",
        stdout_of(&output)
    );
    repo.cleanup();
}

#[test]
fn an_unknown_option_exits_64() {
    let repo = Repo::new("unknown-option");
    repo.seed_baseline();
    let output = repo.run(&["-x"]);
    assert_eq!(output.status.code(), Some(64));
    assert!(
        combined_of(&output).contains("unknown option: -x"),
        "{}",
        combined_of(&output)
    );
    repo.cleanup();
}

#[test]
fn base_with_no_following_argument_exits_64() {
    let repo = Repo::new("base-no-arg");
    repo.seed_baseline();
    let output = repo.run(&["--base"]);
    assert_eq!(output.status.code(), Some(64));
    assert!(
        combined_of(&output).contains("--base needs a ref"),
        "{}",
        combined_of(&output)
    );
    repo.cleanup();
}

#[test]
fn a_non_git_work_tree_exits_69() {
    let dir = unique_dir("non-git");
    fs::create_dir_all(&dir).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_blast-radius"))
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(69));
    assert!(
        combined_of(&output).contains("not a git work tree"),
        "{}",
        combined_of(&output)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_git_work_tree_missing_coupling_tsv_exits_69() {
    let repo = Repo::new("missing-coupling");
    repo.write("placeholder.txt", "x\n");
    repo.commit_all("baseline, no coupling.tsv");
    let output = repo.run(&[]);
    assert_eq!(output.status.code(), Some(69));
    assert!(
        combined_of(&output).contains("coupling.tsv not found"),
        "{}",
        combined_of(&output)
    );
    repo.cleanup();
}

#[test]
fn an_explicit_empty_change_set_prints_no_changes_to_analyse() {
    let repo = Repo::new("empty-change-set");
    repo.seed_baseline();
    let output = repo.run(&[]);
    assert!(output.status.success(), "{}", combined_of(&output));
    assert!(
        stdout_of(&output).contains("no changes to analyse"),
        "{}",
        stdout_of(&output)
    );
    assert!(
        !combined_of(&output).contains("failure(s)"),
        "the no-changes early exit must not also print the failures/warnings summary line:\n{}",
        combined_of(&output)
    );
    repo.cleanup();
}

#[test]
fn an_explicit_blank_positional_argument_counts_as_no_changes() {
    // AR-71: a blank entry in the change set must not count as one changed
    // path, matching bash's own `grep -c .` non-blank counting.
    let repo = Repo::new("blank-positional");
    repo.seed_baseline();
    let output = repo.run(&[""]);
    assert!(output.status.success(), "{}", combined_of(&output));
    assert!(
        stdout_of(&output).contains("no changes to analyse"),
        "{}",
        stdout_of(&output)
    );
    repo.cleanup();
}

/// Read-only against the actual ai-skills repository, never mutating it:
/// runs both bash and the compiled binary against the SAME explicit,
/// deterministic path list and the real coupling.tsv/PACKAGE-MANIFEST.tsv/
/// git history, asserting byte-identical output.
#[test]
fn matches_the_real_bash_original_against_the_real_repository_tree() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("src/blast-radius is two levels below the repo root")
        .to_path_buf();
    let script = repo_root.join("blast-radius.sh");
    assert!(script.is_file(), "{} not found", script.display());

    // A fixed, deterministic path list so the test's own result does not
    // depend on this checkout's own uncommitted state.
    let args = ["--base", "master", "CODE-STYLE.md", "coupling.tsv"];

    let bash_output = Command::new("bash")
        .arg(&script)
        .args(args)
        .current_dir(&repo_root)
        .output()
        .unwrap();
    let binary_output = Command::new(env!("CARGO_BIN_EXE_blast-radius"))
        .args(args)
        .current_dir(&repo_root)
        .output()
        .unwrap();

    assert_eq!(
        bash_output.status.code(),
        binary_output.status.code(),
        "exit codes differ: bash={:?} stdout={} stderr={} | binary={:?} stdout={} stderr={}",
        bash_output.status.code(),
        stdout_of(&bash_output),
        String::from_utf8_lossy(&bash_output.stderr),
        binary_output.status.code(),
        stdout_of(&binary_output),
        String::from_utf8_lossy(&binary_output.stderr),
    );
    assert_eq!(
        combined_of(&bash_output),
        combined_of(&binary_output),
        "bash and the compiled binary produced different combined output"
    );
}
