// MODE: DEV
//! The port transport end to end. On Windows this is not an alternative
//! transport — it is the fallback `open` autostart uses because no Unix
//! socket exists; the Windows CI job runs this file against real
//! loopback TCP, and every platform runs it under the dev shell, so the
//! transport Windows depends on is never only compile-checked.
//!
//! The manual `--tcp 127.0.0.1:0` starts here are what an autostarted
//! Windows server ends up running; the client drives them exactly the way
//! it drives its own autostart — announced endpoint file, registry
//! identity, session cache, and the HMAC challenge handshake.

use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};

struct TcpHarness {
    scratch: PathBuf,
    agent: String,
    server: Option<Child>,
}

impl TcpHarness {
    fn new(name: &str) -> Self {
        let scratch = std::env::temp_dir().join(format!(
            "ai-text-editor-tcp-{name}-{}-{}",
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
            agent: format!("tcp-test-{name}"),
            server: None,
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.scratch.join(name)
    }

    fn base_env(&self, command: &mut Command) {
        command
            .env("HOME", &self.scratch)
            .env("XDG_RUNTIME_DIR", self.scratch.join("runtime"))
            .env("TSCH_AI_EDITOR_METADATA_DIR", self.scratch.join("meta"))
            .env("TSCH_AI_EDITOR_SESSION_DIR", self.scratch.join("sessions"))
            .env("TSCH_AI_EDITOR_AGENT", &self.agent)
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CODEX_SESSION_ID")
            .env_remove("OPENCODE_PID");
    }

    /// Write the auth file the way `read_auth_token_file` demands (owner
    /// access only) and return its path plus the secret inside it.
    fn auth_file(&self, name: &str, secret: &str) -> PathBuf {
        let path = self.path(name);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        file.write_all(secret.as_bytes()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))
                .unwrap();
        }
        path
    }

    /// Start a server on an OS-assigned loopback port (exactly the
    /// `--tcp 127.0.0.1:0` shape a Windows autostart produces) and return
    /// the endpoint it announced, parsed from its own stdout so the test
    /// proves the announce is well-formed and race-free.
    fn start_tcp_server(&mut self, file: &std::path::Path, auth: &std::path::Path) -> String {
        let mut server = Command::new(env!("CARGO_BIN_EXE_ai-text-editor-server"));
        self.base_env(&mut server);
        server
            .arg("start")
            .arg("--file")
            .arg(file)
            .arg("--tcp")
            .arg("127.0.0.1:0")
            .arg("--auth-token-file")
            .arg(auth)
            .stdout(Stdio::piped());
        let mut server = server.spawn().expect("server binary must start");
        let stdout = server.stdout.take().expect("piped stdout");
        let announced = BufReader::new(stdout)
            .lines()
            .map_while(Result::ok)
            .find_map(|line| {
                serde_json::from_str::<Value>(&line)
                    .ok()
                    .and_then(|value| value.get("endpoint")?.as_str().map(str::to_owned))
            })
            .expect("the server announces its endpoint on stdout");
        self.server = Some(server);
        // The endpoint file must exist before a discovery-based client
        // call can find the port; the announce prints after writing it.
        assert!(announced.starts_with("tcp:127.0.0.1:"));
        announced
    }

    fn client(&self, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ai-text-editor"));
        self.base_env(&mut command);
        command.args(args);
        command.output().expect("client binary must run")
    }
}

