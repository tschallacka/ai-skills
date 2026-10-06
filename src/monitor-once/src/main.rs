// MODE: DEV
// PACKAGE: PROD
//! `monitor-once`: run a monitor command only when no live monitor of the same name
//! already holds its lock. A monitor that is still running keeps its lock, so a second
//! start is refused instead of stacking up one more copy of the same watch.
//!
//! The lock is `<dir>/<name>.pid`, three lines: the monitor's pid, the monitor's start
//! identity (so a reused pid is not mistaken for it), and the process group of the
//! watched command (0 until it has started). The lock is live while the monitor or that
//! group still exists.
//!
//! The watched command runs in its own process group. SIGTERM, SIGINT and SIGHUP sent to
//! the monitor are forwarded to the whole group, so a signalled monitor does not leave its
//! watch running unlocked. SIGKILL cannot be forwarded; the surviving group keeps the lock
//! live in that case, so a second start is still refused. The lock is created atomically
//! (a complete file is hard-linked into place), so two simultaneous starts cannot both win.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::Duration;

const PROGRAM: &str = "monitor-once";

/// `EX_TEMPFAIL` from sysexits.h: the monitor is already running, try again later.
const ALREADY_RUNNING: u8 = 75;
/// `EX_USAGE` from sysexits.h.
const USAGE_ERROR: u8 = 64;
/// How often the launcher checks on the watched command and forwards a pending signal.
const POLL: Duration = Duration::from_millis(100);

const USAGE: &str = "monitor-once - run a monitor only if no live one of the same name exists

Usage:
  monitor-once --name NAME [--nick NICK] --dir DIR -- COMMAND [ARGS...]
  monitor-once --help

  --name NAME   the monitor's name; one live monitor per name (letters, digits, - and _)
  --nick NICK   the agent's chat nick: the lock is per name and nick, so each agent runs
                its own watch of NAME; a nick must never be shared between sessions
  --dir DIR     where the lock file lives: <NAME>.pid, or <NAME>.<NICK>.pid (created if missing)
  --help        print this

Exit status: the monitor's own (128+N when it was stopped by signal N), 75 when a live
monitor of that name already holds the lock (nothing is started), or 64 for a bad
invocation.
";

#[derive(Debug, PartialEq)]
struct Args {
    name: String,
    /// The agent's chat nick, when given: the lock is then per agent, so two agents (or a
    /// subworker with its own nick) may each run a watch of the same name.
    nick: Option<String>,
    dir: PathBuf,
    command: Vec<String>,
}

fn parse(args: &[String]) -> Result<Option<Args>, String> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Ok(None);
    }
    let mut name = None;
    let mut nick = None;
    let mut dir = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--name" => name = Some(rest.next().ok_or("--name needs a value")?.clone()),
            "--nick" => nick = Some(rest.next().ok_or("--nick needs a value")?.clone()),
            "--dir" => dir = Some(PathBuf::from(rest.next().ok_or("--dir needs a value")?)),
            "--" => {
                let command: Vec<String> = rest.cloned().collect();
                if command.is_empty() {
                    return Err("nothing to run after --".into());
                }
                let name = name.ok_or("--name is required")?;
                if !valid_name(&name) {
                    return Err(format!("invalid name {name:?}: use letters, digits, - and _"));
                }
                if let Some(nick) = &nick {
                    if !valid_name(nick) {
                        return Err(format!("invalid nick {nick:?}: use letters, digits, - and _"));
                    }
                }
                let dir = dir.ok_or("--dir is required")?;
                return Ok(Some(Args { name, nick, dir, command }));
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Err("expected -- COMMAND after the options".into())
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// What the lock file says about who holds it.
#[derive(Debug, Clone, PartialEq)]
struct Holder {
    pid: i32,
    identity: Option<String>,
    group: i32,
}

fn render_holder(holder: &Holder) -> String {
    format!(
        "{}\n{}\n{}\n",
        holder.pid,
        holder.identity.as_deref().unwrap_or(""),
        holder.group
    )
}

/// `None` when the file is not a lock this tool wrote (empty, truncated or foreign).
fn parse_holder(text: &str) -> Option<Holder> {
    let mut lines = text.lines();
    let pid = lines.next()?.trim().parse().ok()?;
    let identity = lines
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let group = lines.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    Some(Holder { pid, identity, group })
}

/// Live while the monitor still runs under the identity it wrote, or while the watched
/// command's process group still has members (a watch that outlived a killed monitor).
fn holder_live(holder: &Holder) -> bool {
    let monitor = sys::alive(holder.pid)
        && match (&holder.identity, sys::identity(holder.pid)) {
            (Some(recorded), Some(now)) => *recorded == now,
            _ => true,
        };
    monitor || (holder.group > 0 && sys::group_alive(holder.group))
}

/// The lock this process holds. Dropping it does not remove the file; `release` does, so
/// the removal is explicit on every path that ends the monitor.
struct Lock {
    path: PathBuf,
    staging: PathBuf,
    me: Holder,
}

enum Refusal {
    /// A live monitor (or its watch) holds the lock under this pid.
    Held(i32),
    Io(String),
}

fn staging_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!(".{name}.{}.tmp", std::process::id()))
}

