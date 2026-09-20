// MODE: DEV
// PACKAGE: PROD
mod bootstrap;
mod discovery;
mod lock;
mod platform;
mod runner;
mod scratch;

use runner::{run_cargo_one, run_one, Counts, RunConfig};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// The bash original derives its own name from `${0##*/}`, which in every
// real invocation is "run-tests.sh" -- the literal file name, not the
// compiled binary's own bare name (matching the fix goal 14's own
// pre-push-check crate needed, AR-52).
const PROGRAM: &str = "run-tests.sh";

fn discover_repo_root() -> PathBuf {
    Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

struct Args {
    verbose: bool,
    list_only: bool,
    select_file: Option<PathBuf>,
    shard: Option<(usize, usize)>,
}

enum ParseOutcome {
    Run(Args),
    Exit(i32),
}

fn parse_args(argv: &[String]) -> ParseOutcome {
    let mut verbose = false;
    let mut list_only = false;
    let mut select_file = None;
    let mut shard = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--verbose" => {
                verbose = true;
                i += 1;
            }
            "--list-only" => {
                list_only = true;
                i += 1;
            }
            "--select-file" => {
                let Some(path) = argv.get(i + 1) else {
                    eprintln!("{PROGRAM}: --select-file needs a path");
                    return ParseOutcome::Exit(64);
                };
                select_file = Some(PathBuf::from(path));
                i += 2;
            }
            "--shard" => {
                let Some(spec) = argv.get(i + 1) else {
                    eprintln!("{PROGRAM}: --shard needs I/N");
                    return ParseOutcome::Exit(64);
                };
                match discovery::parse_shard(PROGRAM, spec) {
                    Ok(pair) => shard = Some(pair),
                    Err(message) => {
                        eprintln!("{message}");
                        return ParseOutcome::Exit(64);
                    }
                }
                i += 2;
            }
            other => {
                eprintln!("{PROGRAM}: unknown argument: {other}");
                return ParseOutcome::Exit(64);
            }
        }
    }
    ParseOutcome::Run(Args {
        verbose,
        list_only,
        select_file,
        shard,
    })
}

fn resolve_wrapper(repo_root: &Path) -> Option<PathBuf> {
    let default_wrapper = repo_root.join("resource-limited-testing/scripts/limited-run.sh");
    match env::var("AI_SKILLS_RESOURCE_LIMIT").as_deref() {
        Ok("0") => None,
        Ok("1") => Some(default_wrapper),
        _ => {
            // No resource cap on Windows either: the wrapper caps memory with
            // a systemd scope or memlimit, neither of which exists there.
            if env::var_os("GITHUB_ACTIONS").is_some() || cfg!(windows) {
                None
            } else {
                Some(default_wrapper)
            }
        }
    }
}

