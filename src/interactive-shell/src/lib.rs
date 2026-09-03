// MODE: DEV
// PACKAGE: PROD
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::fd::AsRawFd;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const MAX_LINE: usize = 65_536;
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub socket: PathBuf,
    pub cols: u16,
    pub rows: u16,
    pub idle_timeout: u64,
    pub command: Vec<String>,
    #[serde(default)]
    pub agent: String,
}

fn session_root() -> PathBuf {
    if let Ok(root) = std::env::var("INTERACTIVE_SHELL_HOME") {
        return PathBuf::from(root);
    }
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime).join("interactive-shell");
    }
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".interactive-shell")
}

pub fn session_identity(session: Option<&str>, agent: Option<&str>) -> Option<String> {
    session
        .or(agent)
        .map(str::to_owned)
        .or_else(|| std::env::var("INTERACTIVE_SHELL_AGENT").ok())
        .or_else(|| std::env::var("CODEX_AGENT_ID").ok())
        .or_else(|| std::env::var("AGENT_ID").ok())
}

fn validate_session_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err("session id must contain only letters, numbers, '.', '_', or '-'".into());
    }
    Ok(())
}

pub fn session_path(id: &str) -> Result<PathBuf, String> {
    validate_session_id(id)?;
    Ok(session_root().join("sessions").join(format!("{id}.json")))
}

pub fn load_session(id: &str) -> Result<Option<Session>, String> {
    let path = session_path(id)?;
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("read session {}: {error}", path.display())),
    };
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|error| format!("session file {} is malformed: {error}", path.display()))
}

pub fn save_session(id: &str, session: &Session) -> Result<(), String> {
    let path = session_path(id)?;
    let parent = path.parent().ok_or("session path has no parent")?;
    let root = parent.parent().ok_or("session path has no root")?;
    fs::create_dir_all(root).map_err(|error| format!("create session root: {error}"))?;
    fs::set_permissions(root, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("set session root permissions: {error}"))?;
    fs::create_dir_all(parent).map_err(|error| format!("create session directory: {error}"))?;
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("set session directory permissions: {error}"))?;
    let json = serde_json::to_string_pretty(session).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| format!("write session {}: {error}", path.display()))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("set session file permissions: {error}"))
}

pub fn session_socket(id: &str) -> Result<PathBuf, String> {
    validate_session_id(id)?;
    let root = session_root();
    let sockets = root.join("sockets");
    fs::create_dir_all(&sockets).map_err(|error| format!("create session socket root: {error}"))?;
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("set session root permissions: {error}"))?;
    fs::set_permissions(&sockets, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("set session socket root permissions: {error}"))?;
    let dir = sockets.join(id);
    fs::create_dir_all(&dir)
        .map_err(|error| format!("create session socket directory: {error}"))?;
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("set session socket directory permissions: {error}"))?;
    Ok(dir.join("term.sock"))
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
enum Request {
    #[serde(rename = "text")]
    Text { v: u8, text: String },
    #[serde(rename = "key")]
    Key { v: u8, key: String },
    #[serde(rename = "combo")]
    Combo {
        v: u8,
        key: String,
        #[serde(default)]
        ctrl: bool,
        #[serde(default)]
        alt: bool,
        #[serde(default)]
        shift: bool,
    },
    #[serde(rename = "raw")]
    Raw { v: u8, hex: String },
    #[serde(rename = "observe")]
    Observe { v: u8 },
    #[serde(rename = "elements")]
    Elements {
        v: u8,
        #[serde(default)]
        rows: Vec<usize>,
    },
    #[serde(rename = "locate")]
    Locate { v: u8, query: String },
    #[serde(rename = "view")]
    View {
        v: u8,
        #[serde(default)]
        rows: Vec<usize>,
    },
    #[serde(rename = "view-delta")]
    ViewDelta {
        v: u8,
        #[serde(default)]
        rows: Vec<usize>,
    },
    #[serde(rename = "rgbview")]
    RgbView {
        v: u8,
        #[serde(default)]
        rows: Vec<usize>,
    },
    #[serde(rename = "rgbview-delta")]
    RgbViewDelta {
        v: u8,
        #[serde(default)]
        rows: Vec<usize>,
    },
    #[serde(rename = "wait")]
    Wait {
        v: u8,
        contains: String,
        #[serde(default = "default_wait_timeout")]
        timeout_ms: u64,
    },
    #[serde(rename = "paste")]
    Paste { v: u8, text: String },
    #[serde(rename = "mouse")]
    Mouse {
        v: u8,
        x: u16,
        y: u16,
        button: u8,
        action: String,
    },
    #[serde(rename = "click")]
    Click {
        v: u8,
        id: Option<String>,
        label: Option<String>,
        x: Option<u16>,
        y: Option<u16>,
        button: u8,
    },
    #[serde(rename = "resize")]
    Resize { v: u8, cols: u16, rows: u16 },
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
    elements: Vec<Clickable>,
    styles: BTreeMap<usize, Vec<StyleSpan>>,
    scrollback: Vec<String>,
}

#[derive(Serialize)]
struct WaitEvent {
    v: u8,
    event: &'static str,
    matched: bool,
    seq: u64,
    rows: BTreeMap<usize, String>,
    cursor: Cursor,
    elements: Vec<Clickable>,
    styles: BTreeMap<usize, Vec<StyleSpan>>,
    scrollback: Vec<String>,
}

#[derive(Serialize)]
struct ViewEvent {
    v: u8,
    event: &'static str,
    seq: u64,
    cols: usize,
    rows: usize,
    text: String,
}

#[derive(Serialize)]
struct ElementsEvent {
    v: u8,
    event: &'static str,
    seq: u64,
    elements: Vec<Clickable>,
}

#[derive(Serialize)]
struct LocateMatch {
    text: String,
    row: usize,
    col: usize,
    width: usize,
}

#[derive(Serialize)]
struct LocateEvent {
    v: u8,
    event: &'static str,
    seq: u64,
    matches: Vec<LocateMatch>,
}

fn default_wait_timeout() -> u64 {
    30_000
}