fn write_file(path: &Path, text: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(text.as_bytes())
}

/// Takes the lock for `name` in `dir`. The lock is a complete file, written to a staging
/// name and hard-linked into place: the link fails if the lock exists, so two starts
/// cannot both take it, and a reader never sees a half-written lock. A lock whose holder
/// is gone is replaced.
fn take_lock(dir: &Path, name: &str) -> Result<Lock, Refusal> {
    fs::create_dir_all(dir).map_err(|e| Refusal::Io(format!("cannot create {}: {e}", dir.display())))?;
    let path = dir.join(format!("{name}.pid"));
    let staging = staging_path(dir, name);
    let me = Holder {
        pid: std::process::id() as i32,
        identity: sys::identity(std::process::id() as i32),
        group: 0,
    };
    for _ in 0..3 {
        write_file(&staging, &render_holder(&me))
            .map_err(|e| Refusal::Io(format!("cannot write {}: {e}", staging.display())))?;
        match fs::hard_link(&staging, &path) {
            Ok(()) => {
                let _ = fs::remove_file(&staging);
                return Ok(Lock { path, staging, me });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&staging);
                match fs::read_to_string(&path) {
                    Ok(text) => {
                        if let Some(holder) = parse_holder(&text) {
                            if holder_live(&holder) {
                                return Err(Refusal::Held(holder.pid));
                            }
                        }
                        // Stale or unreadable: replace it and try again.
                        let _ = fs::remove_file(&path);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(Refusal::Io(format!("cannot read {}: {error}", path.display())))
                    }
                }
            }
            Err(error) => {
                let _ = fs::remove_file(&staging);
                return Err(Refusal::Io(format!("cannot take {}: {error}", path.display())));
            }
        }
    }
    Err(Refusal::Io(format!(
        "could not take {} after three attempts; another start keeps replacing it",
        path.display()
    )))
}

impl Lock {
    /// Records the watched command's process group, so the lock stays live while that
    /// group runs even if this monitor is killed without a chance to clean up.
    fn record_group(&mut self, group: i32) -> std::io::Result<()> {
        self.me.group = group;
        write_file(&self.staging, &render_holder(&self.me))?;
        fs::rename(&self.staging, &self.path)
    }

    fn release(self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(&self.staging);
    }
}

