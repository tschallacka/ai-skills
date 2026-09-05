// MODE: DEV
// End-to-end regressions for the failure modes found in the first real
// agent test-drive of the skill: silent refusals, a wedged session after
// an external change, edits routed to the wrong tab, stale endpoints that
// nothing could walk past, and the impossibility of creating a file.
// Each test drives the same two binaries an agent drives, in a private
// scratch tree, and asserts on what the *agent* would see: exit status,
// stdout, and stderr.
//
// Cross-platform on purpose: on Unix these flows ride the socket
// transport; on Windows the same `open` autostarts onto the loopback TCP
// fallback, so every assertion below doubles as the fallback's test.
// tcp_flow.rs pins the port transport explicitly on top of that.

use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Kill a pid the way this platform hard-stops a process, so the
/// killed-server tests can assert on the corpse of a real process
/// everywhere.
fn terminate(pid: u32) {
    #[cfg(unix)]
    let mut kill = {
        let mut c = Command::new("kill");
        c.args(["-9", &pid.to_string()]);
        c
    };
    #[cfg(not(unix))]
    let mut kill = {
        let mut c = Command::new("taskkill");
        c.args(["/F", "/PID", &pid.to_string()]);
        c
    };
    let _ = kill.stdout(std::process::Stdio::null()).status();
}

struct Harness {
    scratch: PathBuf,
    agent: String,
}

