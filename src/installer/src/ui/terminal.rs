// MODE: DEV
// PACKAGE: PROD
//! Raw mode, the alternate screen, and a background byte reader -- ported in
//! spirit from installer/src/37-ui-input.sh's iui_term_enter/iui_term_leave.
//! Shells out to `stty`, same as the bash original and same reasoning: no
//! termios crate in this workspace, and this repository's dependency
//! ceiling does not admit one for a single ioctl wrapper.
//!
//! Reads this process's own stdin directly rather than reopening /dev/tty on
//! a separate fd (install.sh's fd 3, there so prompts survive `curl | bash`
//! piping stdin away): this binary is never run that way, so a plain
//! `[ -t 0 ]` equivalent (`IsTerminal`) is the whole story.

use std::io::{IsTerminal, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub fn is_tty() -> bool {
    std::io::stdin().is_terminal()
}

fn stty(args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("stty")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
}

/// The saved mode string (`stty -g`), so a caller can hand it to `leave`.
/// Empty when `stty` itself is unavailable -- `leave` then skips the
/// restore rather than feeding it a garbage argument.
pub fn enter() -> String {
    let saved = stty(&["-g"])
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let _ = stty(&["raw", "-echo"]);
    print!("\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H");
    use std::io::Write;
    let _ = std::io::stdout().flush();
    saved
}

pub fn leave(saved: &str) {
    print!("\x1b[?25h\x1b[?1049l");
    use std::io::Write;
    let _ = std::io::stdout().flush();
    if !saved.is_empty() {
        let _ = stty(&[saved]);
    }
}

/// `stty size` against the inherited tty (`ROWS COLS`), falling back to
/// 80x24 the same way install.sh's iui_measure does when even that fails.
pub fn size() -> (usize, usize) {
    let fallback = (80, 24);
    let Ok(output) = stty(&["size"]) else {
        return fallback;
    };
    if !output.status.success() {
        return fallback;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut parts = text.split_whitespace();
    let (Some(rows), Some(cols)) = (parts.next(), parts.next()) else {
        return fallback;
    };
    match (cols.parse::<usize>(), rows.parse::<usize>()) {
        (Ok(c), Ok(r)) => (c.max(20), r.max(6)),
        _ => fallback,
    }
}

/// A background thread reading stdin one byte at a time, so the picker's
/// event loop can distinguish "no key yet" (Tick) from "a key arrived"
/// without blocking forever -- `recv_timeout` on the returned channel is
/// the seam. Sends `None` once and stops after stdin hits EOF or errors.
pub fn spawn_reader() -> Receiver<Option<u8>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 1];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) | Err(_) => {
                    let _ = tx.send(None);
                    break;
                }
                Ok(_) => {
                    if tx.send(Some(buf[0])).is_err() {
                        break;
                    }
                }
            }
        }
    });
    rx
}

pub fn draw(frame: &[String]) {
    use std::io::Write;
    let mut out = std::io::stdout();
    let mut buffer = String::from("\x1b[H");
    for (i, line) in frame.iter().enumerate() {
        if i > 0 {
            buffer.push_str("\r\n");
        }
        buffer.push_str(line);
    }
    let _ = out.write_all(buffer.as_bytes());
    let _ = out.flush();
}
