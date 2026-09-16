// MODE: DEV
// PACKAGE: PROD

//! report_one/run_one/run_cargo_one: how each test or crate is launched and
//! its outcome classified and counted, matching run-tests.sh's own
//! PASS/SKIP/FAIL/TIMEOUT/UNCONFIGURED reporting exactly.

use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

pub const CONTEXT_GATED: [&str; 2] = [
    "planning/tests/test-plan-context.sh",
    "planning/tests/test-plan-context-deferred-boundary.sh",
];

pub fn is_context_gated(repo_relative: &str) -> bool {
    CONTEXT_GATED.contains(&repo_relative)
}

#[derive(Default)]
pub struct Counts {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub unconfigured: u32,
    pub failed_names: Vec<String>,
    pub skipped_names: Vec<String>,
    pub unconfigured_names: Vec<String>,
}

pub struct RunConfig<'a> {
    pub repo_root: &'a Path,
    pub timeout_cmd: Option<&'a str>,
    pub test_timeout_seconds: u64,
    pub wrapper: Option<&'a str>,
    pub bash: &'a str,
    pub verbose: bool,
    pub extra_path: Option<&'a str>,
    pub context_cache_set: bool,
    pub refuse_unconfigured_cargo: bool,
    /// Every child process inherits these three explicitly (rather than
    /// mutating this process's own environment, which std::env::set_var
    /// cannot safely do on this toolchain), matching bash's own
    /// `export TMPDIR=...`/`export PLANNING_AGENT_TMPDIR=...` for the
    /// duration of the run.
    pub tmpdir: &'a Path,
    pub planning_agent_tmpdir: &'a Path,
    pub test_run_id: &'a str,
}

/// Matches bash's own `sed 's#^.*/tests/##; s#\.sh$##'` exactly -- run on
/// the test's own ABSOLUTE path, not a repo-relative one: the leading "/"
/// bash's pattern relies on to find "/tests/" comes from repo_root's own
/// path, not from the suite name itself (a suite literally named "tests"
/// has no leading slash of its own before stripping repo_root).
fn label_for(test_path: &Path) -> String {
    let full = test_path.to_string_lossy().into_owned();
    let after_tests = full
        .rsplit_once("/tests/")
        .map(|(_, rest)| rest)
        .unwrap_or(&full);
    after_tests
        .strip_suffix(".sh")
        .unwrap_or(after_tests)
        .to_string()
}

fn apply_child_env(command: &mut Command, config: &RunConfig) {
    if let Some(path) = config.extra_path {
        command.env("PATH", path);
    }
    command
        .env("TMPDIR", config.tmpdir)
        .env("PLANNING_AGENT_TMPDIR", config.planning_agent_tmpdir)
        .env("AI_SKILLS_TEST_RUN_ID", config.test_run_id);
}

fn report_one(label: &str, code: Option<i32>, output: &str, verbose: bool, counts: &mut Counts) {
    match code {
        Some(0) => {
            // t_skip exits 0, same as a real pass (B268): only the trailing
            // "<test>: SKIP" line in the test's own output tells them apart.
            if output.lines().any(|line| line.ends_with(": SKIP")) {
                counts.skipped += 1;
                counts.skipped_names.push(label.to_string());
                println!("  {label:<52} SKIP");
                print_indented(output);
            } else {
                counts.passed += 1;
                println!("  {label:<52} PASS");
                if verbose {
                    print_indented(output);
                }
            }
        }
        other => {
            counts.failed += 1;
            counts.failed_names.push(label.to_string());
            match other {
                Some(124) => println!("  {label:<52} TIMEOUT"),
                Some(code) => println!("  {label:<52} FAIL (exit {code})"),
                None => println!("  {label:<52} FAIL (terminated by signal)"),
            }
            print_indented(output);
        }
    }
}

fn print_indented(output: &str) {
    for line in output.lines() {
        println!("      {line}");
    }
}

pub fn run_one(config: &RunConfig, test_path: &Path, counts: &mut Counts) {
    let label = label_for(test_path);
    let relative = test_path
        .strip_prefix(config.repo_root)
        .unwrap_or(test_path)
        .to_string_lossy()
        .into_owned();

    if is_context_gated(&relative) && !config.context_cache_set {
        counts.unconfigured += 1;
        counts.unconfigured_names.push(label.clone());
        println!("  {label:<52} UNCONFIGURED (PLANNING_CONTEXT_CACHE)");
        return;
    }

    counts.total += 1;
    let (mem, cpu) = if relative.starts_with("benchmark/") {
        ("6G", "400")
    } else {
        ("2G", "400")
    };

    let trailing: Vec<OsString> = vec![config.bash.into(), test_path.as_os_str().to_os_string()];
    let mut command = build_command(config, mem, cpu, trailing);
    apply_child_env(&mut command, config);
    let (code, output) = run_captured(&mut command);
    report_one(&label, code, &output, config.verbose, counts);
}

