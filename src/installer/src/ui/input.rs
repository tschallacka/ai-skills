// MODE: DEV
// PACKAGE: PROD
//! Decodes a byte stream into keys, including SGR mouse reports
//! (`ESC [ < Cb ; Cx ; Cy M/m`, enabled by `terminal::enter` alongside raw
//! mode) -- a left-button press becomes `Key::Click`, the wheel becomes
//! `Key::Up`/`Key::Down`, and everything else this slice has no use for
//! (release, drag, other buttons) is swallowed rather than surfaced as a
//! spurious `Escape`.
//!
//! Uses a short timeout to tell an arrow key's trailing bytes (or a mouse
//! report's) from a bare Escape, and a separate, longer timeout for the idle
//! tick -- two independent constants, each free to change without affecting
//! the other.

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
    /// A left-button press, at 1-based (column, row) -- the same coordinate
    /// system `terminal::size`/SGR cursor addressing already uses elsewhere
    /// in this crate, so a caller can compare it directly against a row it
    /// already printed at that row number. Release, drag, other buttons and
    /// the scroll wheel are decoded but never reach here as their own
    /// variant: the wheel folds into `Up`/`Down` instead, since a picker's
    /// wheel and arrow-key scrolling are the same action.
    Click {
        col: u16,
        row: u16,
    },
    /// DEL (0x7f, what most terminals actually send for the Backspace key)
    /// or BS (0x08, what a few still do) -- both read as the same key,
    /// since no caller here has a reason to tell them apart.
    Backspace,
    /// No byte arrived within the idle timeout -- just "nothing happened,
    /// keep waiting".
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
        Some(b'<') => decode_mouse(rx),
        Some(b) if b.is_ascii_digit() => decode_tilde(b, rx),
        Some(b) => key_from_final_byte(b),
    }
}

/// Consumes the rest of an SGR mouse report after `ESC [ <` (already
/// consumed by the caller): `Cb;Cx;Cy` then a final `M` (press) or `m`
/// (release). A malformed or truncated report -- the continuation timeout
/// fires, a digit fails to parse, too many bytes arrive with no final byte
/// in sight -- answers `Tick` rather than `Escape`: a garbled mouse report
/// is not a keypress a caller should act on, but it is also not the user
/// pressing Escape.
fn decode_mouse(rx: &Receiver<Option<u8>>) -> Key {
    let mut digits = Vec::with_capacity(12);
    loop {
        match recv_continuation(rx) {
            Some(b @ (b'M' | b'm')) => return finish_mouse(&digits, b == b'M'),
            Some(b) if digits.len() < 24 => digits.push(b),
            _ => return Key::Tick,
        }
    }
}

/// `Cb`'s low bits name the button (0/1/2 = left/middle/right button; 64/65
/// = wheel up/down, once the higher bits below are masked off); this slice
/// only acts on a left-button press or the wheel, and folds everything else
/// -- a release, a drag, the middle or right button -- into `Tick`, since
/// none of it is a key a picker uses. The wheel folds into `Up`/`Down`
/// rather than its own variant: a picker's wheel and arrow-key scrolling
/// are the same action.
fn finish_mouse(digits: &[u8], pressed: bool) -> Key {
    let text = String::from_utf8_lossy(digits);
    let mut parts = text.split(';');
    let (Some(cb), Some(cx), Some(cy)) = (parts.next(), parts.next(), parts.next()) else {
        return Key::Tick;
    };
    let (Ok(cb), Ok(cx), Ok(cy)) = (cb.parse::<u32>(), cx.parse::<u16>(), cy.parse::<u16>()) else {
        return Key::Tick;
    };
    if !pressed {
        return Key::Tick;
    }
    // Mask off the modifier (shift=4, meta=8, ctrl=16) and motion (32) bits:
    // this slice does not distinguish a modified or dragged click from a
    // plain one.
    match cb & !0x1c {
        0 => Key::Click { col: cx, row: cy },
        64 => Key::Up,
        65 => Key::Down,
        _ => Key::Tick,
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
        0x03 => Key::Char('q'), // Ctrl-C reads as the quit key.
        0x7f | 0x08 => Key::Backspace,
        b if b.is_ascii() => Key::Char(b as char),
        _ => Key::Escape,
    }
}

/// Blocks for up to `IDLE_TICK_TIMEOUT`; `Tick` on timeout, `Eof` once the
/// reader thread has nothing left to send.
pub fn read_key(rx: &Receiver<Option<u8>>) -> Key {
    read_key_within(rx, IDLE_TICK_TIMEOUT)
}

/// `read_key` with its own tick interval, for a screen that animates faster
/// than the idle tick.
pub fn read_key_within(rx: &Receiver<Option<u8>>, tick: Duration) -> Key {
    match rx.recv_timeout(tick) {
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
    fn a_left_click_sgr_report_decodes_its_column_and_row() {
        assert_eq!(
            read_key(&feed(b"\x1b[<0;10;5M")),
            Key::Click { col: 10, row: 5 }
        );
    }

    #[test]
    fn the_wheel_folds_into_up_and_down_rather_than_its_own_variant() {
        assert_eq!(read_key(&feed(b"\x1b[<64;1;1M")), Key::Up);
        assert_eq!(read_key(&feed(b"\x1b[<65;1;1M")), Key::Down);
    }

    #[test]
    fn a_release_report_is_swallowed_as_a_tick_not_a_click() {
        assert_eq!(read_key(&feed(b"\x1b[<0;10;5m")), Key::Tick);
    }

    #[test]
    fn a_middle_or_right_button_press_is_swallowed_as_a_tick() {
        assert_eq!(read_key(&feed(b"\x1b[<1;10;5M")), Key::Tick);
        assert_eq!(read_key(&feed(b"\x1b[<2;10;5M")), Key::Tick);
    }

    #[test]
    fn a_shift_modified_left_click_still_decodes_as_a_plain_click() {
        // Cb=4 sets only the shift bit on top of button 0 (left); this
        // slice does not distinguish a modified click from a plain one, so
        // it must still decode the coordinates rather than falling through
        // to the button-1/2 Tick arm.
        assert_eq!(
            read_key(&feed(b"\x1b[<4;10;5M")),
            Key::Click { col: 10, row: 5 }
        );
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
    fn del_and_bs_both_read_as_backspace() {
        assert_eq!(read_key(&feed(&[0x7f])), Key::Backspace);
        assert_eq!(read_key(&feed(&[0x08])), Key::Backspace);
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
