// MODE: DEV
//! End-to-end regression against the real compiled binary over stdio: add a
//! question, see it in list_open, answer it, see it move to list_decided and
//! drop out of list_open/list_closed. Parallel in spirit to
//! src/chat-mcp/tests/mcp_flow.rs, but with no server of its own to start --
//! every tool here is a direct, synchronous read-mutate-write of
//! DECISIONS.json, so the adapter answers each request inline with nothing to
//! wait on.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::Instant;

struct Adapter {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

fn scratch(name: &str) -> PathBuf {
    let root = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = root.join(format!(
        "decisions-mcp-flow-{name}-{}-{}",
        std::process::id(),
        Instant::now().elapsed().subsec_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    std::fs::write(
        dir.join("DECISIONS.json"),
        r#"{"skill":"decisions","skill_version":"2.0.0-alpha.7","comment":"t","questions":[]}"#,
    )
    .expect("seed DECISIONS.json");
    dir
}

impl Adapter {
    fn start(dir: &Path) -> Adapter {
        let mut child = Command::new(env!("CARGO_BIN_EXE_decisions-mcp"))
            .env("DECISIONS_JSON", dir.join("DECISIONS.json"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("decisions-mcp starts");
        let stdin = child.stdin.take().expect("adapter stdin");
        let stdout = BufReader::new(child.stdout.take().expect("adapter stdout"));
        Adapter {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    fn call(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        writeln!(self.stdin, "{request}").expect("write request");
        self.stdin.flush().expect("flush request");
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read response");
        assert!(!line.is_empty(), "adapter closed stdout without answering");
        let response: Value = serde_json::from_str(line.trim()).expect("response is JSON");
        assert_eq!(response["id"], id, "response id matches request id");
        response
    }

    fn tool(&mut self, name: &str, arguments: Value) -> Value {
        self.call("tools/call", json!({"name": name, "arguments": arguments}))
    }
}

impl Adapter {
    /// Starts the adapter the way `resolved_register_path` resolves it for
    /// real: no `DECISIONS_JSON` override, cwd at `project`, and an isolated
    /// `XDG_CONFIG_HOME`/`USER` so the registers-worktree resolution (W19) is
    /// exercised instead of bypassed.
    fn start_with_resolution(project: &Path, xdg_config_home: &Path) -> Adapter {
        let mut child = Command::new(env!("CARGO_BIN_EXE_decisions-mcp"))
            .current_dir(project)
            .env("XDG_CONFIG_HOME", xdg_config_home)
            .env("USER", TEST_USER)
            .env_remove("USERNAME")
            .env_remove("DECISIONS_JSON")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("decisions-mcp starts");
        let stdin = child.stdin.take().expect("adapter stdin");
        let stdout = BufReader::new(child.stdout.take().expect("adapter stdout"));
        Adapter {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }
}

impl Drop for Adapter {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn text_of(response: &Value) -> String {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text content in {response}"))
        .to_string()
}

#[test]
fn add_answer_and_the_status_filters_follow_it() {
    let dir = scratch("flow");
    let mut adapter = Adapter::start(&dir);

    let initialized = adapter.call("initialize", json!({}));
    assert_eq!(initialized["result"]["serverInfo"]["name"], "decisions");

    let added = adapter.tool(
        "add",
        json!({
            "title": "Which backend?",
            "options": [{"letter": "a", "label": "sqlite"}, {"letter": "b", "label": "postgres"}],
            "priority": "high",
            "context": "stubbed sqlite for now",
        }),
    );
    assert!(added.get("error").is_none(), "{added}");
    let id = text_of(&added);
    assert_eq!(id, "Q1");

    let open = adapter.tool("list_open", json!({}));
    assert!(text_of(&open).contains(&id), "{open}");

    let decided_tool = adapter.tool("answer", json!({"id": id, "letter": "b"}));
    assert!(decided_tool.get("error").is_none(), "{decided_tool}");

    let open_after = adapter.tool("list_open", json!({}));
    assert!(
        !text_of(&open_after).contains(&id),
        "decided question must leave list_open: {open_after}"
    );

    let closed_after = adapter.tool("list_closed", json!({}));
    assert!(
        !text_of(&closed_after).contains(&id),
        "a decided-but-not-closed question must not appear in list_closed: {closed_after}"
    );

    let decided_list = adapter.tool("list_decided", json!({}));
    let decided_text = text_of(&decided_list);
    assert!(decided_text.contains(&id), "{decided_text}");
    assert!(
        decided_text.contains("\"chosen\":\"b\""),
        "the recorded pick must be b: {decided_text}"
    );

    let implemented_tool = adapter.tool("implement", json!({"id": id, "note": "shipped"}));
    assert!(
        implemented_tool.get("error").is_none(),
        "{implemented_tool}"
    );

    let decided_after_implement = adapter.tool("list_decided", json!({}));
    assert!(
        !text_of(&decided_after_implement).contains(&id),
        "implemented question must leave list_decided: {decided_after_implement}"
    );

    let implemented_list = adapter.tool("list_implemented", json!({}));
    let implemented_text = text_of(&implemented_list);
    assert!(implemented_text.contains(&id), "{implemented_text}");
    assert!(implemented_text.contains("shipped"), "{implemented_text}");
}

// --- Registers-worktree resolution (W20): with a recognized worktree
// present, add/answer/implement read and write DECISIONS.json there, not in
// the adapter's own working directory; with none recognized and no
// override, behavior is unchanged from today (bare DECISIONS.json in the
// adapter's own working directory).

const TEST_USER: &str = "mcp-flow-test-user";

fn run_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

/// A real git project with one commit that already carries a seed
/// DECISIONS.json, so a worktree later forked from HEAD (no origin/<branch>
/// exists in these tests) checks one out too.
fn init_registers_project(dir: &Path) {
    run_git(dir, &["init", "-q"]);
    std::fs::write(
        dir.join("DECISIONS.json"),
        r#"{"skill":"decisions","skill_version":"2.0.0-alpha.7","comment":"t","questions":[]}"#,
    )
    .expect("seed register");
    run_git(dir, &["add", "DECISIONS.json"]);
    run_git(
        dir,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-q",
            "-m",
            "init",
        ],
    );
}

/// The canonical registers-worktree path `decisions`/`decisions-mcp` would
/// compute for `project`, given no git remote (the user/projectdir fallback).
fn candidate_registers_root(xdg_config_home: &Path, project: &Path) -> PathBuf {
    let project_dir = project.file_name().unwrap().to_str().unwrap();
    xdg_config_home
        .join("tsch-ai-skills")
        .join("registers")
        .join(TEST_USER)
        .join(project_dir)
}

/// Creates a recognized registers worktree directly against real git -- the
/// same shape the `decisions` CLI's own acceptance path produces (goal 06's
/// relocation, or W15's own first-use accept) -- rather than through a real
/// interactive prompt, which needs a pty this test has none of.
fn make_recognized_worktree(project: &Path, candidate: &Path, branch: &str) {
    run_git(
        project,
        &["worktree", "add", "-b", branch, candidate.to_str().unwrap()],
    );
    run_git(candidate, &["sparse-checkout", "init", "--no-cone"]);
    run_git(
        candidate,
        &[
            "sparse-checkout",
            "set",
            "/BUGS.json",
            "/TODO.json",
            "/DECISIONS.json",
        ],
    );
}

#[test]
fn a_recognized_worktree_is_used_for_every_tool_call_not_the_working_directory() {
    let project = tempfile::tempdir().unwrap();
    init_registers_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_registers_root(home.path(), project.path());
    make_recognized_worktree(project.path(), &candidate, "registers");

    let mut adapter = Adapter::start_with_resolution(project.path(), home.path());
    let added = adapter.tool(
        "add",
        json!({
            "title": "Which backend?",
            "options": [{"letter": "a", "label": "sqlite"}, {"letter": "b", "label": "postgres"}],
        }),
    );
    assert!(added.get("error").is_none(), "{added}");
    let id = text_of(&added);
    assert_eq!(id, "Q1");

    let answered = adapter.tool("answer", json!({"id": id, "letter": "b"}));
    assert!(answered.get("error").is_none(), "{answered}");
    let implemented = adapter.tool("implement", json!({"id": id, "note": "shipped"}));
    assert!(implemented.get("error").is_none(), "{implemented}");

    let worktree_text = std::fs::read_to_string(candidate.join("DECISIONS.json")).unwrap();
    assert!(worktree_text.contains(&id), "{worktree_text}");
    assert!(worktree_text.contains("shipped"), "{worktree_text}");

    let project_text = std::fs::read_to_string(project.path().join("DECISIONS.json")).unwrap();
    assert!(
        !project_text.contains(&id),
        "the project's own working-tree copy must be untouched: {project_text}"
    );
}

#[test]
fn with_nothing_recognized_the_bare_file_in_the_working_directory_is_used_unchanged() {
    let project = tempfile::tempdir().unwrap();
    init_registers_project(project.path());
    let home = tempfile::tempdir().unwrap();
    // No worktree, no config -- genuinely undecided, but decisions-mcp never
    // prompts or creates anything (W19), so this must stay on the bare file
    // in the adapter's own working directory, exactly as it did before this
    // plan.

    let mut adapter = Adapter::start_with_resolution(project.path(), home.path());
    let added = adapter.tool(
        "add",
        json!({
            "title": "Which backend?",
            "options": [{"letter": "a", "label": "sqlite"}, {"letter": "b", "label": "postgres"}],
        }),
    );
    assert!(added.get("error").is_none(), "{added}");
    let id = text_of(&added);
    assert_eq!(id, "Q1");

    let project_text = std::fs::read_to_string(project.path().join("DECISIONS.json")).unwrap();
    assert!(project_text.contains(&id), "{project_text}");

    let candidate = candidate_registers_root(home.path(), project.path());
    assert!(!candidate.exists(), "nothing was ever created");
}
