use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::fd::RawFd;
use std::os::unix::fs::{DirEntryExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const MAX_LINE: usize = 65_536;
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
enum Request {
    #[serde(rename = "text")]
    Text { v: u8, text: String },
    #[serde(rename = "key")]
    Key { v: u8, key: String },
    #[serde(rename = "raw")]
    Raw { v: u8, hex: String },
    #[serde(rename = "shutdown")]
    Shutdown { v: u8 },
}

#[derive(Serialize)]
struct Ack<'a> {
    v: u8,
    event: &'a str,
    op: &'a str,
}
#[derive(Serialize)]
struct ErrorEvent<'a> {
    v: u8,
    event: &'a str,
    message: &'a str,
}
#[derive(Serialize)]
struct Lifecycle<'a> {
    v: u8,
    event: &'a str,
    reason: &'a str,
    status: i32,
}
#[derive(Serialize)]
struct Cursor {
    row: usize,
    col: usize,
    visible: bool,
}
#[derive(Serialize)]
struct ScreenEvent {
    v: u8,
    event: &'static str,
    seq: u64,
    base: u64,
    rows: BTreeMap<usize, String>,
    cursor: Cursor,
}

#[derive(Clone, Debug)]
enum Parser {
    Ground,
    Esc,
    Csi(Vec<u8>),
    Osc(Vec<u8>),
    Charset,
    CsiDiscard,
    OscDiscard(bool),
}

#[derive(Debug)]
struct Screen {
    rows: Vec<Vec<u8>>,
    dirty: Vec<bool>,
    row: usize,
    col: usize,
    visible: bool,
    line_drawing: bool,
    parser: Parser,
    saved_primary: Option<ScreenState>,
}

#[derive(Clone, Debug)]
struct ScreenState {
    rows: Vec<Vec<u8>>,
    row: usize,
    col: usize,
    visible: bool,
    line_drawing: bool,
}