fn run(args: Args) -> ExitCode {
    // One lock per name, or per name and agent when the nick is given.
    let key = match &args.nick {
        Some(nick) => format!("{}.{nick}", args.name),
        None => args.name.clone(),
    };
    let mut lock = match take_lock(&args.dir, &key) {
        Ok(lock) => lock,
        Err(Refusal::Held(pid)) => {
            eprintln!("{PROGRAM}: {} is already running as pid {pid}; not starting another", args.name);
            return ExitCode::from(ALREADY_RUNNING);
        }
        Err(Refusal::Io(message)) => {
            eprintln!("{PROGRAM}: {message}");
            return ExitCode::from(1);
        }
    };
    sys::install_forwarding();
    let mut command = Command::new(&args.command[0]);
    command.args(&args.command[1..]);
    sys::isolate(&mut command);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            lock.release();
            eprintln!("{PROGRAM}: cannot run {}: {error}", args.command[0]);
            return ExitCode::from(127);
        }
    };
    let group = child.id() as i32;
    if let Err(error) = lock.record_group(group) {
        eprintln!("{PROGRAM}: warning: cannot record the watch's process group: {error}");
    }
    let status = loop {
        if let Some(signal) = sys::take_pending() {
            sys::signal_group(group, signal);
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => thread::sleep(POLL),
            Err(error) => {
                eprintln!("{PROGRAM}: cannot wait for {}: {error}", args.command[0]);
                break None;
            }
        }
    };
    // Whatever the command left running in its group (a background job it started) is part
    // of the same watch: stop it, so the lock is released only when the watch is really gone.
    if sys::group_alive(group) {
        sys::signal_group(group, sys::SIGTERM);
    }
    lock.release();
    match status {
        Some(status) => ExitCode::from(sys::exit_code(&status)),
        None => ExitCode::from(1),
    }
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match parse(&argv) {
        Ok(None) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Some(args)) => run(args),
        Err(message) => {
            eprintln!("{PROGRAM}: {message}\n\n{USAGE}");
            ExitCode::from(USAGE_ERROR)
        }
    }
}

/// Process operations. Unix uses libc; other platforms have no signal forwarding or
/// process-group kill here, so the lock falls back to the monitor's own pid.
#[cfg(unix)]
mod sys {
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::{Command, ExitStatus};
    use std::sync::atomic::{AtomicI32, Ordering};

    pub const SIGTERM: i32 = libc::SIGTERM;
    static PENDING: AtomicI32 = AtomicI32::new(0);

    extern "C" fn record(signal: libc::c_int) {
        PENDING.store(signal, Ordering::SeqCst);
    }

