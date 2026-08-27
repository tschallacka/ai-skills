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
    pub bind: String,
    pub port: u16,
    /// True when the operator named the endpoint, which makes this a debug
    /// instance living in its own run directory.
    pub explicit: bool,
    run_dir: PathBuf,
}

pub const DEFAULT_BIND: &str = "127.0.0.1";

/// The port every client helper already assumes.
///
/// `chat-send.sh`, `chat-read.sh` and `chat-tail.sh` all do
/// `[ -z "$port" ] && [ -n "$host" ] && port=7717` — none of them reads
/// `server.port`. The interpreter tiers meanwhile bind port 0 and *publish*
/// the kernel's choice, so the documented default client and the documented
/// default server disagree on the endpoint unless someone passes `--port` to
/// both. That is the 7717-versus-kernel-port trap in `chat/SKILL.md`.
///
/// The binary resolves it in the direction that cannot break a client: the
/// default bus binds the port the clients already use. `server.port` is still
/// published, so anything that does read it still works.
pub const DEFAULT_PORT: u16 = 7717;

impl Instance {
    /// A default instance: loopback, the port the helpers assume, run files in
    /// the home root where clients look for them.
    pub fn default_in(home: &Path) -> Self {
        Instance {
            bind: DEFAULT_BIND.to_string(),
            port: DEFAULT_PORT,
            explicit: false,
            run_dir: home.to_path_buf(),
        }
    }

    /// An explicit debug instance. The port must be concrete: a
    /// kernel-assigned port could not be handed to a client deliberately,
    /// which is the whole point of requiring the override to be explicit on
    /// both sides.
    pub fn explicit_in(home: &Path, bind: &str, port: u16) -> Result<Self, String> {
        if port == 0 {
            return Err(
                "an explicit instance needs a concrete --port: port 0 is kernel-assigned, \
                 so no client could be pointed at it on purpose"
                    .to_string(),
            );
        }
        Ok(Instance {
            bind: bind.to_string(),
            port,
            explicit: true,
            // Derived from the endpoint, so it can never collide with the
            // default location that default clients read.
            run_dir: home
                .join("instances")
                .join(format!("{}_{}", sanitise(bind), port)),
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
                    advertised: self.advertised_port(),
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
                advertised: self.advertised_port(),
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
    pub fn publish(&self, _owned: &Guard, port: u16) -> io::Result<()> {
        write_atomic(&self.port_file(), &format!("{}\n", port))?;
        write_atomic(&self.bind_file(), &format!("{}\n", self.bind))?;
        // Diagnostic only. Nothing reads this to decide whether a server runs;
        // see the module docs for why.
        write_atomic(&self.pid_file(), &format!("{}\n", std::process::id()))
    }

    /// The port the live instance published, read only to make the refusal
    /// message useful. Nothing decides anything from it.
    fn advertised_port(&self) -> Option<u16> {
        fs::read_to_string(self.port_file())
            .ok()
            .and_then(|s| s.trim().parse().ok())
    }
}

/// Why a bus could not be taken.
#[derive(Debug)]
pub enum Busy {
    /// Another live process holds this home's lock.
    Held {
        advertised: Option<u16>,
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

    fn tmp(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("chat-instance-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn default_instance_advertises_in_the_home_root() {
        let home = tmp("default");
        let i = Instance::default_in(&home);
        assert_eq!(i.port_file(), home.join("server.port"));
        assert_eq!(i.bind, DEFAULT_BIND);
        assert_eq!(i.port, DEFAULT_PORT);
        assert!(!i.explicit);
    }

    #[test]
    fn explicit_instance_never_touches_the_default_run_files() {
        let home = tmp("explicit");
        let dbg = Instance::explicit_in(&home, "127.0.0.1", 19999).unwrap();
        // The requirement: no discovery path from a default client to a debug
        // instance. Distinct directory, so distinct port/pid/bind files.
        assert_ne!(dbg.port_file(), home.join("server.port"));
        assert_ne!(dbg.pid_file(), home.join("server.pid"));
        assert!(dbg.run_dir().starts_with(home.join("instances")));
        assert!(dbg.explicit);
    }

    #[test]
    fn an_explicit_instance_must_name_a_concrete_port() {
        let home = tmp("port0");
        let err = Instance::explicit_in(&home, "127.0.0.1", 0).unwrap_err();
        assert!(err.contains("concrete --port"), "unhelpful error: {}", err);
    }

    #[test]
    fn a_second_default_instance_is_refused_while_the_first_lives() {
        let home = tmp("singleton");
        let live = Instance::default_in(&home);
        // The guard is bound, not dropped: releasing it would release the bus
        // and the second acquire below would legitimately succeed.
        let held = live.acquire().expect("first should win");
        live.publish(&held, 45123).unwrap();

        match Instance::default_in(&home).acquire() {
            Err(Busy::Held { advertised }) => {
                assert_eq!(advertised, Some(45123), "refusal should name the live port");
            }
            Err(other) => panic!("wrong refusal: {:?}", other),
            Ok(_) => panic!("two default instances took the same home"),
        }
    }

    #[test]
    fn releasing_the_lock_lets_a_fresh_instance_start() {
        let home = tmp("release");
        {
            let _g = Instance::default_in(&home).acquire().expect("first");
        } // dropped: the kernel releases the lock
        Instance::default_in(&home)
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
        Instance::default_in(&home)
            .acquire()
            .expect("a stale pid file must not make a fresh start refuse");
    }

    #[test]
    fn a_debug_instance_starts_alongside_the_default_one() {
        let home = tmp("alongside");
        let _default = Instance::default_in(&home).acquire().expect("default");
        let dbg = Instance::explicit_in(&home, "127.0.0.1", 19998).unwrap();
        let _dbg_guard = dbg
            .acquire()
            .expect("a debug instance must not contend with the default bus");
    }

    #[test]
    fn two_debug_instances_on_one_endpoint_still_exclude_each_other() {
        let home = tmp("dbgdup");
        let a = Instance::explicit_in(&home, "127.0.0.1", 19997).unwrap();
        let _held = a.acquire().expect("first debug instance");
        let b = Instance::explicit_in(&home, "127.0.0.1", 19997).unwrap();
        assert!(
            matches!(b.acquire(), Err(Busy::Held { .. })),
            "the same explicit endpoint is still a singleton"
        );
    }

    #[test]
    fn ipv6_bind_addresses_reach_a_usable_directory_name() {
        let home = tmp("v6");
        let i = Instance::explicit_in(&home, "::1", 19996).unwrap();
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
}