impl Screen {
    fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows: vec![vec![b' '; cols]; rows],
            dirty: vec![true; rows],
            row: 0,
            col: 0,
            visible: true,
            line_drawing: false,
            parser: Parser::Ground,
            saved_primary: None,
        }
    }
    fn cursor(&self) -> Cursor {
        Cursor {
            row: self.row,
            col: self.col,
            visible: self.visible,
        }
    }
    fn feed(&mut self, input: &[u8]) {
        for &byte in input {
            self.byte(byte);
        }
    }
    fn byte(&mut self, byte: u8) {
        let parser = std::mem::replace(&mut self.parser, Parser::Ground);
        match parser {
            Parser::Ground => match byte {
                0x1b => self.parser = Parser::Esc,
                b'\r' => self.col = 0,
                b'\n' => self.row = (self.row + 1).min(self.rows.len().saturating_sub(1)),
                0x08 => self.col = self.col.saturating_sub(1),
                0x0e => self.line_drawing = true,
                0x0f => self.line_drawing = false,
                0x20..=0x7e => self.put(if self.line_drawing {
                    line_drawing(byte)
                } else {
                    byte
                }),
                _ => {}
            },
            Parser::Esc => match byte {
                b'[' => self.parser = Parser::Csi(Vec::new()),
                b']' => self.parser = Parser::Osc(Vec::new()),
                b'(' | b')' => self.parser = Parser::Charset,
                0x1b => self.parser = Parser::Esc,
                _ => {}
            },
            Parser::Charset => {
                self.line_drawing = byte == b'0';
            }
            Parser::Csi(mut params) => {
                if (0x40..=0x7e).contains(&byte) {
                    self.csi(&params, byte);
                } else if params.len() < 128 {
                    params.push(byte);
                    self.parser = Parser::Csi(params);
                } else {
                    self.parser = Parser::CsiDiscard;
                }
            }
            Parser::Osc(mut value) => {
                if byte == 7 || (byte == b'\\' && value.last() == Some(&0x1b)) {
                } else if value.len() < 4096 {
                    value.push(byte);
                    self.parser = Parser::Osc(value);
                } else {
                    self.parser = Parser::OscDiscard(byte == 0x1b);
                }
            }
            Parser::CsiDiscard => {
                if !(0x40..=0x7e).contains(&byte) {
                    self.parser = Parser::CsiDiscard;
                }
            }
            Parser::OscDiscard(mut escaped) => {
                if byte == 7 || (escaped && byte == b'\\') {
                    self.parser = Parser::Ground;
                } else {
                    escaped = byte == 0x1b;
                    self.parser = Parser::OscDiscard(escaped);
                }
            }
        }
    }
    fn put(&mut self, byte: u8) {
        if let Some(row) = self.rows.get_mut(self.row) {
            if self.col < row.len() {
                row[self.col] = byte;
                self.dirty[self.row] = true;
            }
        }
        self.col = (self.col + 1).min(self.rows[0].len().saturating_sub(1));
    }
    fn csi(&mut self, raw: &[u8], final_byte: u8) {
        let s = String::from_utf8_lossy(raw);
        let private = s.starts_with('?');
        let s = s.trim_start_matches('?');
        let n = |i| {
            s.split(';')
                .nth(i)
                .and_then(|x| x.parse::<usize>().ok())
                .unwrap_or(1)
        };
        match final_byte {
            b'A' => self.row = self.row.saturating_sub(n(0)),
            b'B' => self.row = self.row.saturating_add(n(0)).min(self.rows.len() - 1),
            b'C' => self.col = self.col.saturating_add(n(0)).min(self.rows[0].len() - 1),
            b'D' => self.col = self.col.saturating_sub(n(0)),
            b'G' | b'`' => self.col = n(0).saturating_sub(1).min(self.rows[0].len() - 1),
            b'd' => self.row = n(0).saturating_sub(1).min(self.rows.len() - 1),
            b'H' | b'f' => {
                self.row = n(0).saturating_sub(1).min(self.rows.len() - 1);
                self.col = n(1).saturating_sub(1).min(self.rows[0].len() - 1);
            }
            b'J' if s.is_empty() || s == "2" => {
                for (i, row) in self.rows.iter_mut().enumerate() {
                    row.fill(b' ');
                    self.dirty[i] = true;
                }
            }
            b'K' => {
                if let Some(row) = self.rows.get_mut(self.row) {
                    row[self.col..].fill(b' ');
                    self.dirty[self.row] = true;
                }
            }
            b'h' if private && s == "25" => self.visible = true,
            b'l' if private && s == "25" => self.visible = false,
            b'h' if private && s == "1049" => self.enter_alt(),
            b'l' if private && s == "1049" => {
                self.leave_alt();
            }
            _ => {}
        }
    }
    fn enter_alt(&mut self) {
        if self.saved_primary.is_some() {
            return;
        }
        self.saved_primary = Some(ScreenState {
            rows: std::mem::take(&mut self.rows),
            row: self.row,
            col: self.col,
            visible: self.visible,
            line_drawing: self.line_drawing,
        });
        let rows = self.saved_primary.as_ref().unwrap().rows.len();
        let cols = self.saved_primary.as_ref().unwrap().rows[0].len();
        self.rows = vec![vec![b' '; cols]; rows];
        self.dirty = vec![true; rows];
        self.row = 0;
        self.col = 0;
    }
    fn leave_alt(&mut self) {
        if let Some(state) = self.saved_primary.take() {
            self.rows = state.rows;
            self.dirty = vec![true; self.rows.len()];
            self.row = state.row;
            self.col = state.col;
            self.visible = state.visible;
            self.line_drawing = state.line_drawing;
        }
    }
    fn delta(&mut self) -> BTreeMap<usize, String> {
        let mut out = BTreeMap::new();
        for (i, row) in self.rows.iter().enumerate() {
            if self.dirty[i] {
                out.insert(i, String::from_utf8_lossy(row).trim_end().to_string());
            }
        }
        self.dirty.fill(false);
        out
    }
}
fn line_drawing(b: u8) -> u8 {
    match b {
        b'q' => b'-',
        b'x' => b'|',
        b'j' | b'k' | b'l' | b'm' | b'n' => b'+',
        _ => b,
    }
}

