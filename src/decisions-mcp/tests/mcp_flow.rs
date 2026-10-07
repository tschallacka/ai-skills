// MODE: DEV
//! End-to-end regression against the real compiled binary over stdio: add a
//! question, see it in list_open, answer it, see it move to list_answered and
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
        r#"{"skill":"decisions","skill_version":"2.0.0-alpha.4","comment":"t","questions":[]}"#,
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

    let answered_tool = adapter.tool("answer", json!({"id": id, "letter": "b"}));
    assert!(answered_tool.get("error").is_none(), "{answered_tool}");

    let open_after = adapter.tool("list_open", json!({}));
    assert!(
        !text_of(&open_after).contains(&id),
        "answered question must leave list_open: {open_after}"
    );

    let closed_after = adapter.tool("list_closed", json!({}));
    assert!(
        !text_of(&closed_after).contains(&id),
        "an answered-but-not-closed question must not appear in list_closed: {closed_after}"
    );

    let answered_list = adapter.tool("list_answered", json!({}));
    let answered_text = text_of(&answered_list);
    assert!(answered_text.contains(&id), "{answered_text}");
    assert!(
        answered_text.contains("\"chosen\":\"b\""),
        "the recorded pick must be b: {answered_text}"
    );
}
