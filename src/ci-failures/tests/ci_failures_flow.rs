// MODE: DEV
// Real-subprocess integration coverage for the compiled ci-failures binary,
// mirroring planning/tests/test-ci-failures-contract.sh's own established
// stub convention exactly (a fake gh/glab executable on PATH that logs its
// own argv and returns canned JSON, against a real, local git repository so
// remote/branch parsing is genuinely exercised, not stubbed away). This is
// the one place the glab path is exercised at all, matching the bash
// original's own documented "VERIFIED DIFFERENTLY" gap: there is no live
// GitLab remote to run against.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Harness {
    work: PathBuf,
    repo: PathBuf,
    stub_bin: PathBuf,
    logs_dir: PathBuf,
}

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}

impl Harness {
    fn new(tag: &str) -> Self {
        let mut work = std::env::temp_dir();
        work.push(format!(
            "ci-failures-flow-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work).unwrap();

        let repo = work.join("repo");
        fs::create_dir_all(&repo).unwrap();
        assert!(Command::new("git")
            .args(["init", "-q", "-b", "some-branch"])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success());

        let stub_bin = work.join("bin");
        fs::create_dir_all(&stub_bin).unwrap();
        let logs_dir = work.join("logs");
        fs::create_dir_all(&logs_dir).unwrap();

        write_executable(
            &stub_bin.join("gh"),
            r#"#!/usr/bin/env bash
argv="$*"
printf 'gh %s\n' "$argv" >>"$STUB_LOG"
case "$argv" in
    "repo view"*)
        [ "${STUB_GH_AVAILABLE:-1}" = 1 ] && exit 0 || exit 1
        ;;
    *"pr view "*"--json headRefName"*)
        printf '{"headRefName":"%s"}\n' "$STUB_HEAD_BRANCH"
        ;;
    *"run list "*"--json databaseId,name"*)
        printf '%s\n' "$STUB_RUN_LIST_JSON"
        ;;
    *"run view "*"--json status"*)
        printf '{"status":"%s"}\n' "$STUB_RUN_STATUS"
        ;;
    *"run view "*"--json conclusion"*)
        printf '{"conclusion":"%s"}\n' "$STUB_RUN_CONCLUSION"
        ;;
    *"run view "*"--json jobs"*)
        printf '{"jobs":%s}\n' "$STUB_JOBS_JSON"
        ;;
    *"api --allow-escape-sequences "*"/logs")
        job_id="${argv##*/actions/jobs/}"
        job_id="${job_id%/logs}"
        cat "$STUB_LOGS_DIR/$job_id.log" 2>/dev/null || true
        ;;
    *) exit 1 ;;
esac
"#,
        );
        write_executable(
            &stub_bin.join("glab"),
            r#"#!/usr/bin/env bash
argv="$*"
printf 'glab %s\n' "$argv" >>"$STUB_LOG"
case "$argv" in
    "repo view"*)
        [ "${STUB_GLAB_AVAILABLE:-1}" = 1 ] && exit 0 || exit 1
        ;;
    *"/merge_requests/"*)
        printf '{"source_branch":"%s"}\n' "$STUB_MR_BRANCH"
        ;;
    *"/pipelines?ref="*)
        printf '%s\n' "$STUB_PIPELINE_LIST_JSON"
        ;;
    *"/pipelines/"*"/jobs?"*)
        printf '%s\n' "$STUB_JOBS_JSON"
        ;;
    *"/jobs/"*"/trace")
        job_id="${argv##*/jobs/}"
        job_id="${job_id%/trace}"
        cat "$STUB_LOGS_DIR/$job_id.log" 2>/dev/null || true
        ;;
    *"/pipelines/"*)
        printf '{"status":"%s"}\n' "$STUB_PIPELINE_STATUS"
        ;;
    *) exit 1 ;;
