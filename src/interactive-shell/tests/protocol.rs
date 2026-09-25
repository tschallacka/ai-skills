// MODE: DEV
#![cfg(unix)]
use interactive_shell_core::ClientStream;
use serde_json::Value;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::Duration;

/// How many 10ms polls a readiness wait is allowed: 3000, so thirty seconds.
///
/// These loops were `0..100`, a ONE second budget, which is ample on a Linux
/// runner and far too tight on the macOS one. The macOS runner is a shared,
/// oversubscribed VPS that pauses for other tenants, so a readiness budget
/// has to cover the worst scheduling delay rather than the typical one.
///
/// It is a CEILING, not a sleep: every loop returns the moment its condition
/// holds, so a healthy run is no slower than it was. Only a genuine failure
/// pays the thirty seconds, and it pays it once.
const READY_POLLS: usize = 3000;

/// The gap between polls. Kept small so readiness is detected promptly; the
/// budget above is what bounds the wait.
const POLL_INTERVAL: Duration = Duration::from_millis(10);

// WHY EVERY FIXTURE PARKS FOR 600 SECONDS, AND EVERY IDLE TIMEOUT IS 600.
//
// The wrapper exits when its child does, and removes the socket on the way
// out. So a fixture's lifetime is not a detail of the fixture: it is the
// window in which the whole test must finish, and a fixture that dies first
// makes the failure read as a missing socket rather than an expired child.
//
// These were 30 seconds, which is generous on a workstation and a bet on the
// macOS runner. A shared macOS CI runner needed a longer fixture value than
// that original 30s: a fixture can outlive the socket-readiness wait on a
// slow enough leg, and the failure then reads as a missing socket rather
// than an expired child.
//
// 600 costs a healthy run nothing: every test that needs its wrapper gone
// either kills it or asserts against its exit, so nothing waits out the
// ceiling. Two deliberate exceptions, neither of which is an oversight:
//
//   * `wrapper_reports_screen_ack_and_lifecycle` keeps `sleep 1`, because the
//     lifecycle event it asserts is produced BY the child exiting.
//   * `idle_timeout_reports_status_124` keeps an idle timeout of 1, because
//     that deadline is its subject -- and its child now parks for 600 so a
//     starved wrapper cannot report the child's exit in place of the timeout.

/// Where a wrapper's stderr is kept, inside the test's own directory.
///
/// It was `Stdio::null()`. That is why a wrapper that failed before it could
/// bind produced only a "socket did not appear" panic and not one line saying
/// what went wrong -- the diagnosis had to be guessed at from which tests
/// failed. A FILE and not a pipe, because the wrapper
/// outlives the assertion and a pipe nobody drains blocks it once the buffer
/// fills; opened for append so several wrappers in one directory accumulate
/// rather than truncating each other.
fn stderr_path(dir: &Path) -> PathBuf {
    dir.join("wrapper.stderr")
}

/// Whatever the wrapper said, ready to append to a panic message.
fn wrapper_stderr(dir: &Path) -> String {
    match fs::read(stderr_path(dir)) {
        Ok(bytes) if !bytes.is_empty() => {
            format!(
                "wrapper stderr:\n{}",
                String::from_utf8_lossy(&bytes).trim_end()
            )
        }
        _ => "wrapper stderr: (empty)".to_string(),
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("interactive-shell-{label}-{}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

fn start(dir: &Path, command: &[&str], idle: &str) -> Child {
    start_binary(env!("CARGO_BIN_EXE_interactive-shell"), dir, command, idle)
}

fn start_binary(binary: &str, dir: &Path, command: &[&str], idle: &str) -> Child {
    Command::new(binary)
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "--cols",
            "20",
            "--rows",
            "4",
            "--idle-timeout",
            idle,
            "--",
        ])
        .args(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(stderr_path(dir))
                .unwrap(),
        ))
        .spawn()
        .unwrap()
}

/// Like `start`, but with `--tcp`: the discovery file at `dir/socket` ends up
/// holding a port+nonce instead of being a real Unix socket special file, and
/// `connect_socket`/`request`/`request_all` need no changes to reach it --
/// `connect_in_directory` (lib.rs) auto-detects which transport is actually
/// there.
fn start_tcp(dir: &Path, command: &[&str], idle: &str) -> Child {
    Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "--tcp",
            "--cols",
            "20",
            "--rows",
            "4",
            "--idle-timeout",
            idle,
            "--",
        ])
        .args(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(stderr_path(dir))
                .unwrap(),
        ))
        .spawn()
        .unwrap()
}

fn start_fixture(dir: &Path, idle: &str) -> Child {
    start_binary(
        env!("CARGO_BIN_EXE_interactive-shell"),
        dir,
        &[env!("CARGO_BIN_EXE_interactive-shell-fixture")],
        idle,
    )
}

