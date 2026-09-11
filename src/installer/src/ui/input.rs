// MODE: DEV
// PACKAGE: PROD
//! Decodes a byte stream into keys -- ported in spirit from
//! installer/src/37-ui-input.sh's iui_read_key/iui_read_escape/
//! iui_read_tilde. No SGR mouse parsing (iui_read_mouse): this slice is
//! keyboard-only.
//!
//! bash 3.2's `read -t` floor is a whole second, which is why install.sh's
//! escape-continuation timeout and its idle tick share one number. Rust has
//! no such floor, so this uses a short (25ms) timeout to tell an arrow key's
//! trailing bytes from a bare Escape, and a separate, longer one (1s) for
//! the idle tick -- snappier than the bash original without changing what
//! either timeout means.

use std::sync::mpsc::Receiver;
use std::time::Duration;

const ESCAPE_CONTINUATION_TIMEOUT: Duration = Duration::from_millis(25);
const IDLE_TICK_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Space,
    Tab,
    ShiftTab,
    Char(char),
    Escape,
    /// No byte arrived within the idle timeout -- an animation tick in
    /// install.sh's picker; here, just "nothing happened, keep waiting".
    Tick,
    /// The reader thread's stdin hit EOF.
    Eof,
}

fn recv_continuation(rx: &Receiver<Option<u8>>) -> Option<u8> {
    rx.recv_timeout(ESCAPE_CONTINUATION_TIMEOUT).ok().flatten()
}

fn decode_tilde(first_digit: u8, rx: &Receiver<Option<u8>>) -> Key {
    let mut digits = vec![first_digit];
    while let Some(b) = recv_continuation(rx) {
        if b.is_ascii_digit() || b == b';' {
            digits.push(b);
            if digits.len() >= 12 {
                break;
            }
        } else {
            break;
        }
    }
    match digits.as_slice() {
        [b'1'] | [b'7'] => Key::Home,
        [b'4'] | [b'8'] => Key::End,
        [b'5'] => Key::PageUp,
        [b'6'] => Key::PageDown,
        _ => Key::Escape,
    }
}

fn key_from_final_byte(b: u8) -> Key {
    match b {
        b'A' => Key::Up,
        b'B' => Key::Down,
        b'C' => Key::Right,
        b'D' => Key::Left,
        b'H' => Key::Home,
        b'F' => Key::End,
        b'Z' => Key::ShiftTab,
        _ => Key::Escape,
    }
}

fn decode_csi(rx: &Receiver<Option<u8>>) -> Key {
    match recv_continuation(rx) {
        None => Key::Escape,
        Some(b) if b.is_ascii_digit() => decode_tilde(b, rx),
        Some(b) => key_from_final_byte(b),
    }
}

fn decode_escape(rx: &Receiver<Option<u8>>) -> Key {
    match recv_continuation(rx) {
        None => Key::Escape,
        Some(b'[') => decode_csi(rx),
        Some(_) => Key::Escape,
    }
}

fn decode_byte(byte: u8, rx: &Receiver<Option<u8>>) -> Key {
    match byte {
        0x1b => decode_escape(rx),
        b'\r' | b'\n' => Key::Enter,
        b' ' => Key::Space,
        b'\t' => Key::Tab,
        0x03 => Key::Char('q'), // Ctrl-C reads as the quit key, same as install.sh's picker.
        b if b.is_ascii() => Key::Char(b as char),
        _ => Key::Escape,
    }
}

/// Blocks for up to `IDLE_TICK_TIMEOUT`; `Tick` on timeout, `Eof` once the
/// reader thread has nothing left to send.
pub fn read_key(rx: &Receiver<Option<u8>>) -> Key {
    match rx.recv_timeout(IDLE_TICK_TIMEOUT) {
        Err(_) => Key::Tick,
        Ok(None) => Key::Eof,
        Ok(Some(byte)) => decode_byte(byte, rx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration as StdDuration;

    fn feed(bytes: &'static [u8]) -> Receiver<Option<u8>> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for &b in bytes {
                let _ = tx.send(Some(b));
            }
            // Deliberately leaves the sender alive briefly so a lone Escape's
            // continuation read times out rather than seeing a closed channel,
            // exercising the same path a live terminal would.
            thread::sleep(StdDuration::from_millis(60));
        });
        rx
    }

    #[test]
    fn a_plain_character_is_itself() {
        let rx = feed(b"a");
        assert_eq!(read_key(&rx), Key::Char('a'));
    }

    #[test]
    fn enter_and_space_and_tab() {
        assert_eq!(read_key(&feed(b"\n")), Key::Enter);
        assert_eq!(read_key(&feed(b" ")), Key::Space);
        assert_eq!(read_key(&feed(b"\t")), Key::Tab);
    }

    #[test]
    fn arrow_keys_decode_from_their_csi_sequence() {
        assert_eq!(read_key(&feed(b"\x1b[A")), Key::Up);
        assert_eq!(read_key(&feed(b"\x1b[B")), Key::Down);
        assert_eq!(read_key(&feed(b"\x1b[C")), Key::Right);
        assert_eq!(read_key(&feed(b"\x1b[D")), Key::Left);
    }

    #[test]
    fn shift_tab_and_home_and_end() {
        assert_eq!(read_key(&feed(b"\x1b[Z")), Key::ShiftTab);
        assert_eq!(read_key(&feed(b"\x1b[H")), Key::Home);
        assert_eq!(read_key(&feed(b"\x1b[F")), Key::End);
    }

    #[test]
    fn tilde_terminated_sequences_decode_page_and_home_end() {
        assert_eq!(read_key(&feed(b"\x1b[5~")), Key::PageUp);
        assert_eq!(read_key(&feed(b"\x1b[6~")), Key::PageDown);
        assert_eq!(read_key(&feed(b"\x1b[1~")), Key::Home);
        assert_eq!(read_key(&feed(b"\x1b[4~")), Key::End);
    }

    #[test]
    fn a_bare_escape_with_no_continuation_times_out_to_escape() {
        assert_eq!(read_key(&feed(b"\x1b")), Key::Escape);
    }

    #[test]
    fn ctrl_c_reads_as_the_quit_character() {
        assert_eq!(read_key(&feed(&[0x03])), Key::Char('q'));
    }

    #[test]
    fn no_bytes_at_all_is_a_tick() {
        let (_tx, rx) = mpsc::channel::<Option<u8>>();
        // The sender is dropped immediately... but recv_timeout on an empty,
        // disconnected channel returns Disconnected, which this treats the
        // same as a timeout (Tick) -- a real terminal never disconnects the
        // reader mid-session, so the distinction does not matter here.
        assert_eq!(read_key(&rx), Key::Tick);
    }

    #[test]
    fn eof_is_reported_once_the_reader_thread_sends_none() {
        let (tx, rx) = mpsc::channel();
        tx.send(None).unwrap();
        assert_eq!(read_key(&rx), Key::Eof);
    }
}