esac
"#,
        );

        Harness {
            work,
            repo,
            stub_bin,
            logs_dir,
        }
    }

    fn set_remote(&self, url: &str) {
        let _ = Command::new("git")
            .args(["remote", "remove", "origin"])
            .current_dir(&self.repo)
            .stderr(std::process::Stdio::null())
            .status();
        assert!(Command::new("git")
            .args(["remote", "add", "origin", url])
            .current_dir(&self.repo)
            .status()
            .unwrap()
            .success());
    }

    fn run(&self, args: &[&str]) -> (i32, String, String, String) {
        let binary = env!("CARGO_BIN_EXE_ci-failures");
        let call_log = self.work.join("call.log");
        let _ = fs::remove_file(&call_log);
        let path_env = format!(
            "{}:{}",
            self.stub_bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let output = Command::new(binary)
            .args(args)
            .current_dir(&self.repo)
            .env("PATH", path_env)
            .env("STUB_LOG", &call_log)
            .env("STUB_LOGS_DIR", &self.logs_dir)
            .env_remove("CI_FAILURES_REPO")
            .env_remove("CI_FAILURES_FORGE")
            .env("STUB_GH_AVAILABLE", "1")
            .env("STUB_GLAB_AVAILABLE", "1")
            .env("STUB_HEAD_BRANCH", "some-branch")
            .env("STUB_RUN_STATUS", "completed")
            .env("STUB_RUN_CONCLUSION", "success")
            .env(
                "STUB_RUN_LIST_JSON",
                r#"[{"databaseId":123456789,"name":"build"}]"#,
            )
            .env("STUB_JOBS_JSON", "[]")
            .env("STUB_MR_BRANCH", "some-branch")
            .env("STUB_PIPELINE_STATUS", "success")
            .env("STUB_PIPELINE_LIST_JSON", r#"[{"id":987654321}]"#)
            .output()
            .expect("run the compiled ci-failures binary");
        let calls = fs::read_to_string(&call_log).unwrap_or_default();
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
            calls,
        )
    }

    fn write_log(&self, job_id: &str, content: &str) {
        fs::write(self.logs_dir.join(format!("{job_id}.log")), content).unwrap();
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.work);
    }
}

#[test]
fn a_github_remote_selects_gh() {
    let h = Harness::new("gh-select");
    h.set_remote("https://github.com/tschallacka/ai-skills.git");
    let (_, out, _, _) = h.run(&[]);
    assert!(out.starts_with("forge: gh"), "{out}");
}

#[test]
fn a_gitlab_remote_selects_glab() {
    let h = Harness::new("glab-select");
    h.set_remote("git@gitlab.com:tschallacka/ai-skills.git");
    let (_, out, _, _) = h.run(&[]);
    assert!(out.starts_with("forge: glab"), "{out}");
}

#[test]
fn a_ten_digit_target_is_a_run_id_with_no_pr_lookup() {
    let h = Harness::new("gh-run-id");
    h.set_remote("https://github.com/tschallacka/ai-skills.git");
    let (_, _, _, calls) = h.run(&["3389420559"]);
    assert!(!calls.contains("pr view"), "{calls}");
    assert!(calls.contains("run view 3389420559"), "{calls}");
}

#[test]
fn a_short_number_resolves_as_a_pr() {
    let h = Harness::new("gh-pr-number");
    h.set_remote("https://github.com/tschallacka/ai-skills.git");
    let (_, _, _, calls) = h.run(&["47"]);
    assert!(calls.contains("pr view 47 "), "{calls}");
}

#[test]
fn empty_target_uses_the_current_branch_on_gh() {
    let h = Harness::new("gh-current-branch");
    h.set_remote("https://github.com/tschallacka/ai-skills.git");
    let (_, _, _, calls) = h.run(&[]);
    assert!(
        calls.contains("run list") && calls.contains("--branch some-branch"),
        "{calls}"
    );
}