    pub fn install_forwarding() {
        let handler: extern "C" fn(libc::c_int) = record;
        for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            unsafe {
                libc::signal(signal, handler as libc::sighandler_t);
            }
        }
    }

    /// The last forwarded signal, cleared once taken.
    pub fn take_pending() -> Option<i32> {
        let signal = PENDING.swap(0, Ordering::SeqCst);
        (signal != 0).then_some(signal)
    }

    pub fn signal_group(group: i32, signal: i32) {
        unsafe {
            libc::kill(-group, signal);
        }
    }

    pub fn alive(pid: i32) -> bool {
        unsafe { libc::kill(pid, 0) == 0 || last_error_is_eperm() }
    }

    pub fn group_alive(group: i32) -> bool {
        unsafe { libc::kill(-group, 0) == 0 || last_error_is_eperm() }
    }

    fn last_error_is_eperm() -> bool {
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }

    /// The process start time as `ps` reports it: stable for a process's life, different
    /// for a later process that reuses its pid. `None` when `ps` cannot say.
    pub fn identity(pid: i32) -> Option<String> {
        let output = Command::new("ps")
            .args(["-o", "lstart=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!text.is_empty()).then_some(text)
    }

    /// Puts the watched command in its own process group, so a signal reaches the watch
    /// and everything it started.
    pub fn isolate(command: &mut Command) {
        command.process_group(0);
    }

    /// Exit status for the monitor: the command's own, or 128+N when it died to signal N.
    pub fn exit_code(status: &ExitStatus) -> u8 {
        match (status.code(), status.signal()) {
            (Some(code), _) => code.clamp(0, 255) as u8,
            (None, Some(signal)) => (128 + signal).clamp(0, 255) as u8,
            (None, None) => 1,
        }
    }
}

#[cfg(not(unix))]
mod sys {
    use std::process::{Command, ExitStatus};

    pub const SIGTERM: i32 = 15;

    pub fn install_forwarding() {}

    pub fn take_pending() -> Option<i32> {
        None
    }

    pub fn signal_group(_group: i32, _signal: i32) {}

    pub fn alive(_pid: i32) -> bool {
        false
    }

    pub fn group_alive(_group: i32) -> bool {
        false
    }

    pub fn identity(_pid: i32) -> Option<String> {
        None
    }

    pub fn isolate(_command: &mut Command) {}

    pub fn exit_code(status: &ExitStatus) -> u8 {
        status.code().map_or(1, |code| code.clamp(0, 255) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_name_dir_and_the_command_after_the_separator() {
        let parsed = parse(&words(&["--name", "chat", "--dir", "/tmp/x", "--", "tail", "-f"]))
            .unwrap()
            .unwrap();
        assert_eq!(parsed.name, "chat");
        assert_eq!(parsed.dir, PathBuf::from("/tmp/x"));
        assert_eq!(parsed.command, words(&["tail", "-f"]));
    }

    #[test]
    fn help_is_not_an_error() {
        assert_eq!(parse(&words(&["--help"])).unwrap(), None);
    }

    #[test]
    fn a_missing_command_or_name_is_refused() {
        assert!(parse(&words(&["--name", "chat", "--dir", "/tmp/x", "--"])).is_err());
        assert!(parse(&words(&["--dir", "/tmp/x", "--", "tail"])).is_err());
        assert!(parse(&words(&["--name", "chat", "--", "tail"])).is_err());
    }

    #[test]
    fn a_nick_is_parsed_and_checked_like_a_name() {
        let parsed = parse(&words(&["--name", "chat", "--nick", "agent-a", "--dir", "/tmp/x", "--", "tail"]))
            .unwrap()
            .unwrap();
        assert_eq!(parsed.nick.as_deref(), Some("agent-a"));
        assert!(parse(&words(&["--name", "chat", "--nick", "../a", "--dir", "/tmp/x", "--", "tail"])).is_err());
    }

    #[test]
    fn a_name_with_a_slash_is_refused() {
        assert!(parse(&words(&["--name", "../x", "--dir", "/tmp", "--", "tail"])).is_err());
    }

    #[test]
    fn a_holder_line_round_trips() {
        let holder = Holder {
            pid: 42,
            identity: Some("Mon Jan  1 00:00:00 2024".into()),
            group: 7,
        };
        assert_eq!(parse_holder(&render_holder(&holder)), Some(holder));
        let bare = Holder { pid: 9, identity: None, group: 0 };
        assert_eq!(parse_holder(&render_holder(&bare)), Some(bare));
        assert_eq!(parse_holder(""), None);
    }

    #[test]
    fn a_live_lock_is_held_and_a_dead_one_is_replaced() {
        let dir = std::env::temp_dir().join(format!("monitor-once-unit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // This test process is alive, so its pid holds the lock.
        let me = std::process::id();
        fs::write(dir.join("watch.pid"), format!("{me}\n\n0\n")).unwrap();
        assert!(matches!(take_lock(&dir, "watch"), Err(Refusal::Held(pid)) if pid == me as i32));
        // A pid that cannot exist is stale: the lock is taken over.
        fs::write(dir.join("watch.pid"), "4000000\n\n0\n").unwrap();
        let lock = take_lock(&dir, "watch").ok().expect("stale lock should be replaced");
        lock.release();
        assert!(!dir.join("watch.pid").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_reused_pid_with_another_identity_does_not_hold_the_lock() {
        let me = Holder {
            pid: std::process::id() as i32,
            identity: Some("a start time no process has".into()),
            group: 0,
        };
        // Only meaningful where `ps` can report identities.
        if sys::identity(me.pid).is_some() {
            assert!(!holder_live(&me));
        }
    }
}
