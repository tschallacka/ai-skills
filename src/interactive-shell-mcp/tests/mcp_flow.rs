// MODE: DEV
// End-to-end regressions for the MCP adapter's own wire behavior, driven the
// way a real MCP client drives it: raw JSON-RPC over stdio against the real
// adapter binary, which in turn spawns the real `interactive-shell` binary
// for its `start` tool.
#![cfg(unix)]

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Harness {
    scratch: PathBuf,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    child: Child,
}

impl Harness {
    /// The adapter resolves `interactive-shell` as a sibling of its own
    /// executable. A workspace-wide build places that sibling in the SAME
    /// `target/debug/` directory every crate's `[[bin]]` targets land in;
    /// `cargo test -p interactive-shell-mcp` alone does not build the
    /// OTHER package's binary, so this names what it skipped rather than
    /// failing on suite choreography or passing quietly -- the same
    /// contract ai-text-editor-mcp's own flow test uses for its sibling
    /// server binary.
    fn new(name: &str) -> Option<Self> {
        let root = std::env::var_os("TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let scratch = root.join(format!(
            "interactive-shell-mcp-flow-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_micros()
        ));
        std::fs::create_dir_all(&scratch).unwrap();
        let scratch = std::fs::canonicalize(scratch).unwrap();

        let adapter_dir = std::path::Path::new(env!("CARGO_BIN_EXE_interactive-shell-mcp"))
            .parent()
            .expect("the adapter binary lives in a directory")
            .to_path_buf();
        if !adapter_dir.join("interactive-shell").is_file() {
            eprintln!(
                "mcp_flow[{name}]: skipped -- no interactive-shell beside the adapter in \
                 this build; `cargo test --workspace` drives the flow"
            );
            let _ = std::fs::remove_dir_all(&scratch);
            return None;
        }

        let mut command = Command::new(env!("CARGO_BIN_EXE_interactive-shell-mcp"));
        command
            .env("INTERACTIVE_SHELL_HOME", &scratch)
            .env_remove("INTERACTIVE_SHELL_AGENT")
            .env_remove("CODEX_AGENT_ID")
            .env_remove("AGENT_ID")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        // The adapter's own `start` tool spawns interactive-shell as ITS
        // child, one level below this harness's direct child -- a session
        // of its own, inherited by everything the adapter spawns, so one
        // killpg on the adapter's pid reaches the grandchild too, exactly
        // like ai-text-editor-mcp's own harness does for its autostarted
        // server.
        unsafe {
            use std::os::unix::process::CommandExt;
            command.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        let mut child = command.spawn().expect("the adapter binary must run");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Some(Self {
            scratch,
            stdin,
            stdout,
            child,
        })
    }

    fn call(&mut self, id: i64, tool: &str, arguments: Value) -> Value {
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": tool, "arguments": arguments},
        });
        writeln!(self.stdin, "{request}").expect("adapter stdin must accept a request");
        self.stdin.flush().unwrap();
        let mut line = String::new();
        let read = self
            .stdout
            .read_line(&mut line)
            .expect("adapter must answer one line per request");
        assert!(read > 0, "adapter closed the pipe without answering {tool}");
        serde_json::from_str(line.trim()).expect("adapter answers one JSON line per request")
    }

    fn content(response: &Value) -> Value {
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("response carried no content text: {response}"));
        serde_json::from_str(text).unwrap_or_else(|_| json!(text))
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        unsafe {
            libc::killpg(self.child.id() as libc::c_int, libc::SIGKILL);
        }
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

#[test]
fn start_text_view_and_shutdown_drive_a_real_session() {
    let Some(mut harness) = Harness::new("basic") else {
        return;
    };
    let start = harness.call(
        1,
        "start",
        json!({
            "command": ["sh", "-c", "printf hello; sleep 600"],
            "session": "flow-basic",
            "cols": 20,
            "rows": 4,
            "idle_timeout_seconds": 600
        }),
    );
    let started = Harness::content(&start);
    assert_eq!(started["started"], json!(true));
    assert_eq!(
        started["ready"],
        json!(true),
        "session never became ready: {started}"
    );

    let view = harness.call(2, "view", json!({"session":"flow-basic"}));
    let view = Harness::content(&view);
    let responses = view["responses"]
        .as_array()
        .unwrap_or_else(|| panic!("view returned no responses: {view}"));
    let view_event = responses
        .iter()
        .find(|event| event["event"] == "view")
        .unwrap_or_else(|| panic!("no view event in {responses:?}"));
    assert!(
        view_event["text"]
            .as_str()
            .unwrap_or_default()
            .contains("hello"),
        "expected \"hello\" on screen, got {view_event}"
    );

    let shutdown = harness.call(3, "shutdown", json!({"session":"flow-basic"}));
    let shutdown = Harness::content(&shutdown);
    assert!(
        shutdown["responses"]
            .as_array()
            .is_some_and(|responses| responses.iter().any(|event| event["event"] == "ack")),
        "shutdown was not acknowledged: {shutdown}"
    );
}

#[test]
fn an_unknown_tool_reports_iserror_over_the_wire() {
    let Some(mut harness) = Harness::new("unknown-tool") else {
        return;
    };
    let response = harness.call(1, "not-a-real-tool", json!({}));
    assert_eq!(response["result"]["isError"], json!(true));
}

#[test]
fn tools_list_is_reachable_with_no_session_running() {
    let Some(mut harness) = Harness::new("tools-list") else {
        return;
    };
    let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}});
    writeln!(harness.stdin, "{request}").unwrap();
    harness.stdin.flush().unwrap();
    let mut line = String::new();
    harness.stdout.read_line(&mut line).unwrap();
    let response: Value = serde_json::from_str(line.trim()).unwrap();
    let names: Vec<&str> = response["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    for expected in ["start", "text", "view", "observe", "wait", "shutdown"] {
        assert!(names.contains(&expected), "tools/list missing {expected}");
    }
}
