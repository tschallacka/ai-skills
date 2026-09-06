// MODE: DEV
// End-to-end regressions for the MCP adapter's own wire behavior, driven
// the way a harness drives it: raw JSON-RPC over stdio against the real
// autostarted server. The first two drives found the adapter answering a
// refused edit as if it had applied, and discarding the revision guard a
// schema-following client was told to send; until now nothing in the suite
// covered the adapter at all.
#![cfg(unix)]

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Harness {
    scratch: PathBuf,
    agent: String,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    child: Child,
}

impl Harness {
    fn new(name: &str) -> Option<Self> {
        let root = std::env::var_os("TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let scratch = root.join(format!(
            "ai-text-editor-mcp-flow-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_micros()
        ));
        for dir in ["runtime", "meta", "sessions"] {
            std::fs::create_dir_all(scratch.join(dir)).unwrap();
        }
        let scratch = std::fs::canonicalize(scratch).unwrap();
        std::fs::write(scratch.join("doc.txt"), "alpha\nbeta\n").unwrap();
        // The adapter resolves its server as a sibling of its own
        // executable. A leg that builds the workspace places that sibling,
        // and the protocol flow is then driven in full. The per-crate suite
        // legs do not order this guarantee — run-tests.sh sorts crates with
        // the ambient locale, and under LC_ALL=C `ai-text-editor-mcp`
        // lands before `ai-text-editor`, with no server built yet. There
        // the flow names what it skipped rather than failing on suite
        // choreography or passing quietly.
        let adapter_dir = std::path::Path::new(env!("CARGO_BIN_EXE_ai-text-editor-mcp"))
            .parent()
            .expect("the adapter binary lives in a directory")
            .to_path_buf();
        if !adapter_dir.join("ai-text-editor-server").is_file() {
            eprintln!(
                "mcp_flow[{name}]: skipped — no ai-text-editor-server beside the \
                 adapter in this build; `cargo test --workspace` drives the flow"
            );
            let _ = std::fs::remove_dir_all(&scratch);
            return None;
        }
        let mut child = Command::new(env!("CARGO_BIN_EXE_ai-text-editor-mcp"))
            .env("HOME", &scratch)
            .env("XDG_RUNTIME_DIR", scratch.join("runtime"))
            .env("TSCH_AI_EDITOR_METADATA_DIR", scratch.join("meta"))
            .env("TSCH_AI_EDITOR_SESSION_DIR", scratch.join("sessions"))
            .env_remove("TSCH_AI_EDITOR_AGENT")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CODEX_SESSION_ID")
            .env_remove("OPENCODE_PID")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the adapter binary must run");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Some(Self {
            scratch,
            agent: format!("mcp-flow-{name}"),
            stdin,
            stdout,
            child,
        })
    }

    fn file(&self) -> String {
        self.scratch.join("doc.txt").to_string_lossy().into_owned()
    }

    fn call(&mut self, id: i64, tool: &str, arguments: Vec<(&str, Value)>) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("agent".into(), json!(self.agent));
        for (key, value) in arguments {
            map.insert(key.into(), value);
        }
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": tool, "arguments": map},
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

    fn tools_list(&mut self, id: i64) -> Value {
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/list",
            "params": {},
        });
        writeln!(self.stdin, "{request}").expect("adapter stdin must accept a request");
        self.stdin.flush().unwrap();
        let mut line = String::new();
        let read = self
            .stdout
            .read_line(&mut line)
            .expect("adapter must answer one line per request");
        assert!(
            read > 0,
            "adapter closed the pipe without answering tools/list"
        );
        serde_json::from_str(line.trim()).expect("adapter answers one JSON line per request")
    }

    fn content(response: &Value) -> String {
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }

    fn frames(response: &Value) -> Vec<Value> {
        let text = Self::content(response);
        serde_json::from_str(&text)
            .unwrap_or_else(|_| panic!("content is not a frame array: {text}"))
    }

    fn refused(response: &Value) -> bool {
        response["result"]["isError"].as_bool().unwrap_or(false)
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        // The server the adapter autostarted is its child; when a test
        // panicked before its close, stop it by the recorded pid rather
        // than leaving it for the idle watchdog.
        let endpoint_root = self.scratch.join("runtime").join("tsch-ai-skills-editor");
        if let Ok(entries) = std::fs::read_dir(&endpoint_root) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.ends_with(".endpoint") {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(entry.path()) else {
                    continue;
                };
                if let Ok(value) = serde_json::from_str::<Value>(&content) {
                    if let Some(pid) = value.get("pid").and_then(Value::as_u64) {
                        unsafe { libc::kill(pid as libc::c_int, libc::SIGKILL) };
                    }
                }
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

#[test]
fn a_schema_following_string_revision_satisfies_the_guard() {
    let Some(mut h) = Harness::new("stringguard") else {
        return;
    };
    let opened = h.call(1, "open", vec![("file", json!(h.file()))]);
    assert!(!Harness::refused(&opened), "open refused: {opened}");
    let inserted = h.call(
        2,
        "insert",
        vec![
            ("file", json!(h.file())),
            ("offset", json!(6)),
            ("text", json!("gamma\n")),
            ("expected_revision", json!("0")),
        ],
    );
    assert!(
        !Harness::refused(&inserted),
        "the advertised string form of the guard was refused: {inserted}"
    );
    let frames = Harness::frames(&inserted);
    assert_eq!(frames[0]["payload"]["revision"], json!(1), "{frames:?}");
}

#[test]
fn a_refusal_is_marked_refused_on_the_wire() {
    let Some(mut h) = Harness::new("iserror") else {
        return;
    };
    let opened = h.call(1, "open", vec![("file", json!(h.file()))]);
    assert!(!Harness::refused(&opened), "open refused: {opened}");
    let stale = h.call(
        2,
        "insert",
        vec![
            ("file", json!(h.file())),
            ("offset", json!(6)),
            ("text", json!("gamma\n")),
            ("expected_revision", json!("41")),
        ],
    );
    assert!(
        Harness::refused(&stale),
        "a stale revision answered as success: {stale}"
    );
    let frames = Harness::frames(&stale);
    assert_eq!(frames[0]["code"], json!("stale_revision"), "{frames:?}");
    let junk = h.call(
        3,
        "insert",
        vec![
            ("file", json!(h.file())),
            ("offset", json!(6)),
            ("text", json!("gamma\n")),
            ("expected_revision", json!("not-a-revision")),
        ],
    );
    assert!(
        Harness::refused(&junk),
        "an unparsable guard answered as success: {junk}"
    );
    let junk_text = Harness::content(&junk);
    assert!(
        junk_text.contains("expected_revision"),
        "an unsatisfiable guard must name itself, not the server's generic refusal: {junk_text}"
    );
    assert!(
        !junk_text.contains("revision_required"),
        "the bad argument must not hide behind revision_required: {junk_text}"
    );
}

#[test]
fn the_page_tool_advertises_and_honours_the_historical_escape() {
    // B211: the transport forwarded `historical` and the server honoured it,
    // but `page`'s published schema omitted the key while two lines below
    // `search` declared it - a schema-following client was stripped of the
    // documented stale-result escape and saw only the plain refusal.
    let Some(mut h) = Harness::new("historical") else {
        return;
    };
    let listing = h.tools_list(1);
    let page = listing["result"]["tools"]
        .as_array()
        .expect("a tools array")
        .iter()
        .find(|tool| tool["name"] == json!("page"))
        .expect("the page tool is published");
    assert!(
        page["inputSchema"]["properties"]
            .get("historical")
            .is_some(),
        "page's schema still hides the escape: {page}"
    );
    let opened = h.call(2, "open", vec![("file", json!(h.file()))]);
    assert!(!Harness::refused(&opened), "open refused: {opened}");
    let found = h.call(
        3,
        "search",
        vec![
            ("file", json!(h.file())),
            ("mode", json!("exact_text")),
            ("query", json!("alpha")),
        ],
    );
    let pager_key = Harness::frames(&found)[0]["payload"]["pager_key"]
        .as_str()
        .expect("a pager key")
        .to_string();
    let inserted = h.call(
        4,
        "insert",
        vec![
            ("file", json!(h.file())),
            ("offset", json!(0)),
            ("text", json!("Z")),
            ("expected_revision", json!("0")),
        ],
    );
    assert!(!Harness::refused(&inserted), "edit refused: {inserted}");
    let plain = h.call(
        5,
        "page",
        vec![
            ("file", json!(h.file())),
            ("pager_key", json!(pager_key.clone())),
        ],
    );
    assert!(
        Harness::refused(&plain),
        "a post-edit page must still refuse by default: {plain}"
    );
    let escaped = h.call(
        6,
        "page",
        vec![
            ("file", json!(h.file())),
            ("pager_key", json!(pager_key)),
            ("historical", json!(true)),
        ],
    );
    assert!(
        !Harness::refused(&escaped),
        "the advertised escape was refused: {escaped}"
    );
    let frames = Harness::frames(&escaped);
    assert_eq!(frames[0]["payload"]["stale"], json!(true), "{frames:?}");
    assert!(
        frames[0]["payload"].get("source_revision").is_some(),
        "a historical page must name its source revision: {frames:?}"
    );
    let closed = h.call(
        7,
        "close",
        vec![
            ("file", json!(h.file())),
            ("journal_action", json!("clean")),
        ],
    );
    assert!(!Harness::refused(&closed), "close refused: {closed}");
}