pub fn run_cargo_one(config: &RunConfig, crate_dir: &str, counts: &mut Counts) {
    let label = format!(
        "cargo-{}",
        crate_dir.rsplit('/').next().unwrap_or(crate_dir)
    );
    if !which("cargo") {
        if config.refuse_unconfigured_cargo {
            counts.failed += 1;
            counts.failed_names.push(label.clone());
            println!("  {label:<52} FAIL (cargo unavailable; REFUSE_UNCONFIGURED_CARGO=1)");
        } else {
            counts.unconfigured += 1;
            counts.unconfigured_names.push(label.clone());
            println!("  {label:<52} UNCONFIGURED (cargo)");
        }
        return;
    }
    counts.total += 1;
    let manifest = config.repo_root.join(crate_dir).join("Cargo.toml");
    let trailing: Vec<OsString> = vec![
        "cargo".into(),
        "test".into(),
        "--manifest-path".into(),
        manifest.into_os_string(),
    ];
    let mut command = build_command(config, "2G", "400", trailing);
    apply_child_env(&mut command, config);
    let (code, output) = run_captured(&mut command);
    report_one(&label, code, &output, config.verbose, counts);
}

fn which(program: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(program).is_file())
}

/// Builds the full argv exactly as the original does: optional timeout_cmd,
/// optional wrapper (mem cpu --), then `trailing` (the bash-test or
/// cargo-test invocation) -- each optional layer present only when
/// configured, matching bash's own optional-array-element construction (an
/// absent wrapper must not appear as an empty argv element).
fn build_command(config: &RunConfig, mem: &str, cpu: &str, trailing: Vec<OsString>) -> Command {
    let mut argv: Vec<OsString> = Vec::new();
    if let Some(timeout_cmd) = config.timeout_cmd {
        argv.push(timeout_cmd.into());
        argv.push(config.test_timeout_seconds.to_string().into());
    }
    if let Some(wrapper) = config.wrapper {
        argv.push(wrapper.into());
        argv.push(mem.into());
        argv.push(cpu.into());
        argv.push("--".into());
    }
    argv.extend(trailing);
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);
    command
}

fn run_captured(command: &mut Command) -> (Option<i32>, String) {
    let output = command.stdin(Stdio::null()).output();
    match output {
        Ok(out) => {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            (out.status.code(), combined)
        }
        Err(error) => (Some(127), format!("could not run: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_strips_suite_tests_prefix_and_sh_suffix() {
        let root = Path::new("/repo");
        let path = root.join("planning/tests/test-foo.sh");
        assert_eq!(label_for(&path), "test-foo");
    }

    #[test]
    fn label_strips_correctly_for_a_suite_directly_named_tests() {
        // A suite literally named "tests" (a direct child of repo_root) has
        // no leading slash of its own before "tests/" once repo_root is
        // stripped -- the fix for a real bug this crate shipped with,
        // caught by direct comparison against the real bash original.
        let root = Path::new("/repo");
        let path = root.join("tests/test-foo.sh");
        assert_eq!(label_for(&path), "test-foo");
    }

    #[test]
    fn context_gated_matches_only_the_two_named_entries() {
        assert!(is_context_gated("planning/tests/test-plan-context.sh"));
        assert!(is_context_gated(
            "planning/tests/test-plan-context-deferred-boundary.sh"
        ));
        assert!(!is_context_gated(
            "planning/tests/test-plan-context-extra.sh"
        ));
    }

    #[test]
    fn skip_detection_requires_the_exact_trailing_marker() {
        let mut counts = Counts::default();
        report_one("t", Some(0), "line one\nt: SKIP", false, &mut counts);
        assert_eq!(counts.skipped, 1);
        assert_eq!(counts.passed, 0);

        let mut counts2 = Counts::default();
        report_one("t", Some(0), "t: SKIPPED\n", false, &mut counts2);
        assert_eq!(counts2.skipped, 0);
        assert_eq!(counts2.passed, 1);
    }
}