impl Harness {
    fn new(name: &str) -> Self {
        let root = std::env::var_os("TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let scratch = root.join(format!(
            "ai-text-editor-flow-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_micros()
        ));
        std::fs::create_dir_all(scratch.join("runtime")).unwrap();
        std::fs::create_dir_all(scratch.join("sessions")).unwrap();
        Self {
            scratch,
            // A literal identity keeps each test's workspace isolated from
            // any harness session this test process itself runs under.
            agent: format!("flow-test-{name}"),
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.scratch.join(name)
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.path(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    fn client(&self, args: &[&str]) -> Output {
        self.client_env(args, &[])
    }

    fn client_env(&self, args: &[&str], extra_env: &[(&str, &str)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ai-text-editor"));
        command
            .env("HOME", &self.scratch)
            .env("XDG_RUNTIME_DIR", self.scratch.join("runtime"))
            .env("TSCH_AI_EDITOR_METADATA_DIR", self.scratch.join("meta"))
            .env("TSCH_AI_EDITOR_SESSION_DIR", self.scratch.join("sessions"))
            .env("TSCH_AI_EDITOR_AGENT", &self.agent)
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CODEX_SESSION_ID")
            .env_remove("OPENCODE_PID")
            .args(args);
        for (key, value) in extra_env {
            command.env(key, value);
        }
        command.output().expect("client binary must run")
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        // Autostarted servers outlive their short-lived client; stop the
        // ones this test left behind before removing the tree they run in.
        if let Ok(entries) = std::fs::read_dir(self.scratch.join("runtime")) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.ends_with(".endpoint") {
                    continue;
                }
                let Ok(meta) = std::fs::metadata(entry.path()) else {
                    continue;
                };
                if meta.len() > 4096 {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(entry.path()) else {
                    continue;
                };
                if let Ok(value) = serde_json::from_str::<Value>(&content) {
                    if let Some(pid) = value.get("pid").and_then(Value::as_u64) {
                        terminate(pid as u32);
                    }
                }
            }
        }
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

fn stdout_json(output: &Output) -> Vec<Value> {
    let text = String::from_utf8_lossy(&output.stdout);
    let decoder = serde_json::Deserializer::from_str(&text);
    decoder
        .into_iter::<Value>()
        .filter_map(Result::ok)
        .collect()
}

fn first_payload(output: &Output) -> Value {
    stdout_json(output)
        .into_iter()
        .find(|frame| frame.get("type").and_then(Value::as_str) == Some("data"))
        .and_then(|frame| frame.get("payload").cloned())
        .expect("a data frame")
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn revision_of(open_output: &Output) -> u64 {
    first_payload(open_output)
        .get("revision")
        .and_then(Value::as_u64)
        .unwrap()
}

fn server_pid(open_output: &Output) -> u32 {
    first_payload(open_output)
        .get("server_pid")
        .and_then(Value::as_u64)
        .unwrap() as u32
}

#[test]
fn new_file_opens_edits_and_is_created_by_save() {
    let harness = Harness::new("newfile");
    let target = harness.path("created.txt");
    assert!(!target.exists());
    let opened = harness.client(&["open", "-f", target.to_str().unwrap(), "-p", "structured"]);
    assert!(opened.status.success(), "{}", stderr_text(&opened));
    assert_eq!(first_payload(&opened)["dirty"], json!(false));
    let inserted = harness.client(&[
        "insert",
        "-f",
        target.to_str().unwrap(),
        "-o",
        "0",
        "-t",
        "hello from a new tab",
        "-r",
        "0",
        "-p",
        "structured",
    ]);
    assert!(inserted.status.success(), "{}", stderr_text(&inserted));
    assert_eq!(first_payload(&inserted)["dirty"], json!(true));
    assert!(!target.exists(), "an edit alone must not touch the disk");
    let saved = harness.client(&[
        "save",
        "-f",
        target.to_str().unwrap(),
        "-r",
        "1",
        "-p",
        "structured",
    ]);
    assert!(saved.status.success(), "{}", stderr_text(&saved));
    assert_eq!(first_payload(&saved)["saved"], json!(true));
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "hello from a new tab"
    );
    let reopened = harness.client(&["open", "-f", target.to_str().unwrap(), "-p", "structured"]);
    assert_eq!(first_payload(&reopened)["dirty"], json!(false));
}

#[test]
fn stale_revision_is_named_on_stderr_under_text_presentation() {
    let harness = Harness::new("stale");
    let file = harness.write("stale.txt", "alpha\nbeta\n");
    harness.client(&["open", "-f", file.to_str().unwrap()]);
    let refused = harness.client(&[
        "replace",
        "-f",
        file.to_str().unwrap(),
        "-o",
        "0",
        "-d",
        "5",
        "-t",
        "X",
        "-r",
        "999",
        "-p",
        "text",
    ]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(
        refused.stdout.is_empty(),
        "a refused edit must not print a payload"
    );
    let error = stderr_text(&refused);
    assert!(error.contains("stale_revision"), "stderr named: {error}");
    assert!(error.contains("expected 999"), "stderr explained: {error}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "alpha\nbeta\n");
}

#[test]
fn external_change_blocks_writes_loudly_without_blinding_reads() {
    let harness = Harness::new("external");
    let file = harness.write("ext.txt", "alpha\nbeta\n");
    let opened = harness.client(&["open", "-f", file.to_str().unwrap(), "-p", "structured"]);
    let revision = revision_of(&opened).to_string();
    let mut appended = std::fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap();
    appended.write_all(b"EXTERN\n").unwrap();
    drop(appended);
    let read = harness.client(&["read", "-f", file.to_str().unwrap(), "-p", "text"]);
    assert!(read.status.success(), "{}", stderr_text(&read));
    assert_eq!(String::from_utf8_lossy(&read.stdout), "alpha\nbeta\n");
    let refused = harness.client(&[
        "replace",
        "-f",
        file.to_str().unwrap(),
        "-o",
        "0",
        "-d",
        "1",
        "-t",
        "Y",
        "-r",
        &revision,
        "-p",
        "text",
    ]);
    assert_eq!(refused.status.code(), Some(1));
    let error = stderr_text(&refused);
    assert!(error.contains("external_change"), "stderr named: {error}");
    assert!(error.contains("reload"), "stderr offered choices: {error}");
    let reloaded = harness.client(&["resolve", "-f", file.to_str().unwrap(), "-a", "reload"]);
    assert!(reloaded.status.success(), "{}", stderr_text(&reloaded));
    let read = harness.client(&["read", "-f", file.to_str().unwrap(), "-p", "text"]);
    assert_eq!(
        String::from_utf8_lossy(&read.stdout),
        "alpha\nbeta\nEXTERN\n"
    );
}

#[test]
fn a_request_naming_another_file_cannot_edit_the_routed_tab() {
    let harness = Harness::new("mismatch");
    let first = harness.write("first.txt", "one\n");
    let second = harness.write("second.txt", "two\n");
    harness.client(&["open", "-f", first.to_str().unwrap()]);
    // No tab exists for `second` under this identity; the registry's newest
    // tab belongs to `first` and answered earlier calls, so its token would
    // route here. It must be refused, not applied to the wrong buffer.
    let refused = harness.client(&[
        "replace",
        "-f",
        second.to_str().unwrap(),
        "-o",
        "0",
        "-d",
        "3",
        "-t",
        "X",
        "-r",
        "1",
        "-p",
        "text",
    ]);
    assert!(!refused.status.success());
    let error = stderr_text(&refused);
    assert!(
        error.contains("file_mismatch") || error.contains("no server discovered"),
        "refusal must name the mismatch or point at open: {error}"
    );
    assert_eq!(std::fs::read_to_string(&first).unwrap(), "one\n");
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "two\n");
    // Opening the named file routes correctly and the same edit then lands.
    let opened = harness.client(&["open", "-f", second.to_str().unwrap(), "-p", "structured"]);
    let revision = revision_of(&opened).to_string();
    let applied = harness.client(&[
        "replace",
        "-f",
        second.to_str().unwrap(),
        "-o",
        "0",
        "-d",
        "3",
        "-t",
        "X",
        "-r",
        &revision,
        "-p",
        "structured",
    ]);
    assert!(applied.status.success(), "{}", stderr_text(&applied));
    harness.client(&["save", "-f", second.to_str().unwrap(), "-r", "1"]);
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "X\n");
}

#[test]
fn killed_server_is_replaced_by_the_next_open_and_the_journal_replays() {
    let harness = Harness::new("killed");
    let file = harness.write("journal.txt", "alpha\nbeta\n");
    let opened = harness.client(&["open", "-f", file.to_str().unwrap(), "-p", "structured"]);
    let pid = server_pid(&opened);
    let revision = revision_of(&opened).to_string();
    let edited = harness.client(&[
        "replace",
        "-f",
        file.to_str().unwrap(),
        "-o",
        "0",
        "-d",
        "5",
        "-t",
        "BETA",
        "-r",
        &revision,
        "-p",
        "structured",
    ]);
    assert!(edited.status.success(), "{}", stderr_text(&edited));
    assert_eq!(first_payload(&edited)["dirty"], json!(true));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "alpha\nbeta\n");
    terminate(pid);
    std::thread::sleep(std::time::Duration::from_millis(200));
    // A plain command must explain that its server is gone, not die on a
    // bare connection error.
    let blocked = harness.client(&["read", "-f", file.to_str().unwrap(), "-p", "text"]);
    assert!(!blocked.status.success());
    assert!(
        stderr_text(&blocked).contains("has stopped"),
        "stderr must point at open: {}",
        stderr_text(&blocked)
    );
    // `open` reclaims the dead endpoint, starts a replacement, and replays
    // the unsaved edit into the buffer.
    let reopened = harness.client(&["open", "-f", file.to_str().unwrap(), "-p", "structured"]);
    assert!(reopened.status.success(), "{}", stderr_text(&reopened));
    let payload = first_payload(&reopened);
    assert!(
        server_pid(&reopened) != pid,
        "a replacement server answered"
    );
    assert_eq!(payload["revision"], json!(1));
    assert_eq!(payload["dirty"], json!(true));
    let read = harness.client(&["read", "-f", file.to_str().unwrap(), "-p", "text"]);
    assert_eq!(String::from_utf8_lossy(&read.stdout), "BETA\nbeta\n");
    let saved = harness.client(&["save", "-f", file.to_str().unwrap(), "-r", "1"]);
    assert!(saved.status.success(), "{}", stderr_text(&saved));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "BETA\nbeta\n");
}

#[test]
fn text_reads_honor_a_byte_window_and_deletes_report_line_spans() {
    let harness = Harness::new("window");
    let file = harness.write("window.txt", "0123456789\nabcdefgh\n");
    harness.client(&["open", "-f", file.to_str().unwrap()]);
    let window = harness.client(&[
        "read",
        "-f",
        file.to_str().unwrap(),
        "-p",
        "structured",
        "-o",
        "2",
        "-L",
        "5",
    ]);
    let payload = first_payload(&window);
    assert_eq!(payload["text"], json!("23456"));
    assert_eq!(payload["offset"], json!(2));
    assert_eq!(payload["total_bytes"], json!(20));
    assert_eq!(payload["eof"], json!(false));
    // Deleting across the line end must say so.
    let revision = payload["revision"].to_string();
    let joined = harness.client(&[
        "replace",
        "-f",
        file.to_str().unwrap(),
        "-o",
        "8",
        "-d",
        "4",
        "-t",
        "_",
        "-r",
        &revision,
        "-p",
        "structured",
    ]);
    let payload = first_payload(&joined);
    assert_eq!(payload["spans_lines"], json!(true));
    assert_eq!(payload["dirty"], json!(true));
}
