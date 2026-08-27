// MODE: DEV
// PACKAGE: PROD
//! Instance identity: which bus this process is, where it advertises itself,
//! and whether one is already running.
//!
//! Two rules drive everything here, both learned from defects in the shell
//! implementation this replaces:
//!
//! 1. **"Already running" is never decided by a pid file.** `server.pid`
//!    outlives its process — a dead pid sat in `~/.ai-chat/server.pid` for
//!    hours, and `status` had to be cross-checked three ways before anyone
//!    believed the server was gone. Worse, a presence check fails in the
//!    dangerous direction: a stale file makes a *fresh* start refuse, so the
//!    bus stays down while reporting that it is up. So ownership is an OS lock
//!    that the kernel releases when the process dies, and the pid file is
//!    written only as a human-readable diagnostic that nothing trusts.
//!
//! 2. **A debug instance never writes the default run files.** `server.port`
//!    is what anything reading the endpoint reads. If a debug instance wrote
//!    that file it would silently capture every client that does — production
//!    traffic landing in a debug log, invisible until someone wondered why. So
//!    the run-file directory is derived from the endpoint: only the default
//!    instance owns the default location. That single mechanism is also what
//!    makes clients explicit, seen from the other side — there is no discovery
//!    path to a debug instance, so a client can only reach one by being told
//!    its address.
//!
//! The two together give the endpoint two independent witnesses. The lock says
//! whether this *home* is served; the bind says whether this *port* is taken.
//! Neither can be faked by a leftover file, and a debug instance fails the
//! second without ever touching the first.

use crate::config::Transport;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The singleton is keyed to `AI_CHAT_HOME`, not to the machine.
///
/// The home directory *is* the bus: channels, logs and their locks all live
/// under it, so two homes are two independent message stores and must be able
/// to coexist on one box. A machine-wide singleton would also break the
/// existing shell test suite, which starts a server per runtime tier with a
/// distinct `--home`. "Across machines" in this skill's charter is about
/// clients reaching one server over TCP, not about one server per host.
#[derive(Debug)]
pub struct Instance {
    /// Where this instance listens. A transport, not a bind/port pair, because
    /// a unix socket has no port and pretending otherwise puts a 0 in a message
    /// that a reader would take for a kernel-assigned port.
    pub transport: Transport,
    /// True when the operator named the endpoint, which makes this a debug
    /// instance living in its own run directory.
    pub explicit: bool,
    run_dir: PathBuf,
}

impl Instance {
    /// The default bus for a home: run files where clients look for them.
    ///
    /// The transport comes from the caller, because deciding it involves asking
    /// the user, and this module must stay usable in a test with no terminal.
    pub fn default_with(home: &Path, transport: Transport) -> Self {
        Instance {
            transport,
            explicit: false,
            run_dir: home.to_path_buf(),
        }
    }

    /// An explicit debug instance.
    ///
    /// A TCP port must be concrete: a kernel-assigned port could not be handed
    /// to a client deliberately, which is the whole point of requiring the
    /// override to be explicit on both sides.
    pub fn explicit_with(home: &Path, transport: Transport) -> Result<Self, String> {
        if let Transport::Tcp { port: 0, .. } = &transport {
            return Err(
                "an explicit instance needs a concrete --port: port 0 is kernel-assigned, \
                 so no client could be pointed at it on purpose"
                    .to_string(),
            );
        }
        Ok(Instance {
            // Derived from the endpoint, so it can never collide with the
            // default location that default clients read.
            run_dir: home.join("instances").join(endpoint_tag(&transport)),
            transport,
            explicit: true,
        })
    }

    pub fn run_dir(&self) -> &Path {
        &self.run_dir
    }

    pub fn port_file(&self) -> PathBuf {
        self.run_dir.join("server.port")
    }

    pub fn pid_file(&self) -> PathBuf {
        self.run_dir.join("server.pid")
    }

    pub fn bind_file(&self) -> PathBuf {
        self.run_dir.join("server.bind")
    }

    pub fn lock_file(&self) -> PathBuf {
        self.run_dir.join("server.lock")
    }

    /// Where a unix-socket instance records its path, in place of a port file.
    pub fn socket_file(&self) -> PathBuf {
        self.run_dir.join("server.socket")
    }