fn unix_time_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn run() -> i32 {
    let argv: Vec<String> = env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        ParseOutcome::Exit(code) => return code,
        ParseOutcome::Run(args) => args,
    };

    let repo_root = discover_repo_root();
    let wrapper = resolve_wrapper(&repo_root);

    let test_timeout_seconds: u64 = env::var("AI_SKILLS_TEST_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600);
    // GNU timeout(1) bounds a test where there is one. Windows has a
    // `timeout.exe` of its own that merely waits N seconds, so it is never
    // used there; and a host with none (a stock macOS) gets the bound
    // enforced by this process instead of no bound at all.
    let (timeout_cmd, native_timeout) = if !cfg!(windows) && platform::which("timeout") {
        (Some("timeout"), None)
    } else {
        (None, Some(Duration::from_secs(test_timeout_seconds)))
    };

    // ---- discovery: needs no lock, no bootstrap --------------------------
    let mut work_items = discovery::build_work_items(&repo_root);
    if let Some(select_file) = &args.select_file {
        match discovery::apply_select_file(work_items, select_file) {
            Ok(items) => work_items = items,
            Err(_) => {
                eprintln!(
                    "{PROGRAM}: cannot read --select-file {}",
                    select_file.display()
                );
                return 64;
            }
        }
    }
    if let Some((index, total)) = args.shard {
        work_items = discovery::apply_shard(work_items, index, total);
    }

    if args.list_only {
        for item in &work_items {
            println!("{item}");
        }
        return 0;
    }

    let (tests, crates) = discovery::split_tests_and_crates(&repo_root, &work_items);

    // ---- one run at a time, machine-wide ----------------------------------
    let allow_concurrent = env::var("AI_SKILLS_ALLOW_CONCURRENT").as_deref() == Ok("1");
    let lock_path = lock::lock_path();
    let (_lock, acquire_result) = lock::Lock::acquire(&lock_path, &repo_root, allow_concurrent);
    match acquire_result {
        lock::AcquireResult::Bypassed => {
            eprintln!("run-tests: AI_SKILLS_ALLOW_CONCURRENT=1; the single-run lock is bypassed");
        }
        lock::AcquireResult::Acquired => {}
        lock::AcquireResult::RefusedLiveHolder {
            pid,
            started,
            in_dir,
            command,
        } => {
            eprintln!("run-tests: another suite run is already going (pid {pid})");
            eprintln!("  started: {started}");
            eprintln!("  in:      {in_dir}");
            eprintln!("  command: {command}");
            eprintln!("  Two runs on one machine collide over the cargo target dir, the");
            eprintln!("  chat beacon port and the /tmp test roots. Wait for it, or set");
            eprintln!("  AI_SKILLS_ALLOW_CONCURRENT=1 to run anyway and accept the noise.");
            return 75;
        }
        lock::AcquireResult::RefusedLostRace { pid } => {
            eprintln!("run-tests: reusing a stale lock left by pid {pid}");
            eprintln!("run-tests: another run took the lock first (pid {pid}); not starting");
            return 75;
        }
    }

    // ---- per-run scratch root ----------------------------------------------
    let tmp_base = env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(platform::system_tmp);
    let pid = std::process::id();
    let unix_time = unix_time_now();
    let scratch = match scratch::ScratchRoot::create(&tmp_base, &repo_root, pid, unix_time) {
        Ok(s) => s,
        Err(error) => {
            eprintln!("{PROGRAM}: could not create a scratch root: {error}");
            return 1;
        }
    };

    // Install the signal-handling cleanup path: process::exit() from the
    // signal thread bypasses ordinary Drop glue on the main thread's own
    // stack, so this closure independently repeats the same three cleanup
    // actions Lock/ScratchRoot's own Drop impls perform on a normal return.
    {
        let cleanup_lock_path = lock_path.clone();
        let cleanup_scratch_path = scratch.path.clone();
        let cleanup_run_id = scratch.run_id.clone();
        let cleanup_repo_root = repo_root.clone();
        scratch::install_signal_cleanup(move || {
            lock::release_lock_by_path(&cleanup_lock_path, pid);
            scratch::cleanup_marked_test_roots(&cleanup_scratch_path, &cleanup_run_id);
            let _ = std::fs::remove_dir_all(&cleanup_scratch_path);
            scratch::remove_benchmark_staging(&cleanup_repo_root);
        });
    }

    if let Err(message) = bootstrap::refuse_if_dev_env_dirty(&repo_root) {
        eprintln!("{message}");
        return 70;
    }

    let extra_dir = match bootstrap::bootstrap_generated(&repo_root) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("{message}");
            return 69;
        }
    };
    let extra_path = bootstrap::effective_path(extra_dir.as_deref());

    let bash = platform::bash_program(&platform::bash_from_env())
        .to_string_lossy()
        .into_owned();
    let context_cache_set = env::var("PLANNING_CONTEXT_CACHE")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let refuse_unconfigured_cargo = env::var("REFUSE_UNCONFIGURED_CARGO").as_deref() == Ok("1");
    let planning_agent_tmpdir = scratch.path.join("planning-agent");

    let config = RunConfig {
        repo_root: &repo_root,
        timeout_cmd,
        test_timeout_seconds,
        native_timeout,
        wrapper: wrapper.as_deref().and_then(|p| p.to_str()),
        bash: &bash,
        verbose: args.verbose,
        extra_path: extra_path.as_deref(),
        context_cache_set,
        refuse_unconfigured_cargo,
        tmpdir: &scratch.path,
        planning_agent_tmpdir: &planning_agent_tmpdir,
        test_run_id: &scratch.run_id,
    };

    let start = std::time::Instant::now();
    println!(
        "Testing {} — {}",
        repo_root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        platform::utc_stamp()
    );
    println!("Runner order: sorted test files under planning/tests then benchmark/planning/tests");
    println!();

    let mut counts = Counts::default();
    for test in &tests {
        run_one(&config, test, &mut counts);
    }
    for crate_dir in &crates {
        run_cargo_one(&config, crate_dir, &mut counts);
    }

    let elapsed = start.elapsed().as_secs();
    println!();
    println!("──────────────────────────────────────────────");
    println!(
        "Total ran: {}   Passed: {}   Failed: {}   Skipped: {}   Unconfigured: {}",
        counts.total, counts.passed, counts.failed, counts.skipped, counts.unconfigured
    );
    println!("Elapsed: {elapsed}s");
    if !counts.failed_names.is_empty() {
        println!("Failed: {}", counts.failed_names.join(" "));
    }
    if !counts.skipped_names.is_empty() {
        println!("Skipped: {}", counts.skipped_names.join(" "));
    }
    if !counts.unconfigured_names.is_empty() {
        println!(
            "Unconfigured (set PLANNING_CONTEXT_CACHE to run): {}",
            counts.unconfigured_names.join(" ")
        );
    }
    println!("──────────────────────────────────────────────");

    if counts.failed == 0 {
        0
    } else {
        1
    }
}

fn main() {
    let code = run();
    std::process::exit(code);
}