impl Drop for TcpHarness {
    fn drop(&mut self) {
        if let Some(mut server) = self.server.take() {
            let _ = server.kill();
            let _ = server.wait();
        }
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

fn stdout_json(output: &Output) -> Vec<Value> {
    let text = String::from_utf8_lossy(&output.stdout);
    serde_json::Deserializer::from_str(&text)
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

#[test]
fn edits_and_saves_round_trip_over_an_explicit_loopback_port() {
    let mut harness = TcpHarness::new("roundtrip");
    let file = harness.path("port.txt");
    std::fs::write(&file, "alpha\nbeta\n").unwrap();
    let auth = harness.auth_file("auth", "port-test-secret");
    let endpoint = harness.start_tcp_server(&file, &auth);
    let session = harness.path("session.json");
    let opened = harness.client(&[
        "open",
        "--endpoint",
        &endpoint,
        "--auth-token",
        "port-test-secret",
        "--save-session-token",
        session.to_str().unwrap(),
        "-p",
        "structured",
    ]);
    assert!(
        opened.status.success(),
        "open over tcp: {}",
        stderr_text(&opened)
    );
    let payload = first_payload(&opened);
    assert!(
        payload["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("port.txt")),
        "open bound the requested file: {payload}"
    );
    let revision = payload["revision"].as_u64().unwrap().to_string();
    let edited = harness.client(&[
        "replace",
        "--endpoint",
        &endpoint,
        "--session-token",
        session.to_str().unwrap(),
        "--auth-token",
        "port-test-secret",
        "-o",
        "0",
        "-d",
        "5",
        "-t",
        "ALPHA",
        "-r",
        &revision,
        "-p",
        "structured",
    ]);
    assert!(
        edited.status.success(),
        "replace over tcp: {}",
        stderr_text(&edited)
    );
    let saved = harness.client(&[
        "save",
        "--endpoint",
        &endpoint,
        "--session-token",
        session.to_str().unwrap(),
        "--auth-token",
        "port-test-secret",
        "-r",
        "1",
    ]);
    assert!(
        saved.status.success(),
        "save over tcp: {}",
        stderr_text(&saved)
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "ALPHA\nbeta\n");
}

#[test]
fn a_wrong_token_on_the_port_is_refused_by_name_not_silently() {
    let mut harness = TcpHarness::new("wrongtoken");
    let file = harness.path("secret.txt");
    std::fs::write(&file, "keep out\n").unwrap();
    let auth = harness.auth_file("auth", "right-secret");
    let endpoint = harness.start_tcp_server(&file, &auth);
    let refused = harness.client(&[
        "open",
        "--endpoint",
        &endpoint,
        "--auth-token",
        "wrong-secret",
        "-p",
        "text",
    ]);
    assert!(!refused.status.success());
    assert!(
        stderr_text(&refused).contains("authentication_failed"),
        "the refusal must be named on stderr: {}",
        stderr_text(&refused)
    );
}

#[test]
fn a_port_endpoint_is_discoverable_with_no_endpoint_flag_at_all() {
    // The Windows shape of every later call: the client never learned the
    // port or the secret from a flag — it finds the announced endpoint
    // through the per-file discovery path and the registry record carries
    // the authentication token, exactly as autostart leaves them.
    let mut harness = TcpHarness::new("discovery");
    let file = harness.path("found.txt");
    std::fs::write(&file, "port found\n").unwrap();
    let auth = harness.auth_file("auth", "discovery-secret");
    harness.start_tcp_server(&file, &auth);
    let opened = harness.client(&["open", "-f", file.to_str().unwrap(), "-p", "structured"]);
    assert!(
        opened.status.success(),
        "file discovery onto the port: {}",
        stderr_text(&opened)
    );
    let payload = first_payload(&opened);
    assert_eq!(
        payload["path"],
        Value::String(file.to_string_lossy().into_owned())
    );
    let revision = payload["revision"].as_u64().unwrap().to_string();
    let edited = harness.client(&[
        "replace",
        "-f",
        file.to_str().unwrap(),
        "-o",
        "0",
        "-d",
        "4",
        "-t",
        "PORT",
        "-r",
        &revision,
        "-p",
        "structured",
    ]);
    assert!(
        edited.status.success(),
        "edit over discovered port: {}",
        stderr_text(&edited)
    );
    let read = harness.client(&["read", "-f", file.to_str().unwrap(), "-p", "text"]);
    assert_eq!(
        String::from_utf8_lossy(&read.stdout),
        "PORT found\n",
        "the buffer the port server holds"
    );
}

/// The fallback itself, asserted where only it can answer: a plain `open`
/// with no endpoint flag on a platform with no Unix sockets. The server
/// must come up on a loopback port on its own and carry the whole flow.
#[cfg(windows)]
#[test]
fn open_autostarts_onto_a_loopback_port_when_unix_sockets_are_unavailable() {
    let mut harness = TcpHarness::new("fallback");
    // The discovery tree is shared with the other tests' naming; the
    // fallback test must observe what autostart actually announced.
    let file = harness.path("fallback.txt");
    std::fs::write(&file, "first save\n").unwrap();
    let opened = harness.client(&["open", "-f", file.to_str().unwrap(), "-p", "structured"]);
    assert!(
        opened.status.success(),
        "open must fall back to a port: {}",
        stderr_text(&opened)
    );
    let discovery = ai_text_editor::transport::endpoint_for_file(&file);
    let announced = std::fs::read_to_string(&discovery).expect("endpoint was announced");
    assert!(
        announced.contains("\"tcp:127.0.0.1:"),
        "the fallback must announce a loopback port, not a socket: {announced}"
    );
    let payload = first_payload(&opened);
    let pid = payload["server_pid"].as_u64().unwrap();
    let revision = payload["revision"].as_u64().unwrap().to_string();
    let edited = harness.client(&[
        "insert",
        "-f",
        file.to_str().unwrap(),
        "-o",
        "0",
        "-t",
        "edited ",
        "-r",
        &revision,
        "-p",
        "structured",
    ]);
    assert!(
        edited.status.success(),
        "insert over the fallback port: {}",
        stderr_text(&edited)
    );
    let saved = harness.client(&["save", "-f", file.to_str().unwrap(), "-r", "1"]);
    assert!(
        saved.status.success(),
        "save over the fallback port: {}",
        stderr_text(&saved)
    );
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "edited first save\n"
    );
    let _ = Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .stdout(Stdio::null())
        .status();
}
