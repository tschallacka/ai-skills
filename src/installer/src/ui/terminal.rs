// MODE: DEV
// PACKAGE: PROD
//! Raw mode, the alternate screen, and a background byte reader. Shells
//! out to `stty`: no termios crate in this workspace, and this
//! repository's dependency ceiling does not admit one for a single ioctl
//! wrapper.
//!
//! Reads this process's own stdin directly rather than reopening /dev/tty
//! on a separate fd: this binary is never run with stdin piped away from a
//! script, so a plain `[ -t 0 ]` equivalent (`IsTerminal`) is the whole
//! story.

use std::io::{IsTerminal, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, Once, OnceLock};
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
    // ?1000 (basic button tracking) + ?1006 (SGR extended coordinates, so a
    // click past column/row 223 still decodes -- the older non-SGR encoding
    // packs each coordinate into one byte and cannot). input::decode_mouse
    // is the other half of this contract; enabling tracking with nothing on
    // the read side to parse it would leave a raw mouse report arriving as
    // garbage keystrokes instead.
    print!("\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H\x1b[?1000h\x1b[?1006h");
    use std::io::Write;
    let _ = std::io::stdout().flush();
    saved
}

pub fn leave(saved: &str) {
    print!("\x1b[?1000l\x1b[?1006l\x1b[?25h\x1b[?1049l");
    use std::io::Write;
    let _ = std::io::stdout().flush();
    if !saved.is_empty() {
        let _ = stty(&[saved]);
    }
}

/// `stty size` against the inherited tty (`ROWS COLS`), falling back to
/// 80x24 when even that fails.
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

/// Drops the complete SGR mouse reports (`ESC [ < Cb ; Cx ; Cy M|m`) from
/// bytes held for the next screen. A click's release that arrives after the
/// screen which read its press has already exited is a mouse report, not a
/// keypress; left in the queue, the next screen reads it as stray keys (B385).
fn without_mouse_reports(bytes: Vec<Option<u8>>) -> Vec<Option<u8>> {
    let mut kept = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match mouse_report_end(&bytes, i) {
            Some(end) => i = end,
            None => {
                kept.push(bytes[i]);
                i += 1;
            }
        }
    }
    kept
}

/// The index just past the mouse report starting at `start`, if one starts
/// there. Anything that is not a complete report answers `None`, so the
/// bytes of a truncated or unrelated sequence are kept, not dropped.
fn mouse_report_end(bytes: &[Option<u8>], start: usize) -> Option<usize> {
    if bytes.get(start..start + 3)? != [Some(0x1b), Some(b'['), Some(b'<')] {
        return None;
    }
    let mut i = start + 3;
    loop {
        match bytes.get(i)? {
            Some(b'0'..=b'9' | b';') => i += 1,
            Some(b'M' | b'm') => return Some(i + 1),
            _ => return None,
        }
    }
}

/// Routes stdin bytes to whichever screen subscribed last. A byte read while
/// no screen listens is held for the next one: a per-screen reader thread
/// blocked in `read` used to swallow the next screen's first byte.
#[derive(Default)]
struct ReaderHub {
    current: Option<Sender<Option<u8>>>,
    pending: Vec<Option<u8>>,
}

impl ReaderHub {
    fn subscribe(&mut self) -> Receiver<Option<u8>> {
        let (tx, rx) = mpsc::channel();
        for item in without_mouse_reports(std::mem::take(&mut self.pending)) {
            let _ = tx.send(item);
        }
        self.current = Some(tx);
        rx
    }

    fn deliver(&mut self, item: Option<u8>) {
        let sent = self
            .current
            .as_ref()
            .is_some_and(|tx| tx.send(item).is_ok());
        if !sent {
            self.current = None;
            self.pending.push(item);
        }
    }
}

static HUB: OnceLock<Mutex<ReaderHub>> = OnceLock::new();
static READER: Once = Once::new();

fn hub() -> &'static Mutex<ReaderHub> {
    HUB.get_or_init(Mutex::default)
}

/// Subscribes the calling screen to the one process-wide stdin reader, so
/// its event loop can tell "no key yet" (Tick) from "a key arrived" with
/// `recv_timeout`. Receives `None` once stdin hits EOF or errors.
pub fn spawn_reader() -> Receiver<Option<u8>> {
    let rx = hub().lock().unwrap_or_else(|e| e.into_inner()).subscribe();
    READER.call_once(|| {
        thread::spawn(|| {
            let mut stdin = std::io::stdin();
            let mut buf = [0u8; 1];
            loop {
                let item = match stdin.read(&mut buf) {
                    Ok(0) | Err(_) => None,
                    Ok(_) => Some(buf[0]),
                };
                hub()
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .deliver(item);
                if item.is_none() {
                    break;
                }
            }
        });
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

/// Paints `lines` (already SGR-colored, e.g. by ui::mascot::head_line) at
/// consecutive rows starting at `row` (1-based), column `col` (1-based) --
/// the overlay used for the mascot, kept separate from `draw` because a
/// colored line's byte length is not its display width.
pub fn draw_overlay(row: usize, col: usize, lines: &[String]) {
    use std::io::Write;
    let mut out = std::io::stdout();
    let mut buffer = String::new();
    for (i, line) in lines.iter().enumerate() {
        buffer.push_str(&format!("\x1b[{};{}H", row + i, col));
        buffer.push_str(line);
    }
    let _ = out.write_all(buffer.as_bytes());
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_byte_read_after_a_screen_ends_reaches_the_next_screen() {
        let mut hub = ReaderHub::default();
        let wizard = hub.subscribe();
        hub.deliver(Some(b'w'));
        assert_eq!(wizard.try_recv(), Ok(Some(b'w')));
        drop(wizard);
        hub.deliver(Some(0x1b));
        hub.deliver(Some(b'['));
        let picker = hub.subscribe();
        hub.deliver(Some(b'<'));
        let got: Vec<_> = picker.try_iter().collect();
        assert_eq!(got, vec![Some(0x1b), Some(b'['), Some(b'<')]);
    }

    #[test]
    fn a_mouse_release_that_arrives_after_its_screen_exits_is_not_read_as_keys() {
        let mut hub = ReaderHub::default();
        for &b in b"\x1b[<0;38;13mq" {
            hub.deliver(Some(b));
        }
        let picker = hub.subscribe();
        let got: Vec<_> = picker.try_iter().collect();
        assert_eq!(got, vec![Some(b'q')]);
    }

    #[test]
    fn an_arrow_key_held_for_the_next_screen_is_kept() {
        let mut hub = ReaderHub::default();
        for &b in b"\x1b[A" {
            hub.deliver(Some(b));
        }
        let picker = hub.subscribe();
        let got: Vec<_> = picker.try_iter().collect();
        assert_eq!(got, vec![Some(0x1b), Some(b'['), Some(b'A')]);
    }

    #[test]
    fn a_new_screen_takes_over_from_one_still_holding_its_receiver() {
        let mut hub = ReaderHub::default();
        let wizard = hub.subscribe();
        let picker = hub.subscribe();
        hub.deliver(Some(b'i'));
        assert_eq!(picker.try_recv(), Ok(Some(b'i')));
        assert!(wizard.try_recv().is_err());
    }
}