#[derive(Clone, Debug, Serialize)]
struct Clickable {
    id: String,
    label: String,
    uri: String,
    actionable: bool,
    highlighted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    highlight_source: Option<&'static str>,
    row: usize,
    col: usize,
    width: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct StyleSpan {
    start: usize,
    width: usize,
    fg: u8,
    bg: u8,
    bold: bool,
    reverse: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CellStyle {
    fg: u8,
    bg: u8,
    bold: bool,
    reverse: bool,
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
    application_cursor: bool,
    wrap_pending: bool,
    saved_cursor: Option<(usize, usize)>,
    line_drawing: bool,
    parser: Parser,
    saved_primary: Option<ScreenState>,
    active_link: Option<String>,
    elements: Vec<Clickable>,
    styles: Vec<Vec<CellStyle>>,
    style: CellStyle,
    scrollback: Vec<String>,
    last_view: Option<Vec<String>>,
    last_rgb_view: Option<Vec<String>>,
    scroll_top: usize,
    scroll_bottom: usize,
}

#[derive(Clone, Debug)]
struct ScreenState {
    rows: Vec<Vec<u8>>,
    row: usize,
    col: usize,
    visible: bool,
    application_cursor: bool,
    wrap_pending: bool,
    saved_cursor: Option<(usize, usize)>,
    styles: Vec<Vec<CellStyle>>,
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
            application_cursor: false,
            wrap_pending: false,
            saved_cursor: None,
            line_drawing: false,
            parser: Parser::Ground,
            saved_primary: None,
            active_link: None,
            elements: Vec::new(),
            styles: vec![
                vec![
                    CellStyle {
                        fg: 0,
                        bg: 0,
                        bold: false,
                        reverse: false
                    };
                    cols
                ];
                rows
            ],
            style: CellStyle {
                fg: 0,
                bg: 0,
                bold: false,
                reverse: false,
            },
            scrollback: Vec::new(),
            last_view: None,
            last_rgb_view: None,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
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
                0x1b => {
                    self.wrap_pending = false;
                    self.parser = Parser::Esc;
                }
                b'\r' => {
                    self.wrap_pending = false;
                    self.col = 0;
                }
                b'\n' => self.newline(),
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
                b'7' => self.saved_cursor = Some((self.row, self.col)),
                b'8' => {
                    if let Some((row, col)) = self.saved_cursor {
                        self.row = row.min(self.rows.len().saturating_sub(1));
                        self.col = col.min(self.rows[0].len().saturating_sub(1));
                    }
                }
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
                    if byte == 7 {
                        self.osc(&value);
                    } else {
                        value.pop();
                        self.osc(&value);
                    }
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
        if self.wrap_pending {
            self.wrap_pending = false;
            self.col = 0;
            self.newline();
        }
        if let Some(row) = self.rows.get_mut(self.row) {
            if self.col < row.len() {
                row[self.col] = byte;
                self.dirty[self.row] = true;
                self.styles[self.row][self.col] = self.style;
            }
        }
        if let Some(uri) = self.active_link.as_ref() {
            if let Some(element) = self.elements.last_mut() {
                if element.uri == *uri
                    && element.row == self.row
                    && element.col + element.width == self.col
                {
                    element.label.push(byte as char);
                    element.width += 1;
                } else {
                    let id = format!("link-{}", self.elements.len() + 1);
                    self.elements.push(Clickable {
                        id,
                        label: (byte as char).to_string(),
                        uri: uri.clone(),
                        actionable: true,
                        highlighted: false,
                        highlight_source: None,
                        row: self.row,
                        col: self.col,
                        width: 1,
                    });
                }
            } else {
                self.elements.push(Clickable {
                    id: "link-1".into(),
                    label: (byte as char).to_string(),
                    uri: uri.clone(),
                    actionable: true,
                    highlighted: false,
                    highlight_source: None,
                    row: self.row,
                    col: self.col,
                    width: 1,
                });
            }
        }
        if self.col + 1 >= self.rows[0].len() {
            self.wrap_pending = true;
        } else {
            self.col += 1;
        }
    }
    fn newline(&mut self) {
        if self.row == self.scroll_bottom {
            self.scroll_up(
                1,
                self.scroll_top == 0 && self.scroll_bottom + 1 == self.rows.len(),
            );
        } else if self.row + 1 >= self.rows.len() {
            let removed = self.rows.remove(0);
            self.scrollback
                .push(String::from_utf8_lossy(&removed).trim_end().to_string());
            if self.scrollback.len() > 1000 {
                self.scrollback.remove(0);
            }
            let cols = self.rows[0].len();
            self.rows.push(vec![b' '; cols]);
            self.styles.remove(0);
            self.styles.push(vec![self.style; cols]);
            self.dirty.fill(true);
        } else {
            self.row += 1;
        }
    }
    fn scroll_up(&mut self, count: usize, record: bool) {
        for _ in 0..count {
            let removed = self.rows.remove(self.scroll_top);
            let cols = removed.len();
            self.rows.insert(self.scroll_bottom, vec![b' '; cols]);
            self.styles.remove(self.scroll_top);
            self.styles
                .insert(self.scroll_bottom, vec![self.style; cols]);
            if record {
                self.scrollback
                    .push(String::from_utf8_lossy(&removed).trim_end().to_string());
                if self.scrollback.len() > 1000 {
                    self.scrollback.remove(0);
                }
            }
        }
        for row in self.scroll_top..=self.scroll_bottom {
            self.dirty[row] = true;
        }
    }
    fn scroll_down(&mut self, count: usize) {
        for _ in 0..count {
            let removed = self.rows.remove(self.scroll_bottom);
            let cols = removed.len();
            self.rows.insert(self.scroll_top, vec![b' '; cols]);
            self.styles.remove(self.scroll_bottom);
            self.styles.insert(self.scroll_top, vec![self.style; cols]);
        }
        for row in self.scroll_top..=self.scroll_bottom {
            self.dirty[row] = true;
        }
    }
    fn insert_lines(&mut self, count: usize) {
        if self.row < self.scroll_top || self.row > self.scroll_bottom {
            return;
        }
        for _ in 0..count.min(self.scroll_bottom - self.row + 1) {
            let cols = self.rows[0].len();
            self.rows.insert(self.row, vec![b' '; cols]);
            self.rows.remove(self.scroll_bottom + 1);
            self.styles.insert(self.row, vec![self.style; cols]);
            self.styles.remove(self.scroll_bottom + 1);
        }
        for row in self.row..=self.scroll_bottom {
            self.dirty[row] = true;
        }
    }
    fn delete_lines(&mut self, count: usize) {
        if self.row < self.scroll_top || self.row > self.scroll_bottom {
            return;
        }
        for _ in 0..count.min(self.scroll_bottom - self.row + 1) {
            let cols = self.rows[0].len();
            self.rows.remove(self.row);
            self.rows.insert(self.scroll_bottom, vec![b' '; cols]);
            self.styles.remove(self.row);
            self.styles
                .insert(self.scroll_bottom, vec![self.style; cols]);
        }
        for row in self.row..=self.scroll_bottom {
            self.dirty[row] = true;
        }
    }
    fn osc(&mut self, value: &[u8]) {
        let value = String::from_utf8_lossy(value);
        let mut fields = value.splitn(3, ';');
        if fields.next() == Some("8") {
            let _params = fields.next();
            self.active_link = fields
                .next()
                .filter(|uri| !uri.is_empty())
                .map(str::to_owned);
        }
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
            b'm' => self.sgr(s.as_ref()),
            b'r' if !private => {
                self.scroll_top = n(0).saturating_sub(1).min(self.rows.len() - 1);
                self.scroll_bottom = n(1).saturating_sub(1).min(self.rows.len() - 1);
                if self.scroll_top >= self.scroll_bottom {
                    self.scroll_top = 0;
                    self.scroll_bottom = self.rows.len() - 1;
                }
                self.row = self.scroll_top;
                self.col = 0;
            }
            b'S' => self.scroll_up(n(0), false),
            b'T' => self.scroll_down(n(0)),
            b'L' => self.insert_lines(n(0)),
            b'M' => self.delete_lines(n(0)),
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
            b's' => self.saved_cursor = Some((self.row, self.col)),
            b'u' => {
                if let Some((row, col)) = self.saved_cursor {
                    self.row = row.min(self.rows.len().saturating_sub(1));
                    self.col = col.min(self.rows[0].len().saturating_sub(1));
                }
            }
            b'J' => self.erase_display(s),
            b'K' => self.erase_line(s),
            b'h' if private && s == "25" => self.visible = true,
            b'l' if private && s == "25" => self.visible = false,
            b'h' if private && s == "1" => self.application_cursor = true,
            b'l' if private && s == "1" => self.application_cursor = false,
            b'h' if private && s == "1049" => self.enter_alt(),
            b'l' if private && s == "1049" => {
                self.leave_alt();
            }
            _ => {}
        }
    }
    fn erase_display(&mut self, mode: &str) {
        let mode = if mode.is_empty() {
            0
        } else {
            mode.parse().unwrap_or(0)
        };
        let (start, end) = match mode {
            0 => (self.row, self.rows.len()),
            1 => (0, self.row + 1),
            2 => (0, self.rows.len()),
            _ => return,
        };
        for row in start..end {
            self.rows[row].fill(b' ');
            self.styles[row].fill(self.style);
            self.dirty[row] = true;
        }
    }
    fn erase_line(&mut self, mode: &str) {
        let mode = if mode.is_empty() {
            0
        } else {
            mode.parse().unwrap_or(0)
        };
        let (start, end) = match mode {
            0 => (self.col, self.rows[self.row].len()),
            1 => (0, self.col + 1),
            2 => (0, self.rows[self.row].len()),
            _ => return,
        };
        self.rows[self.row][start..end].fill(b' ');
        self.styles[self.row][start..end].fill(self.style);
        self.dirty[self.row] = true;
    }
    fn sgr(&mut self, params: &str) {
        for part in params.split(';').map(|p| p.parse::<u8>().unwrap_or(0)) {
            match part {
                0 => {
                    self.style = CellStyle {
                        fg: 0,
                        bg: 0,
                        bold: false,
                        reverse: false,
                    }
                }
                1 => self.style.bold = true,
                22 => self.style.bold = false,
                7 => self.style.reverse = true,
                27 => self.style.reverse = false,
                30..=37 => self.style.fg = part - 29,
                39 => self.style.fg = 0,
                40..=47 => self.style.bg = part - 39,
                49 => self.style.bg = 0,
                _ => {}
            }
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
            application_cursor: self.application_cursor,
            wrap_pending: self.wrap_pending,
            saved_cursor: self.saved_cursor,
            styles: std::mem::take(&mut self.styles),
            line_drawing: self.line_drawing,
        });
        let rows = self.saved_primary.as_ref().unwrap().rows.len();
        let cols = self.saved_primary.as_ref().unwrap().rows[0].len();
        self.rows = vec![vec![b' '; cols]; rows];
        self.styles = vec![vec![self.style; cols]; rows];
        self.dirty = vec![true; rows];
        self.row = 0;
        self.col = 0;
        self.wrap_pending = false;
        self.saved_cursor = None;
    }
    fn leave_alt(&mut self) {
        if let Some(state) = self.saved_primary.take() {
            self.rows = state.rows;
            self.dirty = vec![true; self.rows.len()];
            self.row = state.row;
            self.col = state.col;
            self.visible = state.visible;
            self.application_cursor = state.application_cursor;
            self.wrap_pending = state.wrap_pending;
            self.saved_cursor = state.saved_cursor;
            self.styles = state.styles;
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
    fn snapshot(&self) -> BTreeMap<usize, String> {
        self.rows
            .iter()
            .enumerate()
            .map(|(i, row)| (i, String::from_utf8_lossy(row).trim_end().to_string()))
            .collect()
    }
    fn view(&mut self, delta: bool, requested_rows: &[usize]) -> String {
        let current = self
            .rows
            .iter()
            .map(|row| String::from_utf8_lossy(row).trim_end().to_string())
            .collect::<Vec<_>>();
        let previous = self.last_view.replace(current.clone());
        let requested = if requested_rows.is_empty() {
            (0..current.len()).collect::<Vec<_>>()
        } else {
            requested_rows
                .iter()
                .copied()
                .filter(|row| *row > 0 && *row <= current.len())
                .map(|row| row - 1)
                .collect::<Vec<_>>()
        };
        requested
            .into_iter()
            .filter(|row| {
                !delta
                    || previous
                        .as_ref()
                        .is_none_or(|old| old[*row] != current[*row])
            })
            .map(|row| {
                let text = &current[row];
                let (start, end) = rendered_span(text.as_bytes());
                format!("{line:03} [{start:03}-{end:03}] {text}", line = row + 1,)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn rgbview(&mut self, delta: bool, requested_rows: &[usize]) -> String {
        let current = self
            .rows
            .iter()
            .enumerate()
            .map(|(row, cells)| {
                let end = cells
                    .iter()
                    .rposition(|cell| *cell != b' ')
                    .map_or(0, |index| index + 1);
                let (start, end) = rendered_span(&cells[..end]);
                let mut text = format!("{line:03} [{start:03}-{end:03}] ", line = row + 1);
                let mut previous = CellStyle {
                    fg: 0,
                    bg: 0,
                    bold: false,
                    reverse: false,
                };
                for (cell, style) in cells.iter().zip(&self.styles[row]).take(end) {
                    if *style != previous {
                        text.push_str(&style_sgr(*style));
                        previous = *style;
                    }
                    text.push(*cell as char);
                }
                if previous.fg != 0 || previous.bg != 0 || previous.bold || previous.reverse {
                    text.push_str("\x1b[0m");
                }
                text
            })
            .collect::<Vec<_>>();
        let previous = self.last_rgb_view.replace(current.clone());
        let requested = if requested_rows.is_empty() {
            (0..current.len()).collect::<Vec<_>>()
        } else {
            requested_rows
                .iter()
                .copied()
                .filter(|row| *row > 0 && *row <= current.len())
                .map(|row| row - 1)
                .collect::<Vec<_>>()
        };
        requested
            .into_iter()
            .filter(|row| {
                !delta
                    || previous
                        .as_ref()
                        .is_none_or(|old| old[*row] != current[*row])
            })
            .map(|row| current[row].clone())
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn elements(&self) -> Vec<Clickable> {
        let mut elements = self
            .elements
            .iter()
            .filter(|element| {
                self.rows
                    .get(element.row)
                    .and_then(|row| row.get(element.col..element.col.saturating_add(element.width)))
                    .is_some_and(|cells| cells == element.label.as_bytes())
            })
            .cloned()
            .collect::<Vec<_>>();
        for (row, cells) in self.rows.iter().enumerate() {
            let mut col = 0;
            while col < cells.len() {
                while col < cells.len() && cells[col].is_ascii_whitespace() {
                    col += 1;
                }
                let start = col;
                while col < cells.len() && !cells[col].is_ascii_whitespace() {
                    col += 1;
                }
                if col > start + 1 {
                    let label = String::from_utf8_lossy(&cells[start..col]).into_owned();
                    elements.push(Clickable {
                        id: format!("text-{row}-{start}"),
                        label,
                        uri: "terminal://visible-text".into(),
                        actionable: false,
                        highlighted: false,
                        highlight_source: None,
                        row,
                        col: start,
                        width: col - start,
                    });
                }
            }
        }
        for element in &mut elements {
            let reverse = self.styles[element.row]
                .get(element.col..element.col.saturating_add(element.width))
                .is_some_and(|styles| styles.iter().any(|style| style.reverse));
            let color_outlier = self.color_outlier(element.row, element.col, element.width);
            element.highlighted = reverse || color_outlier;
            element.highlight_source = if reverse {
                Some("reverse-video")
            } else if color_outlier {
                Some("color-outlier")
            } else {
                None
            };
        }
        elements
    }
    fn color_outlier(&self, row: usize, col: usize, width: usize) -> bool {
        if row == 0 || row + 1 >= self.styles.len() {
            return false;
        }
        let Some(current) = self.styles[row].get(col..col.saturating_add(width)) else {
            return false;
        };
        let Some(above) = self.styles[row - 1].get(col..col.saturating_add(width)) else {
            return false;
        };
        let Some(below) = self.styles[row + 1].get(col..col.saturating_add(width)) else {
            return false;
        };
        let comparable = current
            .iter()
            .zip(above.iter().zip(below.iter()))
            .filter(|(_, (above, below))| above == below)
            .count();
        let different = current
            .iter()
            .zip(above.iter().zip(below.iter()))
            .filter(|(current, (above, below))| above == below && *current != *above)
            .count();
        comparable >= 2 && different * 2 >= comparable
    }
    fn styles(&self) -> BTreeMap<usize, Vec<StyleSpan>> {
        let plain = CellStyle {
            fg: 0,
            bg: 0,
            bold: false,
            reverse: false,
        };
        self.styles
            .iter()
            .enumerate()
            .filter_map(|(row, cells)| {
                let mut spans = Vec::new();
                let mut start = 0;
                while start < cells.len() {
                    let style = cells[start];
                    let mut end = start + 1;
                    while end < cells.len() && cells[end] == style {
                        end += 1;
                    }
                    if style != plain {
                        spans.push(StyleSpan {
                            start,
                            width: end - start,
                            fg: style.fg,
                            bg: style.bg,
                            bold: style.bold,
                            reverse: style.reverse,
                        });
                    }
                    start = end;
                }
                (!spans.is_empty()).then_some((row, spans))
            })
            .collect()
    }
    fn scrollback(&self) -> Vec<String> {
        self.scrollback.clone()
    }
    fn resize(&mut self, rows: usize, cols: usize) {
        self.rows.resize_with(rows, || vec![b' '; cols]);
        for row in &mut self.rows {
            row.resize(cols, b' ');
        }
        self.dirty = vec![true; rows];
        self.styles.resize_with(rows, || vec![self.style; cols]);
        for row in &mut self.styles {
            row.resize(cols, self.style);
        }
        self.row = self.row.min(rows.saturating_sub(1));
        self.col = self.col.min(cols.saturating_sub(1));
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);
        self.wrap_pending = false;
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

fn rendered_span(row: &[u8]) -> (usize, usize) {
    let Some(start) = row.iter().position(|cell| *cell != b' ') else {
        return (0, 0);
    };
    let end = row.iter().rposition(|cell| *cell != b' ').unwrap_or(start);
    (start + 1, end + 1)
}

fn style_sgr(style: CellStyle) -> String {
    let mut codes = vec!["0".to_owned()];
    if style.bold {
        codes.push("1".to_owned());
    }
    if style.reverse {
        codes.push("7".to_owned());
    }
    if (1..=8).contains(&style.fg) {
        codes.push((29 + style.fg).to_string());
    }
    if (1..=8).contains(&style.bg) {
        codes.push((39 + style.bg).to_string());
    }
    format!("\x1b[{}m", codes.join(";"))
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
        "CTRL-A" => Some(b"\x01"),
        "CTRL-B" => Some(b"\x02"),
        "CTRL-F" => Some(b"\x06"),
        "CTRL-G" => Some(b"\x07"),
        "CTRL-H" => Some(b"\x08"),
        "CTRL-I" => Some(b"\x09"),
        "CTRL-J" => Some(b"\x0a"),
        "CTRL-K" => Some(b"\x0b"),
        "CTRL-L" => Some(b"\x0c"),
        "CTRL-M" => Some(b"\x0d"),
        "CTRL-N" => Some(b"\x0e"),
        "CTRL-P" => Some(b"\x10"),
        "CTRL-Q" => Some(b"\x11"),
        "CTRL-R" => Some(b"\x12"),
        "CTRL-S" => Some(b"\x13"),
        "CTRL-T" => Some(b"\x14"),
        "CTRL-U" => Some(b"\x15"),
        "CTRL-V" => Some(b"\x16"),
        "CTRL-W" => Some(b"\x17"),
        "CTRL-Y" => Some(b"\x19"),
        "BACKSPACE" => Some(b"\x7f"),
        "TAB" => Some(b"\t"),
        "ESC" => Some(b"\x1b"),
        "META-RIGHT" => Some(b"\x1b[1;3C"),
        "META-LEFT" => Some(b"\x1b[1;3D"),
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
        "CTRL-PAGEUP" => Some(b"\x1b[5;5~"),
        "CTRL-PAGEDOWN" => Some(b"\x1b[6;5~"),
        "CTRL-INSERT" => Some(b"\x1b[2;5~"),
        "CTRL-DELETE" => Some(b"\x1b[3;5~"),
        "F1" => Some(b"\x1bOP"),
        "F2" => Some(b"\x1bOQ"),
        "F3" => Some(b"\x1bOR"),
        "F4" => Some(b"\x1bOS"),
        "F5" => Some(b"\x1b[15~"),
        "F6" => Some(b"\x1b[17~"),
        "F7" => Some(b"\x1b[18~"),
        "F8" => Some(b"\x1b[19~"),
        "F9" => Some(b"\x1b[20~"),
        "F10" => Some(b"\x1b[21~"),
        "F11" => Some(b"\x1b[23~"),
        "F12" => Some(b"\x1b[24~"),
        "SHIFT-UP" => Some(b"\x1b[1;2A"),
        "SHIFT-DOWN" => Some(b"\x1b[1;2B"),
        "SHIFT-LEFT" => Some(b"\x1b[1;2D"),
        "SHIFT-RIGHT" => Some(b"\x1b[1;2C"),
        "CTRL-UP" => Some(b"\x1b[1;5A"),
        "CTRL-DOWN" => Some(b"\x1b[1;5B"),
        "CTRL-LEFT" => Some(b"\x1b[1;5D"),
        "CTRL-RIGHT" => Some(b"\x1b[1;5C"),
        _ => None,
    }
}

pub fn key_sequence(key: &str) -> Option<Vec<u8>> {
    if let Some(bytes) = key_bytes(key) {
        return Some(bytes.to_vec());
    }
    let normalized = key.to_ascii_uppercase();
    if normalized != key {
        if let Some(bytes) = key_bytes(&normalized) {
            return Some(bytes.to_vec());
        }
    }
    if key.len() == 1 && key.as_bytes()[0].is_ascii_graphic() {
        return Some(key.as_bytes().to_vec());
    }
    if let Some(letter) = key.strip_prefix("ALT-") {
        let bytes = letter.as_bytes();
        if bytes.len() == 1 && bytes[0].is_ascii_graphic() {
            return Some(vec![0x1b, bytes[0]]);
        }
    }
    None
}

fn key_sequence_for_screen(key: &str, application_cursor: bool) -> Option<Vec<u8>> {
    if application_cursor {
        let sequence = match key.to_ascii_uppercase().as_str() {
            "UP" => Some(b"\x1bOA".as_slice()),
            "DOWN" => Some(b"\x1bOB".as_slice()),
            "RIGHT" => Some(b"\x1bOC".as_slice()),
            "LEFT" => Some(b"\x1bOD".as_slice()),
            _ => None,
        };
        if let Some(bytes) = sequence {
            return Some(bytes.to_vec());
        }
    }
    key_sequence(key)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

pub fn key_combo_sequence(key: &str, modifiers: KeyModifiers) -> Result<Vec<u8>, &'static str> {
    if !modifiers.ctrl && !modifiers.alt && !modifiers.shift {
        return key_sequence(key).ok_or("unknown key");
    }
    let upper = key.to_ascii_uppercase();
    let named = match upper.as_str() {
        "UP" => Some("A"),
        "DOWN" => Some("B"),
        "RIGHT" => Some("C"),
        "LEFT" => Some("D"),
        "HOME" => Some("H"),
        "END" => Some("F"),
        _ => None,
    };
    if let Some(final_byte) = named {
        let modifier = 1
            + u8::from(modifiers.shift)
            + 2 * u8::from(modifiers.alt)
            + 4 * u8::from(modifiers.ctrl);
        return Ok(format!("\x1b[1;{modifier}{final_byte}").into_bytes());
    }
    let tilde_key = match upper.as_str() {
        "INSERT" => Some("2~"),
        "DELETE" => Some("3~"),
        "PAGEUP" => Some("5~"),
        "PAGEDOWN" => Some("6~"),
        _ => None,
    };
    if let Some(base) = tilde_key {
        let (number, suffix) = base.split_at(base.len() - 1);
        let modifier = 1
            + u8::from(modifiers.shift)
            + 2 * u8::from(modifiers.alt)
            + 4 * u8::from(modifiers.ctrl);
        return Ok(format!("\x1b[{number};{modifier}{suffix}").into_bytes());
    }
    if let Some(function) = upper.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()) {
        if (1..=12).contains(&function) {
            let base = match function {
                1 => "P",
                2 => "Q",
                3 => "R",
                4 => "S",
                5 => "15~",
                6 => "17~",
                7 => "18~",
                8 => "19~",
                9 => "20~",
                10 => "21~",
                11 => "23~",
                12 => "24~",
                _ => unreachable!(),
            };
            let modifier = 1
                + u8::from(modifiers.shift)
                + 2 * u8::from(modifiers.alt)
                + 4 * u8::from(modifiers.ctrl);
            if function <= 4 {
                return Ok(format!("\x1b[1;{modifier}{base}").into_bytes());
            }
            let (number, suffix) = base.split_at(base.len() - 1);
            return Ok(format!("\x1b[{number};{modifier}{suffix}").into_bytes());
        }
    }
    let bytes = key.as_bytes();
    if bytes.len() != 1 || !bytes[0].is_ascii_graphic() {
        return Err("combination requires one printable ASCII key or a supported named key");
    }
    let mut byte = bytes[0];
    if modifiers.shift && byte.is_ascii_lowercase() {
        byte = byte.to_ascii_uppercase();
    }
    if modifiers.ctrl {
        let control = match byte.to_ascii_uppercase() {
            b'@'..=b'_' => byte.to_ascii_uppercase() - b'@',
            b' ' => 0,
            _ => return Err("Ctrl combination is unsupported for this key"),
        };
        byte = control;
    }
    let mut result = Vec::with_capacity(2);
    if modifiers.alt {
        result.push(0x1b);
    }
    result.push(byte);
    Ok(result)
}
fn decode_hex(s: &str) -> Result<Vec<u8>, &'static str> {
    if s.is_empty() || !s.len().is_multiple_of(2) || !s.is_ascii() {
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
struct SocketIdentity {
    parent: File,
    name: CString,
    device: u64,
    inode: u64,
}

fn remove_socket(identity: &SocketIdentity) {
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    let same_entry = unsafe {
        libc::fstatat(
            identity.parent.as_raw_fd(),
            identity.name.as_ptr(),
            &mut stat,
            libc::AT_SYMLINK_NOFOLLOW,
        ) == 0
            && stat.st_dev == identity.device
            && stat.st_ino == identity.inode
    };
    if same_entry {
        unsafe {
            libc::unlinkat(identity.parent.as_raw_fd(), identity.name.as_ptr(), 0);
        }
    }
}

#[cfg(target_os = "linux")]
fn fd_path(fd: RawFd) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{fd}"))
}

#[cfg(target_os = "macos")]
fn fd_path(fd: RawFd) -> PathBuf {
    PathBuf::from(format!("/dev/fd/{fd}"))
}

fn capture_socket_identity(path: &Path, parent_fd: File) -> Result<SocketIdentity, String> {
    let name = path.file_name().ok_or("socket needs a filename")?;
    let name_c = CString::new(name.as_bytes()).map_err(|e| e.to_string())?;
    let fd = parent_fd.as_raw_fd();
    loop {
        let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
        let result =
            unsafe { libc::fstatat(fd, name_c.as_ptr(), &mut stat, libc::AT_SYMLINK_NOFOLLOW) };
        if result == 0 {
            let mode = stat.st_mode as libc::mode_t;
            if mode & libc::S_IFMT != libc::S_IFSOCK {
                return Err("bound socket entry is not a socket".into());
            }
            return Ok(SocketIdentity {
                parent: parent_fd,
                name: name_c,
                device: stat.st_dev,
                inode: stat.st_ino,
            });
        }
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::NotFound {
            return Err("bound socket disappeared before identity capture".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

struct SocketGuard {
    identity: Option<SocketIdentity>,
}

impl SocketGuard {
    fn new() -> Self {
        Self { identity: None }
    }
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        if let Some(identity) = self.identity.as_ref() {
            remove_socket(identity);
        }
    }
}

extern "C" fn interrupt_handler(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

struct Cleanup {
    master: RawFd,
    pid: libc::pid_t,
    identity: SocketIdentity,
    reaped: bool,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        if !self.reaped {
            stop(self.pid);
        }
        unsafe {
            libc::close(self.master);
        }
        remove_socket(&self.identity);
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
#[allow(clippy::too_many_arguments)]
fn client(
    mut stream: UnixStream,
    master: RawFd,
    screen: &mut Screen,
    seq: &mut u64,
    out: &mut impl Write,
    last: &mut Instant,
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
        Request::Key { v: 1, key } => {
            match key_sequence_for_screen(&key, screen.application_cursor) {
                Some(bytes) => write_master(master, &bytes)?,
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
            }
        }
        Request::Combo {
            v: 1,
            key,
            ctrl,
            alt,
            shift,
        } => match key_combo_sequence(&key, KeyModifiers { ctrl, alt, shift }) {
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
        Request::Observe { v: 1 } => {
            json(
                &mut stream,
                &ScreenEvent {
                    v: 1,
                    event: "snapshot",
                    seq: *seq,
                    base: *seq,
                    rows: screen.snapshot(),
                    cursor: screen.cursor(),
                    elements: screen.elements(),
                    styles: screen.styles(),
                    scrollback: screen.scrollback(),
                },
            )
            .map_err(|e| e.to_string())?;
        }
        Request::Elements { v: 1, rows } => {
            let elements = screen
                .elements()
                .into_iter()
                .filter(|element| element.actionable)
                .filter(|element| {
                    rows.is_empty()
                        || rows
                            .iter()
                            .any(|requested| *requested == element.row.saturating_add(1))
                })
                .collect();
            json(
                &mut stream,
                &ElementsEvent {
                    v: 1,
                    event: "elements",
                    seq: *seq,
                    elements,
                },
            )
            .map_err(|e| e.to_string())?;
        }
        Request::Locate { v: 1, query } => {
            if query.is_empty() {
                return Err("locate query must not be empty".into());
            }
            let matches = screen
                .rows
                .iter()
                .enumerate()
                .flat_map(|(row, cells)| {
                    let line = String::from_utf8_lossy(cells).into_owned();
                    line.match_indices(&query)
                        .map(|(col, text)| LocateMatch {
                            text: text.to_owned(),
                            row: row + 1,
                            col: line[..col].chars().count() + 1,
                            width: text.chars().count(),
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            json(
                &mut stream,
                &LocateEvent {
                    v: 1,
                    event: "locate",
                    seq: *seq,
                    matches,
                },
            )
            .map_err(|e| e.to_string())?;
        }
        Request::View { v: 1, rows } => {
            json(
                &mut stream,
                &ViewEvent {
                    v: 1,
                    event: "view",
                    seq: *seq,
                    cols: screen.rows.first().map_or(0, Vec::len),
                    rows: screen.rows.len(),
                    text: screen.view(false, &rows),
                },
            )
            .map_err(|e| e.to_string())?;
        }
        Request::ViewDelta { v: 1, rows } => {
            json(
                &mut stream,
                &ViewEvent {
                    v: 1,
                    event: "view",
                    seq: *seq,
                    cols: screen.rows.first().map_or(0, Vec::len),
                    rows: screen.rows.len(),
                    text: screen.view(true, &rows),
                },
            )
            .map_err(|e| e.to_string())?;
        }
        Request::RgbView { v: 1, rows } => {
            json(
                &mut stream,
                &ViewEvent {
                    v: 1,
                    event: "view",
                    seq: *seq,
                    cols: screen.rows.first().map_or(0, Vec::len),
                    rows: screen.rows.len(),
                    text: screen.rgbview(false, &rows),
                },
            )
            .map_err(|e| e.to_string())?;
        }
        Request::RgbViewDelta { v: 1, rows } => {
            json(
                &mut stream,
                &ViewEvent {
                    v: 1,
                    event: "view",
                    seq: *seq,
                    cols: screen.rows.first().map_or(0, Vec::len),
                    rows: screen.rows.len(),
                    text: screen.rgbview(true, &rows),
                },
            )
            .map_err(|e| e.to_string())?;
        }
        Request::Wait {
            v: 1,
            contains,
            timeout_ms,
        } => {
            let deadline = Instant::now() + Duration::from_millis(timeout_ms.min(300_000));
            let mut matched = screen
                .snapshot()
                .values()
                .any(|row| row.contains(&contains));
            while !matched && Instant::now() < deadline {
                let mut buf = [0; 8192];
                let n = unsafe { libc::read(master, buf.as_mut_ptr().cast(), buf.len()) };
                if n > 0 {
                    *last = Instant::now();
                    screen.feed(&buf[..n as usize]);
                    *seq += 1;
                    let event = ScreenEvent {
                        v: 1,
                        event: "screen",
                        seq: *seq,
                        base: *seq - 1,
                        rows: screen.delta(),
                        cursor: screen.cursor(),
                        elements: screen.elements(),
                        styles: screen.styles(),
                        scrollback: screen.scrollback(),
                    };
                    json(out, &event).map_err(|e| e.to_string())?;
                    json(&mut stream, &event).map_err(|e| e.to_string())?;
                    matched = screen
                        .snapshot()
                        .values()
                        .any(|row| row.contains(&contains));
                } else if n < 0 && io::Error::last_os_error().kind() != io::ErrorKind::WouldBlock {
                    break;
                } else {
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
            json(
                &mut stream,
                &WaitEvent {
                    v: 1,
                    event: "wait",
                    matched,
                    seq: *seq,
                    rows: screen.snapshot(),
                    cursor: screen.cursor(),
                    elements: screen.elements(),
                    styles: screen.styles(),
                    scrollback: screen.scrollback(),
                },
            )
            .map_err(|e| e.to_string())?;
        }
        Request::Paste { v: 1, text } => {
            write_master(master, b"\x1b[200~")?;
            write_master(master, text.as_bytes())?;
            write_master(master, b"\x1b[201~")?;
        }
        Request::Mouse {
            v: 1,
            x,
            y,
            button,
            action,
        } => write_master(master, &mouse_bytes(x, y, button, &action)?)?,
        Request::Click {
            v: 1,
            id,
            label,
            x,
            y,
            button,
        } => {
            let target = screen.elements().into_iter().find(|element| {
                id.as_deref() == Some(element.id.as_str())
                    || label.as_deref() == Some(element.label.as_str())
            });
            let (x, y) = match target {
                Some(element) if !element.actionable => {
                    return Err("target is visible text, not a verified clickable element; use click-at for a deliberate coordinate click".into());
                }
                Some(element) => (element.col as u16 + 1, element.row as u16 + 1),
                None => match (x, y) {
                    (Some(x), Some(y)) => (x, y),
                    _ => return Err("click needs a known id/label or x and y".into()),
                },
            };
            write_master(master, &mouse_bytes(x, y, button, "down")?)?;
            write_master(master, &mouse_bytes(x, y, button, "up")?)?;
        }
        Request::Resize { v: 1, cols, rows } => {
            resize_master(master, cols, rows)?;
            screen.resize(rows as usize, cols as usize);
        }
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

fn resize_master(fd: RawFd, cols: u16, rows: u16) -> Result<(), String> {
    if !(1..=240).contains(&cols) || !(1..=100).contains(&rows) {
        return Err("dimensions must be within cols 1..240 and rows 1..100".into());
    }
    let size = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    if unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &size) } < 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    Ok(())
}

fn mouse_bytes(x: u16, y: u16, button: u8, action: &str) -> Result<Vec<u8>, String> {
    if x == 0 || y == 0 || button > 7 {
        return Err("mouse coordinates are 1-based and button must be 0..7".into());
    }
    let suffix = match action {
        "down" => 'M',
        "up" => 'm',
        "move" => 'M',
        _ => return Err("mouse action must be down, up, or move".into()),
    };
    let code = if action == "move" {
        button | 32
    } else {
        button
    };
    Ok(format!("\x1b[<{};{};{}{}", code, x, y, suffix).into_bytes())
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
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
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
    let parent_fd = File::open(socket.parent().unwrap()).map_err(|e| e.to_string())?;
    if socket.exists() {
        return Err("refusing existing socket".into());
    };
    let name = socket.file_name().ok_or("socket needs a filename")?;
    let bind_path = fd_path(parent_fd.as_raw_fd()).join(name);
    let old_umask = unsafe { libc::umask(0o177) };
    let listener_result = UnixListener::bind(&bind_path);
    unsafe {
        libc::umask(old_umask);
    }
    let listener = listener_result.map_err(|e| e.to_string())?;
    let mut socket_guard = SocketGuard::new();
    socket_guard.identity = Some(capture_socket_identity(&socket, parent_fd)?);
    if let Err(error) = listener.set_nonblocking(true) {
        remove_socket(socket_guard.identity.as_ref().unwrap());
        return Err(error.to_string());
    }
    let (master, pid) = match spawn(&command, cols, rows) {
        Ok(x) => x,
        Err(e) => {
            remove_socket(socket_guard.identity.as_ref().unwrap());
            return Err(e);
        }
    };
    let mut cleanup = Cleanup {
        master,
        pid,
        identity: socket_guard.identity.take().unwrap(),
        reaped: false,
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
                    elements: screen.elements(),
                    styles: screen.styles(),
                    scrollback: screen.scrollback(),
                },
            )
            .map_err(|e| e.to_string())?;
        }
        if let Ok((s, _)) = listener.accept() {
            if let Err(e) = client(
                s,
                master,
                &mut screen,
                &mut seq,
                &mut out,
                &mut last,
                &mut stopped,
                &mut reason,
            ) {
                eprintln!("interactive-shell client: {e}");
            }
        }
        let mut st = 0;
        if unsafe { libc::waitpid(pid, &mut st, libc::WNOHANG) } == pid {
            stopped = true;
            code = status(st);
            cleanup.reaped = true;
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
            "ENTER",
            "CTRL-X",
            "UP",
            "DOWN",
            "LEFT",
            "RIGHT",
            "HOME",
            "END",
            "META-LEFT",
            "PAGEUP",
            "PAGEDOWN",
            "INSERT",
            "DELETE",
            "CTRL-PAGEUP",
            "CTRL-PAGEDOWN",
            "CTRL-INSERT",
            "CTRL-DELETE",
            "F1",
            "F2",
            "F3",
            "F4",
            "F5",
            "F6",
            "F7",
            "F8",
            "F9",
            "F10",
            "F11",
            "F12",
            "CTRL-A",
            "CTRL-K",
            "CTRL-S",
            "SHIFT-UP",
            "CTRL-RIGHT",
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
    fn printable_output_wraps_at_the_terminal_width() {
        let mut screen = Screen::new(2, 4);
        screen.feed(b"abcde");
        assert_eq!(String::from_utf8_lossy(&screen.rows[0]), "abcd");
        assert_eq!(String::from_utf8_lossy(&screen.rows[1]), "e   ");
        assert_eq!(screen.col, 1);
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
        s.feed(b"\x1b[31mprimary");
        let _ = s.delta();
        s.feed(b"\x1b[?1049hALT\x1b[?1049l");
        assert_eq!(String::from_utf8_lossy(&s.rows[0][..7]), "primary");
        assert_eq!(s.styles[0][0].fg, 2);
        assert!(s.delta().contains_key(&0));
    }
    #[test]
    fn erase_sequences_clear_text_and_styles_by_mode() {
        let mut s = Screen::new(2, 6);
        s.feed(b"\x1b[31mabcdef\x1b[2K");
        assert_eq!(String::from_utf8_lossy(&s.rows[0]), "      ");
        assert_eq!(s.styles[0][0].fg, 2);
        s.feed(b"\x1b[1Gabcdef\x1b[1G\x1b[K");
        assert_eq!(String::from_utf8_lossy(&s.rows[0]), "      ");
    }
    #[test]
    fn cursor_save_and_restore_sequences_round_trip() {
        let mut screen = Screen::new(3, 8);
        screen.feed(b"\x1b[2;3H\x1b[s\x1b[3;5H\x1b[uX");
        assert_eq!(screen.row, 1);
        assert_eq!(screen.col, 3);
        assert_eq!(screen.rows[1][2], b'X');
        screen.feed(b"\x1b7\x1b[1;1H\x1b8Y");
        assert_eq!(screen.rows[1][3], b'Y');
    }
    #[test]
    fn visible_text_reports_reverse_video_as_a_highlight_hint() {
        let mut screen = Screen::new(1, 12);
        screen.feed(b"\x1b[7mSELECT\x1b[27m");
        let element = screen
            .elements()
            .into_iter()
            .find(|element| element.label == "SELECT")
            .unwrap();
        assert!(!element.actionable);
        assert!(element.highlighted);
        assert_eq!(element.highlight_source, Some("reverse-video"));
    }
    #[test]
    fn compact_view_reports_the_actual_rendered_columns() {
        let mut screen = Screen::new(1, 12);
        screen.feed(b"  PAD");
        assert_eq!(screen.view(false, &[]), "001 [003-005]   PAD");
    }
    #[test]
    fn visible_text_reports_a_color_outlier_as_a_selection_hint() {
        let mut screen = Screen::new(3, 12);
        screen.feed(
            b"\x1b[31;44mNORMAL\x1b[0m\r\n\x1b[32;45mSELECT\x1b[0m\r\n\x1b[31;44mNORMAL\x1b[0m",
        );
        let element = screen
            .elements()
            .into_iter()
            .find(|element| element.label == "SELECT")
            .unwrap();
        assert!(element.highlighted);
        assert_eq!(element.highlight_source, Some("color-outlier"));
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
    #[test]
    fn printable_keys_are_accepted() {
        assert_eq!(key_sequence("Q"), Some(vec![b'Q']));
        assert_eq!(key_sequence("q"), Some(vec![b'q']));
        assert_eq!(key_sequence("enter"), Some(vec![b'\r']));
        assert_eq!(key_sequence("up"), Some(b"\x1b[A".to_vec()));
    }
    #[test]
    fn application_cursor_mode_uses_ss3_cursor_sequences() {
        let mut screen = Screen::new(2, 10);
        screen.feed(b"\x1b[?1h");
        assert!(screen.application_cursor);
        assert_eq!(
            key_sequence_for_screen("up", screen.application_cursor),
            Some(b"\x1bOA".to_vec())
        );
        screen.feed(b"\x1b[?1l");
        assert!(!screen.application_cursor);
        assert_eq!(
            key_sequence_for_screen("up", screen.application_cursor),
            Some(b"\x1b[A".to_vec())
        );
    }
    #[test]
    fn styles_and_scrollback_are_retained() {
        let mut s = Screen::new(2, 4);
        s.feed(b"\x1b[31;1mR\x1b[0ma\nb");
        let styles = s.styles();
        assert_eq!(
            styles[&0][0],
            StyleSpan {
                start: 0,
                width: 1,
                fg: 2,
                bg: 0,
                bold: true,
                reverse: false
            }
        );
        s.feed(b"\nc");
        assert_eq!(s.scrollback, vec!["Ra"]);
        assert_eq!(String::from_utf8_lossy(&s.rows[0]), "  b ");
        assert_eq!(String::from_utf8_lossy(&s.rows[1]), "   c");
    }
    #[test]
    fn alt_graphic_keys_are_encoded_without_raw_bytes() {
        assert_eq!(key_sequence("ALT-X"), Some(vec![0x1b, b'X']));
        assert_eq!(key_sequence("ALT-LEFT"), None);
        assert_eq!(key_sequence("META-LEFT"), Some(b"\x1b[1;3D".to_vec()));
        assert_eq!(key_sequence("F10"), Some(b"\x1b[21~".to_vec()));
        assert_eq!(key_sequence("CTRL-DELETE"), Some(b"\x1b[3;5~".to_vec()));
        // "éé" does NOT discriminate: it is 4 bytes, both two-byte slices land
        // on char boundaries, and from_str_radix rejects them anyway, so the
        // assertion held with or without the is_ascii guard it was meant to
        // pin. A 3-byte char followed by an ASCII byte is the case that panics
        // ("end byte index 2 is not a char boundary") once the guard is gone.
        assert!(decode_hex("éé").is_err());
        assert!(decode_hex("€x").is_err());
    }

    #[test]
    fn arbitrary_modifier_combinations_are_encoded() {
        assert_eq!(
            key_combo_sequence(
                "h",
                KeyModifiers {
                    ctrl: true,
                    alt: true,
                    shift: true
                }
            ),
            Ok(vec![0x1b, 0x08])
        );
        assert_eq!(
            key_combo_sequence(
                "RIGHT",
                KeyModifiers {
                    ctrl: true,
                    alt: true,
                    shift: true
                }
            ),
            Ok(b"\x1b[1;8C".to_vec())
        );
        assert_eq!(
            key_combo_sequence(
                "PAGEUP",
                KeyModifiers {
                    ctrl: true,
                    alt: false,
                    shift: false
                }
            ),
            Ok(b"\x1b[5;5~".to_vec())
        );
        assert_eq!(
            key_combo_sequence(
                "PAGEDOWN",
                KeyModifiers {
                    ctrl: true,
                    alt: false,
                    shift: false
                }
            ),
            Ok(b"\x1b[6;5~".to_vec())
        );
    }
}