/// Block until the wrapper's socket exists, or the budget runs out.
///
/// The CLI paths need this and `request()` does not: request() polls connect in
/// its own loop, so it waits by construction, while a `Command` invocation gets
/// exactly one attempt. `cli_text_preserves_spaces_and_input_help_is_available`
/// sent text with no wait at all and passed on Linux for as long as the wrapper
/// won that race; on the macOS runner it lost it and failed with ENOENT.
fn wait_for_socket(dir: &Path) {
    for _ in 0..READY_POLLS {
        if dir.join("socket").exists() {
            return;
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!(
        "socket did not appear at {} within {:?}\n{}",
        dir.join("socket").display(),
        POLL_INTERVAL * READY_POLLS as u32,
        wrapper_stderr(dir)
    )
}

/// Poll `observe` until the screen carries `needle`, and return that snapshot.
///
/// An ack means the WRAPPER accepted the input, never that the program acted
/// on it, so a state change is confirmed against the next screen. Text typed
/// at a `sleep` appears only
/// because the tty line discipline echoes it, which means it has to travel
/// input -> pty master -> echo -> the wrapper's read loop -> the screen model
/// before any observe can see it. A single observe straight after the ack is
/// sampling that journey once and hoping.
///
/// On Linux the echo lands within microseconds and the one-shot assertion
/// passed for as long as the test existed. On the macOS runner it did not.
fn wait_for_rows(dir: &Path, needle: &str) -> Value {
    let mut last = Value::Null;
    for _ in 0..READY_POLLS {
        if let Some(snapshot) = request_all(dir, "{\"v\":1,\"op\":\"observe\"}\n")
            .into_iter()
            .find(|event| event["event"] == "snapshot")
        {
            if snapshot["rows"].to_string().contains(needle) {
                return snapshot;
            }
            last = snapshot;
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!(
        "{:?} never reached the screen within {:?}\nlast snapshot rows: {}\n{}",
        needle,
        POLL_INTERVAL * READY_POLLS as u32,
        last["rows"],
        wrapper_stderr(dir)
    )
}

/// Poll `observe` until `needle` is NO LONGER on the screen.
///
/// The counterpart of `wait_for_rows`, for a test that asserts something was
/// removed. Waiting for the condition rather than sleeping a guessed interval
/// is the same doctrine: a fixed `thread::sleep` is a bet on how fast the
/// runner is, and the macOS runner loses that bet.
fn wait_until_gone(dir: &Path, needle: &str) {
    for _ in 0..READY_POLLS {
        if let Some(snapshot) = request_all(dir, "{\"v\":1,\"op\":\"observe\"}\n")
            .into_iter()
            .find(|event| event["event"] == "snapshot")
        {
            if !snapshot["rows"].to_string().contains(needle) {
                return;
            }
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!(
        "{:?} was still on the screen after {:?}\n{}",
        needle,
        POLL_INTERVAL * READY_POLLS as u32,
        wrapper_stderr(dir)
    )
}

/// Send `body` and read the whole reply, saying what went wrong if it cannot.
///
/// These were four bare `unwrap()`s. A wrapper that exits mid-request -- an
/// idle timeout, a fixture that ended -- surfaced as
/// `Os { code: 104, kind: ConnectionReset }` from `read_to_string`, with no
/// indication of which request, which socket, or what the wrapper had said on
/// its way out. `code: 104` is also Linux-only; macOS numbers ECONNRESET 54,
/// so the number in a CI log is not even stable across the legs.
fn exchange(dir: &Path, stream: &mut ClientStream, body: &str) -> String {
    stream.write_all(body.as_bytes()).unwrap_or_else(|error| {
        panic!("sending {body:?} failed: {error}\n{}", wrapper_stderr(dir))
    });
    stream
        .shutdown(std::net::Shutdown::Write)
        .unwrap_or_else(|error| {
            panic!(
                "half-closing after {body:?} failed: {error}\n{}",
                wrapper_stderr(dir)
            )
        });
    let mut out = String::new();
    stream.read_to_string(&mut out).unwrap_or_else(|error| {
        panic!(
            "reading the reply to {body:?} failed: {error}\npartial reply: {out:?}\n{}",
            wrapper_stderr(dir)
        )
    });
    out
}

/// Connect to the wrapper's socket the way the library does: by name, from
/// inside its own directory.
///
/// NOT `UnixStream::connect(dir.join("socket"))`. That is an absolute address,
/// and sun_path caps a Unix socket address at 104 bytes. On macOS $TMPDIR is a
/// per-user, deeply nested path, and the scratch directory each test creates
/// pushes the address past that cap, so every connect fails with "path must
/// be shorter than SUN_LEN". The library already answers this with
/// `connect_in_directory`, whose other half is `bind_in_directory`: both ends
/// have to be relative or the shorter one just moves the failure. The tests
/// are the third end of that rule and were still connecting absolutely.
///
/// WHY THE LOCK. `connect_in_directory` gets its relative address by moving the
/// process into the directory with `fchdir`, and the cwd is per PROCESS, not per
/// thread. The wrapper is single-threaded so it pays nothing for that; this
/// binary runs its tests as threads in one process, so an unguarded move lets
/// one test's connect run while another test's cwd is in force. The same
/// hazard is why `the_socket_is_bound_by_name_inside_the_held_directory` and its
/// sibling are serialised in the library's own unit tests.
///
/// The lock covers the move and nothing else -- `exchange` reads and writes
/// after it is released -- so the suite still finishes quickly.
/// A poisoned lock is recovered rather than propagated: the poison would come
/// from some other test's panic, and propagating it would hide the failure
/// that actually matters.
static CWD_LOCK: Mutex<()> = Mutex::new(());

fn connect_socket(dir: &Path) -> Result<ClientStream, String> {
    let _guard = CWD_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    interactive_shell_core::connect_in_directory(&dir.join("socket"))
}

/// What to say when no connect ever succeeded.
///
/// The old message was "socket did not appear", which named the one thing that
/// was not wrong: the socket was there throughout and `exists()` said so. A
/// harness that misattributes its own failure sends the next reader after the
/// wrapper, and this message sent three sessions there. So it prints the last
/// connect error, which identifies the cause, and whether the socket exists,
/// which is what separates "never bound" from "bound but unreachable".
fn unreachable_socket(dir: &Path, last_error: &str) -> String {
    format!(
        "could not connect to {} within {:?}\nsocket present: {}\nlast connect error: {}\n{}",
        dir.join("socket").display(),
        POLL_INTERVAL * READY_POLLS as u32,
        dir.join("socket").exists(),
        if last_error.is_empty() {
            "(never attempted)"
        } else {
            last_error
        },
        wrapper_stderr(dir)
    )
}

fn request(dir: &Path, body: &str) -> Value {
    let mut last_error = String::new();
    for _ in 0..READY_POLLS {
        match connect_socket(dir) {
            Ok(mut stream) => {
                let out = exchange(dir, &mut stream, body);
                return serde_json::from_str(out.trim()).unwrap_or_else(|error| {
                    panic!(
                        "reply to {body:?} is not JSON: {error}\nreply: {out:?}\n{}",
                        wrapper_stderr(dir)
                    )
                });
            }
            Err(error) => last_error = error,
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("{}", unreachable_socket(dir, &last_error))
}

fn request_all(dir: &Path, body: &str) -> Vec<Value> {
    let mut last_error = String::new();
    for _ in 0..READY_POLLS {
        match connect_socket(dir) {
            Ok(mut stream) => {
                let out = exchange(dir, &mut stream, body);
                return out
                    .lines()
                    .map(|line| {
                        serde_json::from_str(line).unwrap_or_else(|error| {
                            panic!(
                                "reply line is not JSON: {error}\nline: {line:?}\nreply: {out:?}"
                            )
                        })
                    })
                    .collect();
            }
            Err(error) => last_error = error,
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("{}", unreachable_socket(dir, &last_error))
}

#[test]
fn wrapper_reports_screen_ack_and_lifecycle() {
    let dir = temp_dir("events");
    let mut child = start(
        &dir,
        &["sh", "-c", "stty size; printf first; sleep 1"],
        "600",
    );
    let ack = request(
        &dir,
        r#"{"v":1,"op":"key","key":"ENTER"}
"#,
    );
    assert_eq!(ack["event"], "ack");
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(status.success());
    assert!(output
        .lines()
        .any(|line| line.contains("\"event\":\"screen\"")));
    assert!(output
        .lines()
        .any(|line| line.contains("\"event\":\"lifecycle\"")));
    assert!(!dir.join("socket").exists());
}

#[test]
fn screen_deltas_chain_and_publish_restored_primary_rows() {
    let dir = temp_dir("alternate-event");
    let mut child = start(
        &dir,
        &[
            "sh",
            "-c",
            "printf primary; sleep .1; printf '\\x1b[?1049hALT\\x1b[?1049l'; sleep .2",
        ],
        "600",
    );
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    child.wait().unwrap();
    let events: Vec<Value> = output
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let screens: Vec<&Value> = events.iter().filter(|v| v["event"] == "screen").collect();
    assert!(
        !screens.is_empty(),
        "no screen events\n{}",
        wrapper_stderr(&dir)
    );
    assert_eq!(screens[0]["seq"], 1);
    assert_eq!(screens[0]["base"], 0);
    for pair in screens.windows(2) {
        assert_eq!(pair[1]["base"].as_u64(), pair[0]["seq"].as_u64());
    }
    assert!(screens
        .iter()
        .any(|v| v["rows"].to_string().contains("primary")));
    assert!(screens.last().unwrap()["rows"]
        .to_string()
        .contains("primary"));
}

#[test]
fn child_exit_drains_buffered_output_past_one_read_chunk() {
    let dir = temp_dir("drain-on-exit");
    let script = "yes A | head -c 20000; printf DRAINEDMARK9F3XQ";
    let mut child = start(&dir, &["sh", "-c", script], "600");
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(status.success(), "{}", wrapper_stderr(&dir));
    assert!(
        output.contains("DRAINEDMARK9F3XQ"),
        "the tail marker written just before the child exited never reached a screen event -- \
         the pty's final buffered burst was dropped when child_exit fired\n{}",
        wrapper_stderr(&dir)
    );
}

#[test]
fn protocol_observes_fragmented_osc_overflow_without_leaking_payload() {
    let dir = temp_dir("fragmented-osc");
    let mut child = start_fixture(&dir, "600");
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    child.wait().unwrap();
    let rows = output
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|event| event["event"] == "screen")
        .flat_map(|event| {
            event["rows"]
                .as_object()
                .map(|rows| {
                    rows.values()
                        .filter_map(|row| row.as_str().map(str::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        rows.contains("CSI_SAFE"),
        "CSI_SAFE never reached the screen\n{}",
        wrapper_stderr(&dir)
    );
    assert!(rows.contains("OSC_SAFE"));
    assert!(!rows.contains(&"1".repeat(129)));
    assert!(!rows.contains(&"x".repeat(4097)));
}

#[test]
fn socket_is_private_and_invalid_requests_are_rejected() {
    let dir = temp_dir("bounds");
    let mut child = start(&dir, &["sleep", "600"], "600");
    for _ in 0..READY_POLLS {
        if dir.join("socket").exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    let mode = fs::metadata(dir.join("socket"))
        .unwrap_or_else(|e| panic!("no socket to inspect: {e}\n{}", wrapper_stderr(&dir)))
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
    let error = request(
        &dir,
        r#"{"v":1,"op":"raw","hex":"0"}
"#,
    );
    assert_eq!(error["event"], "error");
    let malformed = request(&dir, "{\"v\":1,\"op\":\"text\"}\n");
    assert_eq!(malformed["event"], "error");
    let combination = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "key",
            "x",
            "shutdown",
        ])
        .output()
        .unwrap();
    assert!(!combination.status.success());
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
    assert!(!dir.join("socket").exists());
}

#[test]
fn closed_output_removes_socket_before_exit() {
    let dir = temp_dir("closed-output");
    let mut child = start(&dir, &["yes"], "600");
    for _ in 0..READY_POLLS {
        if dir.join("socket").exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    assert!(
        dir.join("socket").exists(),
        "the socket was never created\n{}",
        wrapper_stderr(&dir)
    );
    drop(child.stdout.take());
    let status = child.wait().unwrap();
    assert!(!status.success());
    assert!(!dir.join("socket").exists());
}

#[test]
fn invalid_dimensions_fail_before_creating_socket() {
    let dir = temp_dir("dimensions");
    let output = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "--cols",
            "0",
            "--rows",
            "4",
            "--",
            "true",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!dir.join("socket").exists());
    let upper = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .args([
            "--socket",
            dir.join("upper").to_str().unwrap(),
            "--cols",
            "240",
            "--rows",
            "100",
            "--idle-timeout",
            "1",
            "--",
            "true",
        ])
        .output()
        .unwrap();
    assert!(
        upper.status.success(),
        "the upper-bound wrapper failed: {}",
        String::from_utf8_lossy(&upper.stderr).trim_end()
    );
    for (option, value) in [("--rows", "0"), ("--cols", "241"), ("--rows", "101")] {
        let output = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
            .args([
                "--socket",
                dir.join(format!("{option}-{value}")).to_str().unwrap(),
                option,
                value,
                "--",
                "true",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
    }
}

#[test]
fn structured_observe_paste_mouse_and_resize_requests_work() {
    let dir = temp_dir("structured-actions");
    let mut child = start(&dir, &["sh", "-c", "sleep 600"], "600");
    let observed = request_all(
        &dir,
        r#"{"v":1,"op":"observe"}
"#,
    );
    assert_eq!(observed[0]["event"], "snapshot");
    assert_eq!(observed[1]["event"], "ack");
    assert_eq!(
        request(
            &dir,
            r#"{"v":1,"op":"paste","text":"pasted"}
"#
        )["event"],
        "ack"
    );
    assert_eq!(
        request(
            &dir,
            r#"{"v":1,"op":"mouse","x":2,"y":2,"button":0,"action":"down"}
"#
        )["event"],
        "ack"
    );
    assert_eq!(
        request(
            &dir,
            r#"{"v":1,"op":"resize","cols":30,"rows":6}
"#
        )["event"],
        "ack"
    );
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn view_is_compact_numbered_and_supports_rows_and_deltas() {
    let dir = temp_dir("view");
    let mut child = start(
        &dir,
        // Driven by input, NOT by a timer. A timer-based rewrite races this
        // test's own assertions, and a slow-enough runner can see row 1
        // already moved on where a fast one still sees it fresh -- which
        // reads as "row 2 is missing" when it is really "row 1 changed
        // first". `read` makes the screen change exactly when this test
        // sends a line and never before; `stty -echo` keeps that line off
        // the screen, and the trailing `read` parks the shell until
        // shutdown so no second timer can end it early.
        &[
            "sh",
            "-c",
            "stty -echo; printf ONE; read _go; printf '\\rTWO'; read _park",
        ],
        "600",
    );
    let first_wait = request_all(
        &dir,
        r#"{"v":1,"op":"wait","contains":"ONE"}
"#,
    );
    assert!(
        first_wait.iter().any(|event| event["matched"] == true),
        "no wait event matched ONE: {first_wait:?}"
    );
    let view = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "view",
            "1",
        ])
        .output()
        .unwrap();
    assert!(view.status.success());
    assert_eq!(
        String::from_utf8(view.stdout).unwrap(),
        "terminal=20x4\n001 [001-003] ONE\n"
    );
    let range = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "view",
            "1-2",
        ])
        .output()
        .unwrap();
    assert!(range.status.success());
    // Carries the actual output: this assertion failed on x86_64-apple-darwin
    // while the single-row view above passed, and a bare starts_with reports
    // only that it failed -- nobody can tell whether row 2 was absent,
    // differently numbered, or merely not written yet.
    let range_out = String::from_utf8(range.stdout).unwrap();
    let range_want = "terminal=20x4\n001 [001-003] ONE\n002 ";
    assert!(
        range_out.starts_with(range_want),
        "view 1-2 did not start with the expected two rows\n  want prefix: {:?}\n  got:         {:?}",
        range_want,
        range_out
    );
    // Release the fixture's first `read`, which is what turns ONE into TWO.
    let ack = request(&dir, "{\"v\":1,\"op\":\"text\",\"text\":\"\\n\"}\n");
    assert_eq!(ack["event"], "ack", "text op was not acknowledged: {ack:?}");
    let second_wait = request_all(
        &dir,
        r#"{"v":1,"op":"wait","contains":"TWO"}
"#,
    );
    assert!(
        second_wait.iter().any(|event| event["matched"] == true),
        "no wait event matched TWO: {second_wait:?}"
    );
    let delta = request_all(
        &dir,
        r#"{"v":1,"op":"view-delta","rows":[1]}
"#,
    );
    assert_eq!(delta[0]["event"], "view");
    assert_eq!(delta[0]["text"], "001 [001-003] TWO");
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn rgbview_preserves_styles_without_json_wrapping() {
    let dir = temp_dir("rgbview");
    let mut child = start(
        &dir,
        &["sh", "-c", "printf '\\033[31mRED\\033[0m'; sleep 600"],
        "600",
    );
    let _ = request_all(
        &dir,
        r#"{"v":1,"op":"wait","contains":"RED"}
"#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "rgbview",
            "1",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\x1b[0;31mRED"));
    assert!(!text.contains("\"event\""));
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

// B142: the markup mode over the real socket, not just the unit-tested
// Screen::markup -- proves the CLI wiring (dispatch, row filtering, the
// "markup" event name the CLI's response printer looks for) round-trips.
#[test]
fn markup_reports_a_verified_link_over_the_socket() {
    let dir = temp_dir("markup");
    let mut child = start(
        &dir,
        &[
            "sh",
            "-c",
            "printf '\\033]8;;https://example.test\\033\\\\LINK\\033]8;;\\033\\\\'; sleep 600",
        ],
        "600",
    );
    let _ = request_all(
        &dir,
        r#"{"v":1,"op":"wait","contains":"LINK"}
"#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "markup",
            "1",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("<a href=\"https://example.test\">LINK</a>"),
        "markup did not report the verified link: {text:?}"
    );
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn locate_returns_visible_coordinates_without_sending_input() {
    let dir = temp_dir("locate");
    let mut child = start(&dir, &["sh", "-c", "printf TARGET; sleep 600"], "600");
    let _ = request_all(
        &dir,
        r#"{"v":1,"op":"wait","contains":"TARGET"}
"#,
    );
    let matches = request_all(
        &dir,
        r#"{"v":1,"op":"locate","query":"TARGET"}
"#,
    );
    assert_eq!(matches[0]["event"], "locate");
    assert_eq!(matches[0]["matches"][0]["row"], 1);
    assert_eq!(matches[0]["matches"][0]["col"], 1);
    assert_eq!(matches[0]["matches"][0]["width"], 6);
    assert_eq!(matches[1]["event"], "ack");
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn wait_returns_a_snapshot_after_a_screen_predicate() {
    let dir = temp_dir("wait");
    let mut child = start(
        &dir,
        &[
            "sh",
            "-c",
            "printf before; sleep .1; printf TARGET; read _park",
        ],
        "600",
    );
    let events = request_all(
        &dir,
        r#"{"v":1,"op":"wait","contains":"TARGET","timeout_ms":30000}
"#,
    );
    let wait = events
        .iter()
        .find(|event| event["event"] == "wait")
        .unwrap();
    assert_eq!(wait["matched"], true);
    assert!(wait["rows"].to_string().contains("TARGET"));
    assert!(events.iter().any(|event| event["event"] == "ack"));
    child.kill().ok();
    child.wait().unwrap();
}

#[test]
fn session_file_reuses_socket_and_command_without_repeating_arguments() {
    let state = temp_dir("session-state");
    let mut child = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .env("INTERACTIVE_SHELL_AGENT", "session-agent")
        .args([
            "--session",
            "resume-case",
            "--cols",
            "20",
            "--rows",
            "4",
            "--idle-timeout",
            "600",
            "--",
            "sh",
            "-c",
            "printf SESSION_READY; sleep 600",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let session_file = state.join("sessions/resume-case.json");
    for _ in 0..READY_POLLS {
        if session_file.exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    assert!(session_file.exists());
    let session: Value = serde_json::from_str(&fs::read_to_string(&session_file).unwrap()).unwrap();
    let socket = PathBuf::from(session["socket"].as_str().unwrap());
    assert!(socket.ends_with("term.sock"));
    assert_eq!(
        fs::metadata(&session_file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(socket.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(session["agent"], "session-agent");
    let input = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "resume-case", "wait", "SESSION_READY", "30000"])
        .output()
        .unwrap();
    assert!(
        input.status.success(),
        "session wait failed: {}\n{}",
        String::from_utf8_lossy(&input.stderr).trim_end(),
        wrapper_stderr(socket.parent().unwrap())
    );
    assert!(String::from_utf8_lossy(&input.stdout).contains("\"matched\":true"));
    let shutdown = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "resume-case", "shutdown"])
        .output()
        .unwrap();
    assert!(shutdown.status.success());
    child.wait().unwrap();

    let mut restarted = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "resume-case"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..READY_POLLS {
        if socket.exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    assert!(socket.exists());
    let resumed = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "resume-case", "wait", "SESSION_READY", "30000"])
        .output()
        .unwrap();
    assert!(resumed.status.success());
    assert!(String::from_utf8_lossy(&resumed.stdout).contains("\"matched\":true"));
    let _ = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "resume-case", "shutdown"])
        .output();
    restarted.wait().unwrap();
}

#[test]
fn observe_exposes_osc8_elements_and_click_targets() {
    let dir = temp_dir("elements");
    let mut child = start(
        &dir,
        &[
            "sh",
            "-c",
            "printf '\\033]8;;https://example.test\\033\\\\LINK\\033]8;;\\033\\\\'; sleep 600",
        ],
        "600",
    );
    // Wait for the link to reach the screen first. `observe` used to be issued
    // the instant the wrapper was up, so `elements[0]` was whatever had been
    // parsed by then -- null on a runner that had not scheduled the fixture yet.
    wait_for_rows(&dir, "LINK");
    let snapshot = request_all(
        &dir,
        r#"{"v":1,"op":"observe"}
"#,
    );
    let screen = snapshot
        .iter()
        .find(|event| event["event"] == "snapshot")
        .unwrap();
    let element = &screen["elements"][0];
    assert_eq!(element["label"], "LINK");
    assert_eq!(element["uri"], "https://example.test");
    assert_eq!(element["actionable"], true);
    assert_eq!(element["highlighted"], false);
    assert!(screen["elements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|candidate| candidate["id"] == "text-0-0"));
    assert_eq!(
        screen["elements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["id"] == "text-0-0")
            .unwrap()["actionable"],
        false
    );
    let elements = request_all(
        &dir,
        r#"{"v":1,"op":"elements"}
"#,
    );
    assert_eq!(elements[0]["event"], "elements");
    assert_eq!(elements[0]["elements"].as_array().unwrap().len(), 1);
    assert_eq!(elements[1]["event"], "ack");
    assert_eq!(elements[0]["elements"][0]["label"], "LINK");
    let filtered = request_all(
        &dir,
        r#"{"v":1,"op":"elements","rows":[1]}
"#,
    );
    assert_eq!(filtered[0]["event"], "elements");
    assert_eq!(filtered[0]["elements"].as_array().unwrap().len(), 1);
    let empty = request_all(
        &dir,
        r#"{"v":1,"op":"elements","rows":[2]}
"#,
    );
    assert!(empty[0]["elements"].as_array().unwrap().is_empty());
    let id = element["id"].as_str().unwrap();
    let body = format!(
        r#"{{"v":1,"op":"click","id":"{id}","button":0}}
"#
    );
    assert_eq!(request(&dir, &body)["event"], "ack");
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn observe_drops_osc8_elements_after_their_cells_are_erased() {
    let dir = temp_dir("stale-elements");
    let mut child = start(
        &dir,
        &[
            "sh",
            "-c",
            // Input-driven, like the view test: the erase happens when this
            // test asks for it, not on a timer racing the assertion.
            "stty -echo; printf '\\033]8;;https://example.test\\033\\\\LINK\\033]8;;\\033\\\\'; read _go; printf '\\033[2J'; read _park",
        ],
        "600",
    );
    wait_for_rows(&dir, "LINK");
    let ack = request(&dir, "{\"v\":1,\"op\":\"text\",\"text\":\"\\n\"}\n");
    assert_eq!(ack["event"], "ack", "text op was not acknowledged: {ack:?}");
    wait_until_gone(&dir, "LINK");
    let snapshot = request_all(
        &dir,
        r#"{"v":1,"op":"observe"}
"#,
    )
    .into_iter()
    .find(|event| event["event"] == "snapshot")
    .unwrap();
    assert!(snapshot["elements"]
        .as_array()
        .unwrap()
        .iter()
        .all(|element| element["uri"] != "https://example.test"));
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn malformed_cli_arguments_do_not_panic() {
    let wrapper_help = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(wrapper_help.status.success());
    let wrapper_text = String::from_utf8_lossy(&wrapper_help.stdout);
    assert!(wrapper_text.contains("usage:"));
    assert!(wrapper_text.contains("--session ID"));
    assert!(wrapper_text.contains("default 80x24"));
    assert!(wrapper_text.contains("does not know application keybindings"));
    let wrapper = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .arg("--socket")
        .output()
        .unwrap();
    assert!(!wrapper.status.success());
    assert!(!String::from_utf8_lossy(&wrapper.stderr).contains("panicked"));
    let input = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .arg("--socket")
        .output()
        .unwrap();
    assert!(!input.status.success());
    assert!(!String::from_utf8_lossy(&input.stderr).contains("panicked"));
}

/// A request whose body arrives after the connect is still served.
///
/// This is a macOS-specific failure turned into a test. The listener is
/// non-blocking so the run loop can poll it between reads of the pty master,
/// and BSD copies that O_NONBLOCK onto the socket `accept()` returns while
/// Linux does not. So on macOS the wrapper's first read of a request that had
/// not yet arrived returned EAGAIN, client() failed, the connection was
/// dropped, and the caller's input never reached the program:
///
///     interactive-shell client: Resource temporarily unavailable (os error 35)
///
/// 35 is EAGAIN on macOS and 11 on Linux, so even the errno differs by leg.
///
/// The delay is what makes the mechanism reachable on either platform. Without
/// it the request is always already buffered by the time the wrapper reads,
/// so the defect is invisible on a fast machine. One test pays the delay
/// rather than every exchange in this file.
#[test]
fn a_request_body_that_arrives_late_is_still_served() {
    let dir = temp_dir("late-body");
    let mut child = start(&dir, &["sh", "-c", "sleep 600"], "600");
    wait_for_socket(&dir);
    let mut stream = connect_socket(&dir).expect("the socket must accept a connection");
    thread::sleep(Duration::from_millis(150));
    let reply = exchange(&dir, &mut stream, "{\"v\":1,\"op\":\"observe\"}\n");
    assert!(
        reply.contains("\"event\":\"snapshot\""),
        "a late request body must still be answered, got {reply:?}\n{}",
        wrapper_stderr(&dir)
    );
    // Reaped, not just killed: clippy::zombie_processes is denied here, and a
    // fixture parked for 600 seconds is exactly the one worth not leaving
    // behind on a runner that goes on to run more tests.
    child.kill().ok();
    child.wait().ok();
}

#[test]
fn cli_text_preserves_spaces_and_input_help_is_available() {
    let dir = temp_dir("cli-text");
    let mut child = start(&dir, &["sh", "-c", "sleep 600"], "600");
    let help = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args(["--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    let help_text = String::from_utf8_lossy(&help.stdout);
    assert!(help_text.contains("operations:"));
    assert!(help_text.contains("view 10-15"));
    assert!(help_text.contains("locate TEXT"));
    assert!(help_text.contains("rgbview"));
    assert!(help_text.contains("elements [ROWS...]"));
    assert!(help_text.contains("Application keybindings are unknown"));
    // A one-shot Command gets no retry, unlike request()'s connect loop.
    wait_for_socket(&dir);
    let input = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "text",
            "Hello",
            "World",
        ])
        .output()
        .unwrap();
    assert!(
        input.status.success(),
        "text send failed: {}\n{}",
        String::from_utf8_lossy(&input.stderr).trim_end(),
        wrapper_stderr(&dir)
    );
    // Confirmed against the screen, not against the ack: the ack said the
    // wrapper took the input, which is a different claim.
    wait_for_rows(&dir, "Hello World");
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn signal_cleanup_removes_socket() {
    let dir = temp_dir("signal");
    let mut child = start(&dir, &["sleep", "600"], "600");
    for _ in 0..READY_POLLS {
        if dir.join("socket").exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    unsafe {
        libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
    }
    child.wait().unwrap();
    assert!(!dir.join("socket").exists());
}

#[test]
fn idle_timeout_reports_status_124() {
    let dir = temp_dir("idle");
    let mut child = start(&dir, &["sleep", "600"], "1");
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    child.wait().unwrap();
    assert!(
        output
            .lines()
            .any(|line| line.contains("\"reason\":\"idle_timeout\"")
                && line.contains("\"status\":124")),
        "no idle_timeout lifecycle line\n{}",
        wrapper_stderr(&dir)
    );
    assert!(!dir.join("socket").exists());
}

#[test]
fn long_input_and_descendants_are_handled() {
    let dir = temp_dir("long-input");
    let pid_file = dir.join("descendant.pid");
    let command = format!(
        "sleep 600 & echo $! > {}; wc -c >/dev/null; wait",
        pid_file.display()
    );
    let mut child = start(&dir, &["sh", "-c", &command], "600");
    for _ in 0..READY_POLLS {
        if pid_file.exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    let text = "x".repeat(60_000) + "\n";
    let body = serde_json::json!({"v":1,"op":"text","text":text}).to_string() + "\n";
    let ack = request(&dir, &body);
    assert_eq!(ack["event"], "ack");
    let descendant: libc::pid_t = fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let _ = request(&dir, "{\"v\":1,\"op\":\"shutdown\"}\n");
    child.wait().unwrap();
    for _ in 0..READY_POLLS {
        if unsafe { libc::kill(descendant, 0) } == -1 {
            return;
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("descendant survived wrapper cleanup");
}

/// `--tcp` end to end: a real `interactive-shell --tcp` server, a real
/// `connect_in_directory` connect through the crate's own auto-detection
/// (never told which transport is in play), and positive proof it actually
/// went over TCP rather than happening to also work by falling back to a
/// Unix socket -- the discovery file at `dir/socket` must parse as a bare
/// `<port>\n<hex nonce>\n`, which a real bound Unix socket special file
/// cannot (opening one as a plain file fails fast, confirmed directly against
/// this host).
#[test]
fn tcp_transport_serves_screen_events_over_a_discovery_file() {
    let dir = temp_dir("tcp-events");
    let mut child = start_tcp(
        &dir,
        &["sh", "-c", "stty size; printf first; sleep 1"],
        "600",
    );
    wait_for_socket(&dir);
    let discovery = fs::read_to_string(dir.join("socket")).unwrap_or_else(|error| {
        panic!(
            "discovery file unreadable: {error}\n{}",
            wrapper_stderr(&dir)
        )
    });
    let mut lines = discovery.lines();
    let port: u16 = lines
        .next()
        .unwrap_or_else(|| panic!("discovery file has no port line: {discovery:?}"))
        .parse()
        .unwrap_or_else(|error| {
            panic!("discovery file port did not parse: {error}\n{discovery:?}")
        });
    assert!(port > 0, "an ephemeral TCP port must never be 0");
    assert!(
        lines.next().is_some_and(|nonce| nonce.len() == 64),
        "discovery file nonce should be 64 hex characters: {discovery:?}"
    );

    let ack = request(
        &dir,
        r#"{"v":1,"op":"key","key":"ENTER"}
"#,
    );
    assert_eq!(ack["event"], "ack");
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(status.success());
    assert!(output
        .lines()
        .any(|line| line.contains("\"event\":\"screen\"")));
    assert!(output
        .lines()
        .any(|line| line.contains("\"event\":\"lifecycle\"")));
}

/// A `--tcp` session's own restart (via `--session`, with no `--tcp` repeated)
/// stays on TCP: the saved session file remembers `use_tcp`, matching how it
/// already remembers socket/command/dimensions.
#[test]
fn session_file_remembers_tcp_transport_across_a_restart() {
    let state = temp_dir("tcp-session-state");
    let mut child = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .env("INTERACTIVE_SHELL_AGENT", "tcp-session-agent")
        .args([
            "--session",
            "tcp-resume-case",
            "--tcp",
            "--cols",
            "20",
            "--rows",
            "4",
            "--idle-timeout",
            "600",
            "--",
            "sh",
            "-c",
            "printf SESSION_READY; sleep 600",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let session_file = state.join("sessions/tcp-resume-case.json");
    for _ in 0..READY_POLLS {
        if session_file.exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    assert!(session_file.exists());
    let session: Value = serde_json::from_str(&fs::read_to_string(&session_file).unwrap()).unwrap();
    assert_eq!(session["use_tcp"], true);
    let socket = PathBuf::from(session["socket"].as_str().unwrap());
    let input = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args([
            "--session",
            "tcp-resume-case",
            "wait",
            "SESSION_READY",
            "30000",
        ])
        .output()
        .unwrap();
    assert!(
        input.status.success(),
        "session wait failed: {}\n{}",
        String::from_utf8_lossy(&input.stderr).trim_end(),
        wrapper_stderr(socket.parent().unwrap())
    );
    assert!(String::from_utf8_lossy(&input.stdout).contains("\"matched\":true"));
    let shutdown = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "tcp-resume-case", "shutdown"])
        .output()
        .unwrap();
    assert!(shutdown.status.success());
    child.wait().unwrap();

    // Restarted with NO --tcp repeated: the saved session file alone must
    // carry the transport choice forward.
    let mut restarted = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "tcp-resume-case"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..READY_POLLS {
        if socket.exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    assert!(socket.exists());
    let discovery = fs::read_to_string(&socket).unwrap_or_else(|error| {
        panic!(
            "resumed session's socket path did not parse as a TCP discovery file: {error}\n{}",
            wrapper_stderr(socket.parent().unwrap())
        )
    });
    assert!(
        discovery
            .lines()
            .next()
            .is_some_and(|port| port.parse::<u16>().is_ok()),
        "resumed session did not use TCP: {discovery:?}"
    );
    let resumed = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args([
            "--session",
            "tcp-resume-case",
            "wait",
            "SESSION_READY",
            "30000",
        ])
        .output()
        .unwrap();
    assert!(
        resumed.status.success(),
        "wait on the resumed TCP session failed: {}\n{}",
        String::from_utf8_lossy(&resumed.stderr),
        wrapper_stderr(socket.parent().unwrap())
    );
    assert!(String::from_utf8_lossy(&resumed.stdout).contains("\"matched\":true"));
    let _ = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "tcp-resume-case", "shutdown"])
        .output();
    restarted.wait().unwrap();
}