pub fn key_bytes(key: &str) -> Option<&'static [u8]> {
    match key {
        "ENTER" => Some(b"\r"),
        "CTRL-X" => Some(b"\x18"),
        "CTRL-O" => Some(b"\x0f"),
        "CTRL-C" => Some(b"\x03"),
        "CTRL-D" => Some(b"\x04"),
        "CTRL-Z" => Some(b"\x1a"),
        "CTRL-E" => Some(b"\x05"),
        "BACKSPACE" => Some(b"\x7f"),
        "TAB" => Some(b"\t"),
        "ESC" => Some(b"\x1b"),
        "META-RIGHT" => Some(b"\x1b[1;3C"),
        "UP" => Some(b"\x1b[A"),
        "DOWN" => Some(b"\x1b[B"),
        "LEFT" => Some(b"\x1b[D"),
        "RIGHT" => Some(b"\x1b[C"),
        "HOME" => Some(b"\x1b[H"),
        "END" => Some(b"\x1b[F"),
        "PAGEUP" => Some(b"\x1b[5~"),
        "PAGEDOWN" => Some(b"\x1b[6~"),
        "INSERT" => Some(b"\x1b[2~"),
        "DELETE" => Some(b"\x1b[3~"),
        "F1" => Some(b"\x1bOP"),
        "F2" => Some(b"\x1bOQ"),
        "F3" => Some(b"\x1bOR"),
        "F4" => Some(b"\x1bOS"),
        _ => None,
    }
}
fn decode_hex(s: &str) -> Result<Vec<u8>, &'static str> {
    if s.is_empty() || !s.len().is_multiple_of(2) {
        return Err("hex must be non-empty and even-length");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "invalid hex"))
        .collect()
}
fn json<T: Serialize>(out: &mut impl Write, value: &T) -> io::Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    out.write_all(b"\n")?;
    out.flush()
}
fn valid_dir(path: &Path) -> Result<(), String> {
    let m = fs::metadata(path).map_err(|e| e.to_string())?;
    if !m.is_dir() || m.uid() != unsafe { libc::getuid() } || m.permissions().mode() & 0o077 != 0 {
        return Err("socket parent must be a private directory owned by the current user".into());
    }
    Ok(())
}
fn remove_socket(path: &Path, identity: Option<(u64, u64)>) {
    if let (Some(id), Ok(m)) = (identity, fs::metadata(path)) {
        if (m.dev(), m.ino()) == id {
            let _ = fs::remove_file(path);
        }
    }
}

fn remove_unidentified_bound_socket(path: &Path) {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_socket() {
            let _ = fs::remove_file(path);
        }
    }
}

fn capture_socket_identity(path: &Path) -> Result<(u64, u64), String> {
    let parent = path.parent().ok_or("socket needs a parent")?;
    let parent_fd = File::open(parent).map_err(|e| e.to_string())?;
    let device = parent_fd.metadata().map_err(|e| e.to_string())?.dev();
    let name = path.file_name().ok_or("socket needs a filename")?;
    loop {
        match fs::read_dir(parent) {
            Ok(entries) => {
                for entry in entries {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(_) => break,
                    };
                    if entry.file_name() == name {
                        return Ok((device, entry.ino()));
                    }
                }
                if !path.exists() {
                    return Err("bound socket disappeared before identity capture".into());
                }
            }
            Err(error) if !path.exists() => return Err(error.to_string()),
            Err(_) => {}
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

struct SocketGuard {
    path: PathBuf,
    identity: Option<(u64, u64)>,
}

impl SocketGuard {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            identity: None,
        }
    }
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        match self.identity {
            Some(identity) => remove_socket(&self.path, Some(identity)),
            None => remove_unidentified_bound_socket(&self.path),
        }
    }
}

