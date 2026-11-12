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
        let mut command = Command::new(env!("CARGO_BIN_EXE_ai-text-editor-mcp"));
        command
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
            .stderr(Stdio::null());
        // B239's shape, on this harness too: the adapter autostarts a server
        // that outlives it, and `Drop` used to find it only by reading the pid
        // out of an `.endpoint` record. A record that has not been written yet,
        // or that landed in the length fallback outside this scratch tree, left
        // the server running — six survived one run of this file. A session of
        // its own is inherited by the adapter's own children, so one `killpg`
        // stops the lot whether a record ever named them or not.
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
        // The adapter and every server it autostarted share one session, so
        // this stops them all without needing a record that names them. The
        // record sweep below stays as a backstop, and now reads both roots -
        // it knew only the configured one, and a long TMPDIR puts every record
        // in the other.
        unsafe {
            libc::killpg(self.child.id() as libc::c_int, libc::SIGKILL);
        }
        for endpoint_root in
            ai_text_editor::transport::endpoint_roots(&self.scratch.join("runtime"))
        {
            let Ok(entries) = std::fs::read_dir(&endpoint_root) else {
                continue;
            };
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
        // Outside the scratch tree by construction, and keyed to this
        // harness's own runtime directory, so this removes nothing else.
        let [_, fallback] =
            ai_text_editor::transport::endpoint_roots(&self.scratch.join("runtime"));
        let _ = std::fs::remove_dir_all(fallback);
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

/// T124/B310: "path" is the name an agent's instinct reaches for; it must
/// open the file exactly as "file" would, not refuse as an unknown argument.
#[test]
fn the_path_argument_opens_a_file_exactly_like_the_file_argument() {
    let Some(mut h) = Harness::new("patharg") else {
        return;
    };
    let opened = h.call(1, "open", vec![("path", json!(h.file()))]);
    assert!(!Harness::refused(&opened), "open refused: {opened}");
    let frames = Harness::frames(&opened);
    assert_eq!(
        frames[0]["payload"]["path"],
        json!(h.file()),
        "the tab must report the real path, whichever argument named it: {frames:?}"
    );
}

/// When a caller sends both, `file` is documented to win -- prove it rather
/// than trusting the doc comment alone.
#[test]
fn file_wins_when_both_file_and_path_are_given() {
    let Some(mut h) = Harness::new("patharg-conflict") else {
        return;
    };
    let decoy = h.scratch.join("decoy-does-not-exist.txt");
    let opened = h.call(
        1,
        "open",
        vec![("file", json!(h.file())), ("path", json!(decoy))],
    );
    assert!(!Harness::refused(&opened), "open refused: {opened}");
    let frames = Harness::frames(&opened);
    assert_eq!(
        frames[0]["payload"]["path"],
        json!(h.file()),
        "file must win over path when both are given: {frames:?}"
    );
}

/// B9: a refused edit must not cost a file its tab. Opened at verbosity 0 the
/// file's cached session has no token, so the replace first recovers one, and
/// that recovery used to delete the cache entry and leave it deleted when the
/// replace was then refused. With a second tab in the workspace the following
/// save could no longer find this file's tab and refused with "no tab".
#[test]
fn a_refused_edit_after_a_verbosity_0_open_keeps_the_tab_for_the_next_call_b9() {
    let Some(mut h) = Harness::new("b9") else {
        return;
    };
    let other = h.scratch.join("other.txt");
    std::fs::write(&other, "one\ntwo\n").unwrap();
    let opened_other = h.call(1, "open", vec![("file", json!(other))]);
    assert!(
        !Harness::refused(&opened_other),
        "open refused: {opened_other}"
    );
    let opened = h.call(
        2,
        "open",
        vec![("file", json!(h.file())), ("verbosity", json!(0))],
    );
    assert!(!Harness::refused(&opened), "open refused: {opened}");
    let replaced = h.call(
        3,
        "replace",
        vec![
            ("file", json!(h.file())),
            ("range_start_match", json!("no-such-text")),
            ("range_end_before_match", json!("beta")),
            ("text", json!("x")),
        ],
    );
    assert!(
        Harness::refused(&replaced),
        "a replace with no matching anchor applied: {replaced}"
    );
    assert_eq!(
        Harness::frames(&replaced)[0]["code"],
        json!("range_start_match_not_found"),
        "{replaced}"
    );
    let saved = h.call(4, "save", vec![("file", json!(h.file()))]);
    assert!(
        !Harness::refused(&saved),
        "the refused replace cost the file its tab: {saved}"
    );
}

/// B9's other half: create-parents lived only on the cold-start path, so an
/// `open` answered by an already-running workspace ignored the flag, created
/// nothing, and returned a tab whose save could only fail. A running workspace
/// must refuse an unacknowledged missing parent and create an acknowledged
/// one, exactly as a cold start does.
#[test]
fn a_running_workspace_honours_create_parents_on_open_b9() {
    let Some(mut h) = Harness::new("b9parents") else {
        return;
    };
    let warm = h.call(1, "open", vec![("file", json!(h.file()))]);
    assert!(!Harness::refused(&warm), "open refused: {warm}");
    let nested = h.scratch.join("new").join("deeper").join("c.txt");

    let refused = h.call(2, "open", vec![("file", json!(nested))]);
    assert!(
        Harness::refused(&refused),
        "a missing parent was opened without acknowledgement: {refused}"
    );
    assert!(
        Harness::content(&refused).contains("does not exist"),
        "the refusal must name the missing parent: {refused}"
    );
    assert!(
        !h.scratch.join("new").exists(),
        "a refused open created directories"
    );

    let opened = h.call(
        3,
        "open",
        vec![
            ("file", json!(nested)),
            ("acknowledge_create_parents", json!(true)),
        ],
    );
    assert!(!Harness::refused(&opened), "open refused: {opened}");
    assert!(
        nested.parent().unwrap().is_dir(),
        "an acknowledged open on a running workspace created nothing"
    );
    let inserted = h.call(
        4,
        "insert",
        vec![
            ("file", json!(nested)),
            ("offset", json!(0)),
            ("text", json!("hello\n")),
        ],
    );
    assert!(!Harness::refused(&inserted), "insert refused: {inserted}");
    let saved = h.call(5, "save", vec![("file", json!(nested))]);
    assert!(!Harness::refused(&saved), "save refused: {saved}");
    assert_eq!(std::fs::read_to_string(&nested).unwrap(), "hello\n");
}

/// B9's cold-start half: with two parent levels missing under a path that
/// runs through a symlink, the client keyed the endpoint record it waited for
/// on the raw path, while the server announced under the canonical one once
/// the directories existed, so the open reported "never announced an
/// endpoint" even though the server was up.
#[test]
fn a_cold_open_through_a_symlink_with_missing_parents_finds_its_server_b9() {
    let Some(mut h) = Harness::new("b9cold") else {
        return;
    };
    let real = h.scratch.join("real");
    std::fs::create_dir_all(&real).unwrap();
    let link = h.scratch.join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let nested = link.join("new").join("deeper").join("c.txt");
    let opened = h.call(
        1,
        "open",
        vec![
            ("file", json!(nested)),
            ("acknowledge_create_parents", json!(true)),
        ],
    );
    assert!(
        !Harness::refused(&opened),
        "a cold open with created parents did not find its server: {opened}"
    );
    assert!(real.join("new").join("deeper").is_dir());
}
