// MODE: DEV
// PACKAGE: PROD
//! Real subprocess-level integration tests: a real planning-server binary,
//! started as a genuine child process with its own socket, driven by the
//! real planning-client binary (and, for the parity test, the real
//! planning-mcp binary over real stdio JSON-RPC) -- not planning-server's
//! own in-process handlers::dispatch, which the crate's own unit tests
//! already cover. This is the level at which the CAS-conflict race, the
//! external-write-detection race, and server restart actually happen.

use planning_server::revision::read_with_revision;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "planning-server-integration-{}-{}",
            std::process::id(),
            unique()
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Walks up from this test binary's own path to the workspace's shared
/// target/{debug,release} directory, where every sibling binary this suite
/// drives (planning-server, planning-client, planning-mcp, create-plan,
/// add-goal, create-adversarial-review, add-work-unit) already lands from
/// being built alongside this crate.
fn sibling_bin_dir() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current test binary path");
    dir.pop(); // the test binary itself
    if dir.file_name().is_some_and(|name| name == "deps") {
        dir.pop();
    }
    dir
}

fn run(bin_dir: &Path, name: &str, args: &[&str]) {
    let program = bin_dir.join(name);
    let output = Command::new(&program)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("could not run {}: {error}", program.display()));
    assert!(
        output.status.success(),
        "{} {:?} failed: {}",
        program.display(),
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn setup_plan(bin_dir: &Path, dir: &Path) -> PathBuf {
    let plan_dir = dir.join("plan");
    run(
        bin_dir,
        "create-plan",
        &[plan_dir.to_str().unwrap(), "Demo plan"],
    );
    run(
        bin_dir,
        "add-goal",
        &[
            plan_dir.to_str().unwrap(),
            "01-demo",
            "Demo goal",
            "Demo outcome",
        ],
    );
    run(
        bin_dir,
        "create-adversarial-review",
        &[plan_dir.to_str().unwrap()],
    );
    plan_dir
}

fn cloned_plan(scratch: &Path, container: &str, src: &Path) -> PathBuf {
    let dst_parent = scratch.join(container);
    fs::create_dir_all(&dst_parent).unwrap();
    let output = Command::new("cp")
        .args(["-r", &src.to_string_lossy(), &dst_parent.to_string_lossy()])
        .output()
        .expect("run cp -r");
    assert!(
        output.status.success(),
        "cp -r failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    dst_parent.join(src.file_name().unwrap())
}

/// A running planning-server child process, on its own runtime directory
/// (so concurrent tests never collide on the same socket path), killed and
/// reaped when dropped.
struct ServerGuard {
    child: Child,
    runtime_dir: PathBuf,
}

impl ServerGuard {
    fn start(bin_dir: &Path, scratch: &Path, tag: &str) -> Self {
        let runtime_dir = scratch.join(format!("runtime-{tag}"));
        fs::create_dir_all(&runtime_dir).unwrap();
        let child = Command::new(bin_dir.join("planning-server"))
            .env("XDG_RUNTIME_DIR", &runtime_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn planning-server");
        let socket = planning_server::endpoint::resolve(&runtime_dir);
        for _ in 0..200 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            socket.exists(),
            "planning-server did not create its socket in time"
        );
        ServerGuard { child, runtime_dir }
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn client_output(bin_dir: &Path, server: &ServerGuard, args: &[&str]) -> std::process::Output {
    Command::new(bin_dir.join("planning-client"))
        .env("XDG_RUNTIME_DIR", &server.runtime_dir)
        .args(args)
        .output()
        .expect("run planning-client")
}

fn client_ok(bin_dir: &Path, server: &ServerGuard, args: &[&str]) {
    let output = client_output(bin_dir, server, args);
    assert!(
        output.status.success(),
        "planning-client {:?} failed (exit {:?}): {}",
        args,
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn mcp_call(bin_dir: &Path, tool: &str, arguments: Value) -> Value {
    let mut child = Command::new(bin_dir.join("planning-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn planning-mcp");
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": tool, "arguments": arguments}});
    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(stdin, "{request}").unwrap();
    }
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let _ = child.kill();
    let _ = child.wait();
    serde_json::from_str(&line)
        .unwrap_or_else(|error| panic!("malformed MCP response {line:?}: {error}"))
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn assert_snapshots_match(a: &Path, b: &Path) {
    let snap_a = snapshot(a);
    let snap_b = snapshot(b);
    if snap_a == snap_b {
        return;
    }
    let mut report = String::new();
    for key in snap_a
        .keys()
        .chain(snap_b.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        match (snap_a.get(key), snap_b.get(key)) {
            (Some(x), Some(y)) if x != y => {
                report.push_str(&format!(
                    "--- differs: {} ---\nA: {}\nB: {}\n",
                    key.display(),
                    String::from_utf8_lossy(x),
                    String::from_utf8_lossy(y)
                ));
            }
            (Some(_), None) => report.push_str(&format!("only in A: {}\n", key.display())),
            (None, Some(_)) => report.push_str(&format!("only in B: {}\n", key.display())),
            _ => {}
        }
    }
    panic!("snapshots differ:\n{report}");
}

#[test]
fn cas_conflict_exactly_one_of_two_concurrent_guarded_writes_succeeds() {
    let bin_dir = sibling_bin_dir();
    let scratch = TempDir::new();
    let plan_dir = setup_plan(&bin_dir, scratch.path());
    run(
        &bin_dir,
        "add-work-unit",
        &[
            plan_dir.to_str().unwrap(),
            "--id",
            "W01",
            "--type",
            "source",
            "--file",
            "src/x.rs",
            "--scope",
            "x",
            "--subscope",
            "N/A",
            "--change",
            "do x",
            "--depends-on",
            "--",
            "--goal",
            "01-demo",
            "--step",
            "01-step-x",
        ],
    );
    let server = ServerGuard::start(&bin_dir, scratch.path(), "cas-conflict");

    // update-step's own guarded file is the goal's progress.md, not the step
    // document itself -- confirmed directly against the real standalone
    // binary (it leaves the step .md file completely untouched).
    let progress_path = plan_dir.join("01-demo").join("progress.md");
    let (_, revision) = read_with_revision(&progress_path).unwrap();
    let revision_hex = revision.to_hex();

    let bin_dir_a = bin_dir.clone();
    let runtime_a = server.runtime_dir.clone();
    let plan_dir_a = plan_dir.clone();
    let revision_a = revision_hex.clone();
    let handle_a = std::thread::spawn(move || {
        Command::new(bin_dir_a.join("planning-client"))
            .env("XDG_RUNTIME_DIR", &runtime_a)
            .args([
                "update-step",
                plan_dir_a.to_str().unwrap(),
                "01-demo",
                "01-step-x",
                "in-progress",
                "--revision",
                &revision_a,
            ])
            .output()
            .unwrap()
    });
    let bin_dir_b = bin_dir.clone();
    let runtime_b = server.runtime_dir.clone();
    let plan_dir_b = plan_dir.clone();
    let revision_b = revision_hex.clone();
    let handle_b = std::thread::spawn(move || {
        Command::new(bin_dir_b.join("planning-client"))
            .env("XDG_RUNTIME_DIR", &runtime_b)
            .args([
                "update-step",
                plan_dir_b.to_str().unwrap(),
                "01-demo",
                "01-step-x",
                "completed",
                "--revision",
                &revision_b,
            ])
            .output()
            .unwrap()
    });

    let output_a = handle_a.join().unwrap();
    let output_b = handle_b.join().unwrap();
    let successes = [&output_a, &output_b]
        .iter()
        .filter(|output| output.status.success())
        .count();
    let stale = [&output_a, &output_b]
        .iter()
        .filter(|output| output.status.code() == Some(65))
        .count();
    assert_eq!(
        successes,
        1,
        "exactly one of the two racing writes must succeed (a: {:?}/{:?}, b: {:?}/{:?})",
        output_a.status.code(),
        String::from_utf8_lossy(&output_a.stderr),
        output_b.status.code(),
        String::from_utf8_lossy(&output_b.stderr)
    );
    assert_eq!(stale, 1, "the other racing write must observe Stale");
}

#[test]
fn external_write_between_read_and_guarded_write_is_detected_as_stale() {
    let bin_dir = sibling_bin_dir();
    let scratch = TempDir::new();
    let plan_dir = setup_plan(&bin_dir, scratch.path());
    run(
        &bin_dir,
        "add-work-unit",
        &[
            plan_dir.to_str().unwrap(),
            "--id",
            "W01",
            "--type",
            "source",
            "--file",
            "src/x.rs",
            "--scope",
            "x",
            "--subscope",
            "N/A",
            "--change",
            "do x",
            "--depends-on",
            "--",
            "--goal",
            "01-demo",
            "--step",
            "01-step-x",
        ],
    );
    let server = ServerGuard::start(&bin_dir, scratch.path(), "external-write");

    // update-step's own guarded file is the goal's progress.md, not the step
    // document itself -- confirmed directly against the real standalone
    // binary (it leaves the step .md file completely untouched).
    let progress_path = plan_dir.join("01-demo").join("progress.md");
    let (_, revision) = read_with_revision(&progress_path).unwrap();

    // Simulate an external writer (a still-bash-fallback command, or any
    // process this MVP does not route through the server) racing in
    // between this client's own read and its guarded write.
    let mut external_bytes = fs::read(&progress_path).unwrap();
    external_bytes.extend_from_slice(b"\n<external change>\n");
    fs::write(&progress_path, &external_bytes).unwrap();

    let output = client_output(
        &bin_dir,
        &server,
        &[
            "update-step",
            plan_dir.to_str().unwrap(),
            "01-demo",
            "01-step-x",
            "in-progress",
            "--revision",
            &revision.to_hex(),
        ],
    );
    assert_eq!(output.status.code(), Some(65), "expected exit 65 (Stale)");
    assert_eq!(
        fs::read(&progress_path).unwrap(),
        external_bytes,
        "a stale write must leave the external change untouched"
    );
}

#[test]
fn server_lifecycle_start_call_stop_and_clean_restart() {
    let bin_dir = sibling_bin_dir();
    let scratch = TempDir::new();
    let plan_dir = setup_plan(&bin_dir, scratch.path());

    // A plain read-only call, not validate-plan: a freshly create-plan-built
    // scratch plan (no completed work units, no filled-in placeholders)
    // legitimately fails validation, which would make this call's own
    // success assertion about server lifecycle rather than plan health.
    let server = ServerGuard::start(&bin_dir, scratch.path(), "lifecycle");
    client_ok(
        &bin_dir,
        &server,
        &["read-plan-document", plan_dir.to_str().unwrap(), "plan"],
    );
    drop(server); // stop: kills and reaps the child

    // A clean restart against the SAME plan directory (a different runtime
    // dir, matching how a real restart on the same host would get a fresh
    // socket) must serve it with no corruption from the previous run.
    let server_two = ServerGuard::start(&bin_dir, scratch.path(), "lifecycle-restart");
    let before = snapshot(&plan_dir);
    client_ok(
        &bin_dir,
        &server_two,
        &["read-plan-document", plan_dir.to_str().unwrap(), "plan"],
    );
    assert_eq!(
        before,
        snapshot(&plan_dir),
        "a read-only call after restart must not change anything on disk"
    );
}

#[test]
fn cli_and_mcp_produce_identical_results_for_all_seven_operations() {
    let bin_dir = sibling_bin_dir();
    let scratch = TempDir::new();
    let plan_dir = setup_plan(&bin_dir, scratch.path());
    let server = ServerGuard::start(&bin_dir, scratch.path(), "parity");

    // 1: read_plan_document / read-plan-document
    let cli_read = client_output(
        &bin_dir,
        &server,
        &["read-plan-document", plan_dir.to_str().unwrap(), "plan"],
    );
    let mcp_read = mcp_call(
        &bin_dir,
        "read_plan_document",
        json!({"plan_dir": plan_dir.to_string_lossy(), "document_id": "plan"}),
    );
    assert!(cli_read.status.success());
    let cli_text = String::from_utf8_lossy(&cli_read.stdout);
    let mcp_text = mcp_read["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        mcp_text.starts_with(cli_text.trim_end()),
        "MCP read content must match the CLI's own read content"
    );

    // 2: add_work_unit / add-work-unit, compared byte-for-byte on equivalent copies
    let copy_cli = cloned_plan(scratch.path(), "add-work-unit-cli", &plan_dir);
    let copy_mcp = cloned_plan(scratch.path(), "add-work-unit-mcp", &plan_dir);
    client_ok(
        &bin_dir,
        &server,
        &[
            "add-work-unit",
            copy_cli.to_str().unwrap(),
            "W01",
            "source",
            "src/x.rs",
            "x",
            "N/A",
            "do x",
            "--",
            "01-demo",
            "01-step-x",
        ],
    );
    let response = mcp_call(
        &bin_dir,
        "add_work_unit",
        json!({
            "plan_dir": copy_mcp.to_string_lossy(),
            "id": "W01",
            "type": "source",
            "file": "src/x.rs",
            "scope": "x",
            "subscope": "N/A",
            "change": "do x",
            "depends_on": "--",
            "goal": "01-demo",
            "step": "01-step-x",
        }),
    );
    assert_ne!(
        response["result"]["isError"], true,
        "add_work_unit tool call should not error: {response}"
    );
    assert_snapshots_match(&copy_cli, &copy_mcp);

    // 3: read_work_unit / read-work-unit (against the copy we just populated)
    let cli_unit = client_output(
        &bin_dir,
        &server,
        &["read-work-unit", copy_cli.to_str().unwrap(), "W01"],
    );
    let mcp_unit = mcp_call(
        &bin_dir,
        "read_work_unit",
        json!({"plan_dir": copy_mcp.to_string_lossy(), "unit_id": "W01"}),
    );
    assert!(cli_unit.status.success());
    let cli_unit_text = String::from_utf8_lossy(&cli_unit.stdout);
    let mcp_unit_text = mcp_unit["result"]["content"][0]["text"].as_str().unwrap();
    assert!(mcp_unit_text.starts_with(cli_unit_text.trim_end()));

    // 4: update_step / update-step
    let copy_cli = cloned_plan(scratch.path(), "update-step-cli", &copy_cli);
    let copy_mcp = cloned_plan(scratch.path(), "update-step-mcp", &copy_mcp);
    client_ok(
        &bin_dir,
        &server,
        &[
            "update-step",
            copy_cli.to_str().unwrap(),
            "01-demo",
            "01-step-x",
            "in-progress",
        ],
    );
    let response = mcp_call(
        &bin_dir,
        "update_step",
        json!({"plan_dir": copy_mcp.to_string_lossy(), "goal": "01-demo", "step": "01-step-x", "status": "in-progress"}),
    );
    assert_ne!(
        response["result"]["isError"], true,
        "update_step tool call should not error: {response}"
    );
    assert_snapshots_match(&copy_cli, &copy_mcp);

    // 5: set_review_status / set-review-status
    let copy_cli = cloned_plan(scratch.path(), "review-status-cli", &copy_cli);
    let copy_mcp = cloned_plan(scratch.path(), "review-status-mcp", &copy_mcp);
    client_ok(
        &bin_dir,
        &server,
        &["set-review-status", copy_cli.to_str().unwrap(), "pending"],
    );
    let response = mcp_call(
        &bin_dir,
        "set_review_status",
        json!({"plan_dir": copy_mcp.to_string_lossy(), "status": "pending"}),
    );
    assert_ne!(
        response["result"]["isError"], true,
        "set_review_status tool call should not error: {response}"
    );
    assert_snapshots_match(&copy_cli, &copy_mcp);

    // 6: set_testing_requirement / set-testing-requirement
    let copy_cli = cloned_plan(scratch.path(), "testing-req-cli", &copy_cli);
    let copy_mcp = cloned_plan(scratch.path(), "testing-req-mcp", &copy_mcp);
    client_ok(
        &bin_dir,
        &server,
        &[
            "set-testing-requirement",
            copy_cli.to_str().unwrap(),
            "01-demo",
            "yes",
            "exercised in parity test",
        ],
    );
    let response = mcp_call(
        &bin_dir,
        "set_testing_requirement",
        json!({
            "plan_dir": copy_mcp.to_string_lossy(),
            "goal": "01-demo",
            "required": true,
            "rationale": "exercised in parity test",
        }),
    );
    assert_ne!(
        response["result"]["isError"], true,
        "set_testing_requirement tool call should not error: {response}"
    );
    assert_snapshots_match(&copy_cli, &copy_mcp);

    // 7: validate_plan / validate-plan (read-only; compare pass/fail, not the full report text,
    // since the CLI's stderr/stdout split differs slightly from the MCP adapter's single combined string)
    let cli_validate = client_output(
        &bin_dir,
        &server,
        &["validate-plan", copy_cli.to_str().unwrap()],
    );
    let mcp_validate = mcp_call(
        &bin_dir,
        "validate_plan",
        json!({"plan_dir": copy_mcp.to_string_lossy()}),
    );
    let mcp_passed = mcp_validate["result"]["isError"] != true;
    assert_eq!(
        cli_validate.status.success(),
        mcp_passed,
        "CLI and MCP must agree on whether the plan validates"
    );
}