extern "C" fn interrupt_handler(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

struct Cleanup {
    master: RawFd,
    pid: libc::pid_t,
    socket: PathBuf,
    identity: Option<(u64, u64)>,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        stop(self.pid);
        unsafe {
            libc::close(self.master);
        }
        remove_socket(&self.socket, self.identity);
    }
}
fn spawn(command: &[String], cols: u16, rows: u16) -> Result<(RawFd, libc::pid_t), String> {
    let mut master = 0;
    let mut slave = 0;
    let size = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    if unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            &size,
        )
    } < 0
    {
        return Err(io::Error::last_os_error().to_string());
    }
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        unsafe {
            libc::close(master);
            libc::close(slave)
        };
        return Err(io::Error::last_os_error().to_string());
    }
    if pid == 0 {
        unsafe {
            libc::setsid();
            libc::ioctl(slave, libc::TIOCSCTTY, 0);
            for fd in [0, 1, 2] {
                libc::dup2(slave, fd);
            }
            libc::close(master);
            libc::close(slave);
            libc::setpgid(0, 0);
            std::env::set_var("TERM", "xterm-256color");
            std::env::set_var("LC_ALL", "C");
            let c: Vec<CString> = command
                .iter()
                .map(|x| CString::new(x.as_bytes()).unwrap())
                .collect();
            let p: Vec<*const i8> = c
                .iter()
                .map(|x| x.as_ptr())
                .chain(std::iter::once(std::ptr::null()))
                .collect();
            libc::execvp(p[0], p.as_ptr());
            libc::_exit(127);
        }
    }
    unsafe {
        libc::close(slave);
        let flags = libc::fcntl(master, libc::F_GETFL);
        if flags < 0 || libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            libc::kill(-pid, libc::SIGKILL);
            libc::waitpid(pid, std::ptr::null_mut(), 0);
            libc::close(master);
            return Err(io::Error::last_os_error().to_string());
        }
    };
    Ok((master, pid))
}
fn stop(pid: libc::pid_t) {
    unsafe {
        libc::kill(-pid, libc::SIGTERM);
        let until = Instant::now() + Duration::from_millis(300);
        while Instant::now() < until {
            let mut s = 0;
            if libc::waitpid(pid, &mut s, libc::WNOHANG) == pid {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        libc::kill(-pid, libc::SIGKILL);
        libc::waitpid(pid, std::ptr::null_mut(), 0);
    }
}
fn status(s: i32) -> i32 {
    if s & 0x7f == 0 {
        s >> 8
    } else {
        -(s & 0x7f)
    }
}
fn client(
    mut stream: UnixStream,
    master: RawFd,
    stopped: &mut bool,
    reason: &mut &'static str,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| e.to_string())?;
    let mut line = Vec::new();
    let mut b = [0; 1];
    loop {
        let n = stream.read(&mut b).map_err(|e| e.to_string())?;
        if n == 0 || b[0] == b'\n' {
            break;
        }
        if line.len() >= MAX_LINE {
            return Err("request line too long".into());
        }
        line.push(b[0]);
    }
    let value: serde_json::Value = match serde_json::from_slice(&line) {
        Ok(value) => value,
        Err(error) => {
            json(
                &mut stream,
                &ErrorEvent {
                    v: 1,
                    event: "error",
                    message: "invalid JSON",
                },
            )
            .map_err(|e| e.to_string())?;
            return Err(error.to_string());
        }
    };
    if value.get("v").and_then(|v| v.as_u64()) != Some(1) {
        json(
            &mut stream,
            &ErrorEvent {
                v: 1,
                event: "error",
                message: "unsupported protocol version",
            },
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let req: Request = match serde_json::from_value(value) {
        Ok(request) => request,
        Err(error) => {
            json(
                &mut stream,
                &ErrorEvent {
                    v: 1,
                    event: "error",
                    message: "invalid request",
                },
            )
            .map_err(|e| e.to_string())?;
            return Err(error.to_string());
        }
    };
    match req {
        Request::Text { v: 1, text } => write_master(master, text.as_bytes())?,
        Request::Key { v: 1, key } => match key_bytes(&key) {
            Some(bytes) => write_master(master, bytes)?,
            None => {
                json(
                    &mut stream,
                    &ErrorEvent {
                        v: 1,
                        event: "error",
                        message: "unknown key",
                    },
                )
                .map_err(|e| e.to_string())?;
                return Ok(());
            }
        },
        Request::Raw { v: 1, hex } => match decode_hex(&hex) {
            Ok(bytes) => write_master(master, &bytes)?,
            Err(message) => {
                json(
                    &mut stream,
                    &ErrorEvent {
                        v: 1,
                        event: "error",
                        message,
                    },
                )
                .map_err(|e| e.to_string())?;
                return Ok(());
            }
        },
        Request::Shutdown { v: 1 } => {
            *stopped = true;
            *reason = "client_shutdown";
        }
        _ => return Err("unsupported protocol version".into()),
    }
    json(
        &mut stream,
        &Ack {
            v: 1,
            event: "ack",
            op: "request",
        },
    )
    .map_err(|e| e.to_string())?;
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}
fn write_master(fd: RawFd, bytes: &[u8]) -> Result<(), String> {
    let mut offset = 0;
    let deadline = Instant::now() + Duration::from_secs(2);
    while offset < bytes.len() {
        let n = unsafe { libc::write(fd, bytes[offset..].as_ptr().cast(), bytes.len() - offset) };
        if n > 0 {
            offset += n as usize;
            continue;
        }
        if n < 0 && io::Error::last_os_error().kind() == io::ErrorKind::WouldBlock {
            if Instant::now() >= deadline {
                return Err("PTY write timed out".into());
            }
            let mut poll = libc::pollfd {
                fd,
                events: libc::POLLOUT,
                revents: 0,
            };
            if unsafe { libc::poll(&mut poll, 1, 100) } < 0 {
                return Err(io::Error::last_os_error().to_string());
            }
            continue;
        }
        return Err(io::Error::last_os_error().to_string());
    }
    Ok(())
}

pub fn run(
    socket: PathBuf,
    cols: u16,
    rows: u16,
    idle: u64,
    command: Vec<String>,
) -> Result<(), String> {
    INTERRUPTED.store(false, Ordering::Relaxed);
    unsafe {
        libc::signal(
            libc::SIGTERM,
            interrupt_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            interrupt_handler as *const () as libc::sighandler_t,
        );
    }
    if !(1..=240).contains(&cols) || !(1..=100).contains(&rows) {
        return Err("dimensions must be within cols 1..240 and rows 1..100".into());
    }
    if command.is_empty() {
        return Err("command is required".into());
    }
    valid_dir(socket.parent().ok_or("socket needs parent")?)?;
    if socket.exists() {
        return Err("refusing existing socket".into());
    };
    let listener = UnixListener::bind(&socket).map_err(|e| e.to_string())?;
    let mut socket_guard = SocketGuard::new(socket.clone());
    let id = {
        let identity = capture_socket_identity(&socket)?;
        socket_guard.identity = Some(identity);
        Some(identity)
    };
    if let Err(error) = fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)) {
        remove_socket(&socket, id);
        return Err(error.to_string());
    }
    if let Err(error) = listener.set_nonblocking(true) {
        remove_socket(&socket, id);
        return Err(error.to_string());
    }
    let (master, pid) = match spawn(&command, cols, rows) {
        Ok(x) => x,
        Err(e) => {
            remove_socket(&socket, id);
            return Err(e);
        }
    };
    let cleanup = Cleanup {
        master,
        pid,
        socket: socket.clone(),
        identity: id,
    };
    std::mem::forget(socket_guard);
    let mut screen = Screen::new(rows as usize, cols as usize);
    let mut out = io::BufWriter::new(io::stdout());
    let start = Instant::now();
    let mut last = start;
    let mut stopped = false;
    let mut reason = "child_exit";
    let mut code = 0;
    let mut seq = 0;
    while !stopped && !INTERRUPTED.load(Ordering::Relaxed) {
        let mut buf = [0; 8192];
        let n = unsafe { libc::read(master, buf.as_mut_ptr().cast(), buf.len()) };
        if n > 0 {
            last = Instant::now();
            screen.feed(&buf[..n as usize]);
            seq += 1;
            json(
                &mut out,
                &ScreenEvent {
                    v: 1,
                    event: "screen",
                    seq,
                    base: seq - 1,
                    rows: screen.delta(),
                    cursor: screen.cursor(),
                },
            )
            .map_err(|e| e.to_string())?;
        }
        if let Ok((s, _)) = listener.accept() {
            if let Err(e) = client(s, master, &mut stopped, &mut reason) {
                eprintln!("interactive-shell client: {e}");
            }
        }
        let mut st = 0;
        if unsafe { libc::waitpid(pid, &mut st, libc::WNOHANG) } == pid {
            stopped = true;
            code = status(st);
        }
        if last.elapsed() >= Duration::from_secs(idle) {
            stopped = true;
            reason = "idle_timeout";
            code = 124;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    if INTERRUPTED.load(Ordering::Relaxed) {
        reason = "signal";
        code = 130;
    }
    json(
        &mut out,
        &Lifecycle {
            v: 1,
            event: "lifecycle",
            reason,
            status: code,
        },
    )
    .map_err(|e| e.to_string())?;
    drop(cleanup);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_are_stable() {
        for k in [
            "ENTER", "CTRL-X", "UP", "DOWN", "LEFT", "RIGHT", "HOME", "END", "PAGEUP", "PAGEDOWN",
            "INSERT", "DELETE", "F1", "F2", "F3", "F4",
        ] {
            assert!(key_bytes(k).is_some())
        }
    }
    #[test]
    fn parser_keeps_fragments() {
        let mut s = Screen::new(2, 10);
        s.feed(b"\x1b[");
        s.feed(b"2JX");
        assert_eq!(s.rows[0][0], b'X')
    }
    #[test]
    fn parser_fragments_other_common_sequences() {
        let mut s = Screen::new(2, 10);
        s.feed(b"abc\x1b]");
        s.feed(b"0;title\x07\x1b(");
        s.feed(b"0q\x0f");
        s.feed(b"\x1b[?1049");
        s.feed(b"hALT\x1b[?1049");
        s.feed(b"l");
        let mut overlong = vec![0x1b, b'['];
        overlong.extend(std::iter::repeat_n(b'1', 129));
        overlong.extend_from_slice(b"mSAFE");
        s.feed(&overlong);
        assert_eq!(s.rows[0][3], b'-');
        assert_eq!(s.rows[0][4], b'S');
        let mut osc = vec![0x1b, b']'];
        osc.extend(std::iter::repeat_n(b'x', 4097));
        osc.extend_from_slice(b"\x07O");
        let mut osc_screen = Screen::new(1, 10);
        osc_screen.feed(&osc);
        assert_eq!(osc_screen.rows[0][0], b'O');
    }
    #[test]
    fn alternate_screen_round_trips_primary_content() {
        let mut s = Screen::new(2, 10);
        s.feed(b"primary");
        let _ = s.delta();
        s.feed(b"\x1b[?1049hALT\x1b[?1049l");
        assert_eq!(String::from_utf8_lossy(&s.rows[0][..7]), "primary");
        assert!(s.delta().contains_key(&0));
    }
    #[test]
    fn maximal_cursor_parameters_do_not_overflow() {
        let mut s = Screen::new(2, 10);
        s.feed(b"\x1b[999999999999999999999999999999999999999BX");
        assert_eq!(s.rows[1][0], b'X');
    }
    #[test]
    fn meta_right_is_distinct() {
        assert_ne!(key_bytes("META-RIGHT"), key_bytes("RIGHT"));
    }
}
