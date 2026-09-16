// MODE: DEV
// Real-subprocess integration coverage for the compiled generate-portability
// binary, mirroring goal 16's own Repo-fixture harness shape -- but WITHOUT
// git init, since this script has no git dependency at all.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_dir(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "generate-portability-flow-{tag}-{}-{:?}",
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

    fn write_rules(&self, rules_json: &str) {
        fs::write(self.dir.join("portability-rules.json"), rules_json).unwrap();
    }

    fn write_script(&self, rel_path: &str, content: &str) {
        let path = self.dir.join(rel_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_generate-portability"))
            .args(args)
            .env("PLANNING_SKILL_ROOT", &self.dir)
            .current_dir(&self.dir)
            .output()
            .unwrap()
    }

    fn run_with_output_override(&self, output_path: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_generate-portability"))
            .env("PLANNING_SKILL_ROOT", &self.dir)
            .env("PORTABILITY_OUTPUT", output_path)
            .current_dir(&self.dir)
            .output()
            .unwrap()
    }
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

const SIMPLE_RULES: &str = r#"{
    "target": {
        "shell": "bash 3.2",
        "userland": "GNU or BSD",
        "locale": "C",
        "verified": {"bash-3.2": "yes", "bsd-userland": "yes"}
    },
    "rules": [
        {"id": "assoc-array", "construct": "declare -A", "detect": "declare -A", "breaks": "bash 3.2", "symptom": "invalid option", "replacement": "plan_map_set"},
        {"id": "no-detect", "construct": "a review-only gotcha", "detect": null, "breaks": "any host", "symptom": "n/a", "replacement": "read the code"}
    ]
}"#;

fn real_repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn a_plain_run_writes_the_expected_catalogue_sections() {
    let repo = Repo::new("plain-run");
    repo.write_rules(SIMPLE_RULES);
    repo.write_script(
        "src/marked.sh",
        "#!/usr/bin/env bash\nfoo  # PORTABILITY(assoc-array): why here\n",
    );

    let output = repo.run(&[]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    let content = fs::read_to_string(repo.dir.join("PORTABILITY.md")).unwrap();
    assert!(content.contains("## The gotchas"));
    assert!(content.contains("### `declare -A`"));
    assert!(content.contains("why here"));
    assert!(content.contains("### `a review-only gotcha`"));
    assert!(content.contains("Detection:** none possible"));
}

#[test]
fn check_reports_success_for_two_fresh_builds() {
    let repo = Repo::new("check");
    repo.write_rules(SIMPLE_RULES);
    let output = repo.run(&["--check"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(stdout_of(&output).contains("deterministic"));
}

#[test]
fn help_prints_the_embedded_text_and_exits_zero() {
    let repo = Repo::new("help");
    repo.write_rules(SIMPLE_RULES);
    let output = repo.run(&["--help"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("generate-portability"));
    assert!(stdout_of(&output).contains("Usage:"));
}

#[test]
fn an_unrecognized_argument_is_silently_ignored() {
    let repo = Repo::new("unknown-arg");
    repo.write_rules(SIMPLE_RULES);
    let output = repo.run(&["--bogus"]);
    assert!(
        output.status.success(),
        "expected a normal run, got: {}",
        stderr_of(&output)
    );
    assert!(repo.dir.join("PORTABILITY.md").is_file());
}

#[test]
fn portability_output_override_is_honored() {
    let repo = Repo::new("output-override");
    repo.write_rules(SIMPLE_RULES);
    let scratch_output = repo.dir.join("elsewhere.md");
    let output = repo.run_with_output_override(&scratch_output);
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(scratch_output.is_file());
    assert!(!repo.dir.join("PORTABILITY.md").is_file());
}

#[test]
fn missing_rules_file_is_a_clean_failure_not_a_panic() {
    let repo = Repo::new("missing-rules");
    let output = repo.run(&[]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(66));
}

/// Real-tree parity: read-only against the actual ai-skills repository,
/// never mutating it -- both bash and the compiled binary write to scratch
/// PORTABILITY_OUTPUT paths, diffed with the generated-timestamp line
/// stripped from both.
#[test]
fn matches_the_real_bash_original_against_the_real_repository_tree() {
    let repo_root = real_repo_root();
    let bash_script = repo_root.join("generate-portability.sh");
    assert!(bash_script.is_file());

    let scratch = unique_dir("real-tree-parity");
    fs::create_dir_all(&scratch).unwrap();
    let bash_out = scratch.join("bash.md");
    let rust_out = scratch.join("rust.md");

    let bash_status = Command::new("bash")
        .arg(&bash_script)
        .env("PORTABILITY_OUTPUT", &bash_out)
        .current_dir(&repo_root)
        .status()
        .unwrap();
    assert!(bash_status.success());

    let rust_status = Command::new(env!("CARGO_BIN_EXE_generate-portability"))
        .env("PLANNING_SKILL_ROOT", &repo_root)
        .env("PORTABILITY_OUTPUT", &rust_out)
        .current_dir(&repo_root)
        .status()
        .unwrap();
    assert!(rust_status.success());

    let bash_content = fs::read_to_string(&bash_out).unwrap();
    let rust_content = fs::read_to_string(&rust_out).unwrap();
    let strip = |s: &str| -> String {
        s.lines()
            .filter(|l| !l.starts_with("<!-- generated: "))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(strip(&bash_content), strip(&rust_content));
}