    /// Take ownership of this bus, or report who already has it.
    ///
    /// The returned guard must be held for the process's lifetime; dropping it
    /// (or dying) releases the lock, which is precisely why a crash cannot
    /// leave a bus permanently unstartable.
    pub fn acquire(&self) -> Result<Guard, Busy> {
        fs::create_dir_all(&self.run_dir).map_err(|e| Busy::Io(e.to_string()))?;
        let path = self.lock_file();
        let file = match open_lock_file(&path) {
            Ok(f) => f,
            // Windows makes the exclusive claim at open time, so a live owner
            // shows up here as a sharing violation rather than a lock failure.
            Err(e) if is_sharing_violation(&e) => {
                return Err(Busy::Held {
                    advertised: self.advertised(),
                })
            }
            Err(e) => return Err(Busy::Io(format!("{}: {}", path.display(), e))),
        };
        match lock_exclusive_nonblocking(&file) {
            Ok(()) => Ok(Guard {
                file,
                run_dir: self.run_dir.clone(),
            }),
            Err(_) => Err(Busy::Held {
                advertised: self.advertised(),
            }),
        }
    }

    /// Publish this endpoint for clients to discover.
    ///
    /// Takes the guard by reference as proof of ownership: the borrow checker
    /// then guarantees only the process holding the lock can advertise, so a
    /// declined second start cannot overwrite the live instance's port file.
    /// Call it only once the socket is bound, so a reader never sees a port
    /// that nothing answers on.
    pub fn publish(&self, _owned: &Guard, bound_port: Option<u16>) -> io::Result<()> {
        match (&self.transport, bound_port) {
            (Transport::Tcp { bind, .. }, Some(port)) => {
                write_atomic(&self.port_file(), &format!("{port}\n"))?;
                write_atomic(&self.bind_file(), &format!("{bind}\n"))?;
            }
            (Transport::Socket(path), _) => {
                write_atomic(&self.socket_file(), &format!("{}\n", path.display()))?;
            }
            // A TCP listener with no local address is a platform failure, not a
            // state worth advertising as if it were reachable.
            (Transport::Tcp { .. }, None) => {
                return Err(io::Error::other(
                    "a tcp listener reported no bound port, so there is nothing to publish",
                ))
            }
        }
        // Diagnostic only. Nothing reads this to decide whether a server runs;
        // see the module docs for why.
        write_atomic(&self.pid_file(), &format!("{}\n", std::process::id()))
    }

    /// What the live instance published, read only to make a refusal message
    /// useful. Nothing decides anything from it.
    fn advertised(&self) -> Option<String> {
        if let Ok(p) = fs::read_to_string(self.port_file()) {
            let p = p.trim();
            if !p.is_empty() {
                return Some(format!("port {p}"));
            }
        }
        if let Ok(sock) = fs::read_to_string(self.socket_file()) {
            let sock = sock.trim();
            if !sock.is_empty() {
                return Some(format!("socket {sock}"));
            }
        }
        None
    }
}

/// A directory name for an endpoint, safe on every target.
///
/// IPv6 colons and socket path separators are the reason this exists: both
/// reach a directory name, and neither may survive into it.
fn endpoint_tag(transport: &Transport) -> String {
    match transport {
        Transport::Tcp { bind, port } => format!("{}_{}", sanitise(bind), port),
        // Named by its file, so two debug sockets in one home stay distinct.
        Transport::Socket(path) => format!("socket_{}", sanitise(&path.to_string_lossy())),
    }
}

/// Why a bus could not be taken.
#[derive(Debug)]
pub enum Busy {
    /// Another live process holds this home's lock. `advertised` is whatever it
    /// published, as text, purely so the refusal can name it.
    Held {
        advertised: Option<String>,
    },
    Io(String),
}

/// Holds the bus for the process's lifetime and cleans up the advisory files
/// on an orderly exit. The lock itself needs no cleanup: the kernel drops it.
pub struct Guard {
    /// Held for its side effect: dropping this File releases the OS lock, so
    /// the field must outlive the server even though nothing reads it.
    #[allow(dead_code)]
    file: File,
    run_dir: PathBuf,
}

impl Drop for Guard {
    fn drop(&mut self) {
        // Best effort. A killed process leaves these behind and that is fine —
        // they are advisory, and the lock is what carries the truth.
        let _ = fs::remove_file(self.run_dir.join("server.port"));
        let _ = fs::remove_file(self.run_dir.join("server.socket"));
        let _ = fs::remove_file(self.run_dir.join("server.pid"));
    }
}

fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

/// A bind address reaches a directory name, so keep it to characters that are
/// safe there on every target (IPv6 colons are the reason this exists).
fn sanitise(bind: &str) -> String {
    bind.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Open the lock file, claiming it exclusively where the platform claims at
/// open time rather than through a separate locking call.
#[cfg(not(windows))]
fn open_lock_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
}