#[test]
fn default_gh_listing_shows_only_failing_jobs() {
    let h = Harness::new("gh-default-filter");
    h.set_remote("https://github.com/tschallacka/ai-skills.git");
    let binary = env!("CARGO_BIN_EXE_ci-failures");
    let call_log = h.work.join("call.log");
    let path_env = format!(
        "{}:{}",
        h.stub_bin.display(),
        std::env::var("PATH").unwrap()
    );
    let output = Command::new(binary)
        .args(["pr/47"])
        .current_dir(&h.repo)
        .env("PATH", path_env)
        .env("STUB_LOG", &call_log)
        .env("STUB_LOGS_DIR", &h.logs_dir)
        .env("STUB_GH_AVAILABLE", "1")
        .env("STUB_GLAB_AVAILABLE", "1")
        .env("STUB_HEAD_BRANCH", "some-branch")
        .env("STUB_RUN_STATUS", "completed")
        .env("STUB_RUN_CONCLUSION", "success")
        .env("STUB_RUN_LIST_JSON", r#"[{"databaseId":123456789,"name":"build"}]"#)
        .env(
            "STUB_JOBS_JSON",
            r#"[{"databaseId":1,"conclusion":"success","name":"a"},{"databaseId":2,"conclusion":"failure","name":"b"}]"#,
        )
        .env("STUB_MR_BRANCH", "some-branch")
        .env("STUB_PIPELINE_STATUS", "success")
        .env("STUB_PIPELINE_LIST_JSON", r#"[{"id":987654321}]"#)
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(!out.contains("== a "), "{out}");
    assert!(out.contains("== b "), "{out}");
}

#[test]
fn scp_like_glab_remote_encodes_the_nested_project_path() {
    let h = Harness::new("glab-scp-encode");
    h.set_remote("git@gitlab.com:tschallacka/some-group/ai-skills.git");
    let (_, _, _, calls) = h.run(&["pr/47"]);
    assert!(
        calls.contains("projects/tschallacka%2Fsome-group%2Fai-skills/merge_requests/47"),
        "{calls}"
    );
}

#[test]
fn a_large_glab_target_is_a_pipeline_id_with_no_mr_lookup() {
    let h = Harness::new("glab-pipeline-id");
    h.set_remote("https://gitlab.com/tschallacka/ai-skills.git");
    let (_, _, _, calls) = h.run(&["987654321012"]);
    assert!(!calls.contains("merge_requests"), "{calls}");
    assert!(calls.contains("pipelines/987654321012"), "{calls}");
}

#[test]
fn ci_failures_repo_overrides_the_default_slug_on_gh() {
    let h = Harness::new("gh-repo-override");
    h.set_remote("https://github.com/tschallacka/ai-skills.git");
    let binary = env!("CARGO_BIN_EXE_ci-failures");
    let call_log = h.work.join("call.log");
    let path_env = format!(
        "{}:{}",
        h.stub_bin.display(),
        std::env::var("PATH").unwrap()
    );
    Command::new(binary)
        .args(["pr/47"])
        .current_dir(&h.repo)
        .env("PATH", path_env)
        .env("STUB_LOG", &call_log)
        .env("STUB_LOGS_DIR", &h.logs_dir)
        .env("CI_FAILURES_REPO", "other/repo")
        .env("STUB_GH_AVAILABLE", "1")
        .env("STUB_GLAB_AVAILABLE", "1")
        .env("STUB_HEAD_BRANCH", "some-branch")
        .env("STUB_RUN_STATUS", "completed")
        .env("STUB_RUN_CONCLUSION", "success")
        .env(
            "STUB_RUN_LIST_JSON",
            r#"[{"databaseId":123456789,"name":"build"}]"#,
        )
        .env("STUB_JOBS_JSON", "[]")
        .env("STUB_MR_BRANCH", "some-branch")
        .env("STUB_PIPELINE_STATUS", "success")
        .env("STUB_PIPELINE_LIST_JSON", r#"[{"id":987654321}]"#)
        .output()
        .unwrap();
    let calls = fs::read_to_string(&call_log).unwrap_or_default();
    assert!(calls.contains("--repo other/repo"), "{calls}");
}

#[test]
fn the_extractor_catches_a_panic_and_a_fail_row_in_an_untimestamped_log() {
    let h = Harness::new("extract-untimestamped");
    h.set_remote("https://gitlab.com/tschallacka/ai-skills.git");
    h.write_log(
        "1",
        "running the suite\nthread 'main' panicked at src/lib.rs:42:5:\ncalled `Option::unwrap()` on a `None` value\nnote: run with RUST_BACKTRACE=1 for a backtrace\n  test-something    FAIL (exit 1)\n    reason: assertion failed\ntest result: FAILED. 3 passed; 1 failed\n",
    );
    let binary = env!("CARGO_BIN_EXE_ci-failures");
    let call_log = h.work.join("call.log");
    let path_env = format!(
        "{}:{}",
        h.stub_bin.display(),
        std::env::var("PATH").unwrap()
    );
    let output = Command::new(binary)
        .args(["pr/47"])
        .current_dir(&h.repo)
        .env("PATH", path_env)
        .env("STUB_LOG", &call_log)
        .env("STUB_LOGS_DIR", &h.logs_dir)
        .env("STUB_GH_AVAILABLE", "1")
        .env("STUB_GLAB_AVAILABLE", "1")
        .env("STUB_HEAD_BRANCH", "some-branch")
        .env("STUB_RUN_STATUS", "completed")
        .env("STUB_RUN_CONCLUSION", "success")
        .env(
            "STUB_RUN_LIST_JSON",
            r#"[{"databaseId":123456789,"name":"build"}]"#,
        )
        .env(
            "STUB_JOBS_JSON",
            r#"[{"id":1,"status":"failed","name":"tests"}]"#,
        )
        .env("STUB_MR_BRANCH", "some-branch")
        .env("STUB_PIPELINE_STATUS", "success")
        .env("STUB_PIPELINE_LIST_JSON", r#"[{"id":987654321}]"#)
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(out.contains("panicked at src/lib.rs:42:5:"), "{out}");
    assert!(out.contains("FAIL (exit 1)"), "{out}");
    assert!(out.contains("reason: assertion failed"), "{out}");
    assert!(out.contains("test result: FAILED"), "{out}");
}

#[test]
fn raw_writes_the_de_escaped_log() {
    let h = Harness::new("raw-write");
    h.set_remote("https://gitlab.com/tschallacka/ai-skills.git");
    h.write_log("1", "plain\n\x1b[31mred\x1b[0m\r\n");
    let raw_dir = h.work.join("raw");
    let binary = env!("CARGO_BIN_EXE_ci-failures");
    let call_log = h.work.join("call.log");
    let path_env = format!(
        "{}:{}",
        h.stub_bin.display(),
        std::env::var("PATH").unwrap()
    );
    let output = Command::new(binary)
        .args(["pr/47", "--raw", raw_dir.to_str().unwrap()])
        .current_dir(&h.repo)
        .env("PATH", path_env)
        .env("STUB_LOG", &call_log)
        .env("STUB_LOGS_DIR", &h.logs_dir)
        .env("STUB_GH_AVAILABLE", "1")
        .env("STUB_GLAB_AVAILABLE", "1")
        .env("STUB_HEAD_BRANCH", "some-branch")
        .env("STUB_RUN_STATUS", "completed")
        .env("STUB_RUN_CONCLUSION", "success")
        .env(
            "STUB_RUN_LIST_JSON",
            r#"[{"databaseId":123456789,"name":"build"}]"#,
        )
        .env(
            "STUB_JOBS_JSON",
            r#"[{"id":1,"status":"failed","name":"tests"}]"#,
        )
        .env("STUB_MR_BRANCH", "some-branch")
        .env("STUB_PIPELINE_STATUS", "success")
        .env("STUB_PIPELINE_LIST_JSON", r#"[{"id":987654321}]"#)
        .output()
        .unwrap();
    assert!(output.status.success());
    let written = fs::read_to_string(raw_dir.join("1.log")).expect("raw log written");
    assert!(!written.contains('\x1b'), "{written}");
    assert!(written.contains("red"), "{written}");
}

#[test]
fn raw_with_no_value_exits_64() {
    let h = Harness::new("raw-missing-value");
    h.set_remote("https://github.com/tschallacka/ai-skills.git");
    let (code, _, _, _) = h.run(&["pr/47", "--raw"]);
    assert_eq!(code, 64);
}

#[test]
fn help_prints_the_usage_block_and_exits_0() {
    let h = Harness::new("help");
    let (code, out, _, _) = h.run(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("Usage:"), "{out}");
    assert!(out.contains("FORGE DETECTION."), "{out}");
}