/// Windows has no `flock`, and its advisory `LockFileEx` needs a Win32 import.
/// Denying every sharing mode at open time is the same guarantee with no FFI:
/// the handle is exclusive, and the kernel closes it when the process dies —
/// which is the property that matters, since a stale file must never wedge the
/// bus. `share_mode(0)` is std, via the Windows-only `OpenOptionsExt`.
#[cfg(windows)]
fn open_lock_file(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .share_mode(0)
        .open(path)
}

/// ERROR_SHARING_VIOLATION (32) is how a live owner presents on Windows.
/// Everywhere else the open succeeds and `flock` decides, so nothing here is a
/// sharing violation.
#[cfg(windows)]
fn is_sharing_violation(e: &io::Error) -> bool {
    e.raw_os_error() == Some(32)
}

#[cfg(not(windows))]
fn is_sharing_violation(_e: &io::Error) -> bool {
    false
}

#[cfg(unix)]
fn lock_exclusive_nonblocking(file: &File) -> io::Result<()> {
    use std::os::unix::io::AsRawFd;
    // Declared directly rather than depending on the libc crate: it is linked
    // regardless, and a dependency-free crate keeps the static musl build
    // trivial. LOCK_EX | LOCK_NB.
    extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    let rc = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn lock_exclusive_nonblocking(_file: &File) -> io::Result<()> {
    // The exclusive claim was already made by open_lock_file's share_mode(0),
    // so reaching here means ownership is held and there is nothing to lock.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Transport, DEFAULT_PORT, LOOPBACK};

    fn tmp(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("chat-instance-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn dbg_tcp(port: u16) -> Transport {
        Transport::Tcp {
            bind: LOOPBACK.to_string(),
            port,
        }
    }

    #[test]
    fn the_default_instance_advertises_in_the_home_root() {
        let home = tmp("default");
        let i = Instance::default_with(&home, Transport::loopback());
        assert_eq!(i.port_file(), home.join("server.port"));
        assert_eq!(i.transport, Transport::loopback());
        assert!(!i.explicit);
    }

    /// The default run directory is the home, whatever the transport: a home has
    /// one bus, and switching it from tcp to a socket must not silently become a
    /// second one.
    #[test]
    fn the_default_run_directory_does_not_depend_on_the_transport() {
        let home = tmp("default-transport");
        let tcp = Instance::default_with(&home, Transport::loopback());
        let sock = Instance::default_with(&home, Transport::socket_in(&home));
        assert_eq!(tcp.run_dir(), sock.run_dir());
        assert_eq!(tcp.lock_file(), sock.lock_file());
    }

    #[test]
    fn explicit_instance_never_touches_the_default_run_files() {
        let home = tmp("explicit");
        let d = Instance::explicit_with(&home, dbg_tcp(19999)).unwrap();
        // The requirement: no discovery path from a default client to a debug
        // instance. Distinct directory, so distinct port/pid/bind files.
        assert_ne!(d.port_file(), home.join("server.port"));
        assert_ne!(d.pid_file(), home.join("server.pid"));
        assert_ne!(d.socket_file(), home.join("server.socket"));
        assert!(d.run_dir().starts_with(home.join("instances")));
        assert!(d.explicit);
    }

    #[test]
    fn an_explicit_tcp_instance_must_name_a_concrete_port() {
        let home = tmp("port0");
        let err = Instance::explicit_with(&home, dbg_tcp(0)).unwrap_err();
        assert!(err.contains("concrete --port"), "unhelpful error: {}", err);
    }

    #[test]
    fn a_second_default_instance_is_refused_while_the_first_lives() {
        let home = tmp("singleton");
        let live = Instance::default_with(&home, Transport::loopback());
        // The guard is bound, not dropped: releasing it would release the bus
        // and the second acquire below would legitimately succeed.
        let held = live.acquire().expect("first should win");
        live.publish(&held, Some(45123)).unwrap();

        match Instance::default_with(&home, Transport::loopback()).acquire() {
            Err(Busy::Held { advertised }) => {
                assert_eq!(
                    advertised.as_deref(),
                    Some("port 45123"),
                    "the refusal should name the live endpoint"
                );
            }
            Err(other) => panic!("wrong refusal: {:?}", other),
            Ok(_) => panic!("two default instances took the same home"),
        }
    }

    /// A socket instance publishes its path where a tcp one publishes a port, so
    /// a refusal can still name where the live bus is.
    #[test]
    fn a_socket_instance_advertises_its_path_not_a_port() {
        let home = tmp("sockadv");
        let live = Instance::default_with(&home, Transport::socket_in(&home));
        let held = live.acquire().expect("first");
        live.publish(&held, None).unwrap();
        assert!(
            !home.join("server.port").exists(),
            "a socket bus must not write a port file: a reader would dial it"
        );
        match Instance::default_with(&home, Transport::socket_in(&home)).acquire() {
            Err(Busy::Held { advertised }) => {
                let said = advertised.expect("should name the socket");
                assert!(said.starts_with("socket "), "got: {said}");
                assert!(said.contains("chat.sock"), "got: {said}");
            }
            Ok(_) => panic!("two socket instances took the same home"),
            Err(other) => panic!("expected a refusal naming the socket, got {other:?}"),
        }
    }

    #[test]
    fn releasing_the_lock_lets_a_fresh_instance_start() {
        let home = tmp("release");
        {
            let _g = Instance::default_with(&home, Transport::loopback())
                .acquire()
                .expect("first");
        } // dropped: the kernel releases the lock
        Instance::default_with(&home, Transport::loopback())
            .acquire()
            .expect("a released bus must be startable again");
    }

    /// The regression test for the stale-state defect. A leftover pid file
    /// naming a process that no longer exists must NOT prevent a start: an
    /// implementation that decided from the pid file would refuse here and
    /// leave chat down while reporting it up.
    #[test]
    fn a_stale_pid_file_does_not_block_a_fresh_start() {
        let home = tmp("stale");
        fs::write(home.join("server.pid"), "2941095\n").unwrap();
        fs::write(home.join("server.port"), "45477\n").unwrap();
        Instance::default_with(&home, Transport::loopback())
            .acquire()
            .expect("a stale pid file must not make a fresh start refuse");
    }

    #[test]
    fn a_debug_instance_starts_alongside_the_default_one() {
        let home = tmp("alongside");
        let _default = Instance::default_with(&home, Transport::loopback())
            .acquire()
            .expect("default");
        let d = Instance::explicit_with(&home, dbg_tcp(19998)).unwrap();
        let _dbg_guard = d
            .acquire()
            .expect("a debug instance must not contend with the default bus");
    }

    #[test]
    fn two_debug_instances_on_one_endpoint_still_exclude_each_other() {
        let home = tmp("dbgdup");
        let a = Instance::explicit_with(&home, dbg_tcp(19997)).unwrap();
        let _held = a.acquire().expect("first debug instance");
        let b = Instance::explicit_with(&home, dbg_tcp(19997)).unwrap();
        assert!(
            matches!(b.acquire(), Err(Busy::Held { .. })),
            "the same explicit endpoint is still a singleton"
        );
    }

    #[test]
    fn ipv6_bind_addresses_reach_a_usable_directory_name() {
        let home = tmp("v6");
        let i = Instance::explicit_with(
            &home,
            Transport::Tcp {
                bind: "::1".to_string(),
                port: 19996,
            },
        )
        .unwrap();
        let name = i
            .run_dir()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(
            !name.contains(':'),
            "colons must not reach a path: {}",
            name
        );
        assert!(
            name.ends_with("_19996"),
            "port should stay legible: {}",
            name
        );
    }

    /// A socket path is full of separators, and every one of them would turn a
    /// run directory into a tree of directories somewhere unintended.
    #[test]
    fn a_socket_path_reaches_a_single_directory_name() {
        let home = tmp("sockdir");
        let i = Instance::explicit_with(&home, Transport::Socket(PathBuf::from("/tmp/a/b.sock")))
            .unwrap();
        let name = i
            .run_dir()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(
            !name.contains('/'),
            "separators must not reach a path: {name}"
        );
        assert_eq!(i.run_dir(), home.join("instances").join(name));
    }

    /// Two debug sockets in one home are two buses, so they must not share a
    /// lock — the bug would be one of them silently refusing to start.
    #[test]
    fn two_debug_sockets_in_one_home_are_distinct_instances() {
        let home = tmp("twosock");
        let a = Instance::explicit_with(&home, Transport::Socket(home.join("a.sock"))).unwrap();
        let b = Instance::explicit_with(&home, Transport::Socket(home.join("b.sock"))).unwrap();
        assert_ne!(a.run_dir(), b.run_dir());
        let _held = a.acquire().expect("first socket instance");
        b.acquire().expect("a different socket is a different bus");
    }

    #[test]
    fn the_default_port_is_the_one_the_helpers_assume() {
        // If this drifts, a default bash client talks to nothing.
        assert_eq!(DEFAULT_PORT, 7717);
        assert_eq!(Transport::loopback(), dbg_tcp(DEFAULT_PORT));
    }
}
