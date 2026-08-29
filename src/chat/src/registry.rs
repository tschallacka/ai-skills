// MODE: DEV
// PACKAGE: PROD
//! Where the running servers are, so a chatter attaches instead of guessing.
//!
//! **Presence, not policy.** This is the ephemeral half of the transport model:
//! it answers "is there a bus for this store, and where", and it is expected to
//! vanish. `config.rs` is the persistent half and answers "how should a bus be
//! exposed", which is the user's decision and must outlive a reboot. So a port
//! in the config is what a server should *bind*; the port in a registry entry is
//! what a server *is listening on*. A client discovers, it no longer assumes.
//!
//! ## Where it lives, and the reboot claim
//!
//! `$XDG_RUNTIME_DIR/chat` when that is set, else `<temp>/chat-<uid>`.
//!
//! The reboot claim needs stating precisely, because it is true on one target
//! and false on another. On Linux `/tmp` is usually tmpfs and does vanish at
//! reboot. On **macOS** `/private/tmp` is on disk, survives reboots, and is only
//! pruned by periodic maintenance for files older than three days. So "it auto
//! cleans on next reboot" is a Linux property, not a portable one, and nothing
//! here may depend on it. `XDG_RUNTIME_DIR` is preferred where present because it
//! is per-user, tmpfs, and cleared on logout — a stronger guarantee than /tmp
//! gives anywhere.
//!
//! Which is why the real defence is not the location: **a stale entry is
//! survivable by design**. See below.
//!
//! ## An entry is not proof of life
//!
//! This is the pid-file lesson in a new place. A crashed server leaves its entry
//! behind, and a client that trusts the file dials a socket nothing is listening
//! on. So liveness is decided by *connecting*, never by the file existing, and a
//! failed dial **evicts** the entry rather than surfacing as an error the user
//! has to interpret. The user's next command then behaves as though the entry had
//! never been there, which is the only outcome that does not require them to know
//! this file exists.
//!
//! ## Why the directory is per-uid and 0700
//!
//! `/tmp` is world-writable, so a fixed shared path is a security question and
//! not a layout one. Another user on the box could pre-create the directory, or
//! plant an entry naming a socket they own, and every client on the machine would
//! talk to them — the protocol has no authentication, so nothing would notice.
//! Hence: a per-uid directory, created `0700`, and **refused if its owner is not
//! the current uid or it is a symlink**. The timestamp in an entry's name gives
//! uniqueness; it gives no ownership at all.

use crate::config::{self, Transport};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// One registered server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The store this server owns. The lookup key: one live bus per home, which
    /// is the same singleton the home lock already enforces.
    pub home: PathBuf,
    pub transport: Transport,
    /// Diagnostic and the target of `chat stop`. Never used to decide liveness.
    pub pid: u32,
    pub started: u64,
    /// The file this came from, so it can be evicted.
    pub path: PathBuf,
}

/// An opened registry directory.
///
/// A value rather than a set of free functions reading the environment on every
/// call, and that is not a style choice: the directory used to come from
/// `XDG_RUNTIME_DIR` at each call site, so the tests had to *set* that variable —
/// process-global state, mutated by tests cargo runs in parallel, which made them
/// stomp on each other and fail only in a full run. Threading the directory
/// through as a value removes the shared state instead of serialising around it.
#[derive(Debug)]
pub struct Registry {
    dir: PathBuf,
}

impl Registry {
    /// The registry for this user, at the documented location.
    pub fn open() -> Result<Registry, String> {
        let base = match std::env::var_os("XDG_RUNTIME_DIR") {
            // Already per-user, tmpfs, and cleared on logout: the best of the
            // options, so it does not need a uid suffix.
            Some(x) if !x.is_empty() => PathBuf::from(x).join("chat"),
            _ => std::env::temp_dir().join(format!("chat-{}", current_uid())),
        };
        Registry::at(base)
    }

    /// A registry at a named directory. The ownership and permission checks apply
    /// here too — an explicit path is not a reason to trust it.
    pub fn at(dir: PathBuf) -> Result<Registry, String> {
        ensure_private(&dir)?;
        Ok(Registry { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

#[cfg(unix)]
fn current_uid() -> u32 {
    // Declared directly rather than depending on the libc crate, the same way
    // flock is in instance.rs: libc is linked regardless, and a dependency-free
    // crate keeps the static musl build trivial.
    extern "C" {
        fn getuid() -> u32;
    }
    unsafe { getuid() }
}

#[cfg(not(unix))]
fn current_uid() -> u32 {
    // Windows has no uid, and its per-user temp directory is already inside the
    // user's profile, so the shared-path problem this suffix defends against
    // does not arise there.
    0
}

/// Create the directory 0700, or refuse to use it.
///
/// Refusing is the point. A directory somebody else owns, or a symlink pointing
/// anywhere, is exactly the attack the per-uid path exists to prevent, and
/// "carry on anyway" would hand every client on the box to whoever got there
/// first.
#[cfg(unix)]
fn ensure_private(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return Err(format!(
                "{} is a symlink; refusing to use it as the server registry",
                path.display()
            ));
        }
        if !meta.is_dir() {
            return Err(format!("{} exists and is not a directory", path.display()));
        }
        if meta.uid() != current_uid() {
            return Err(format!(
                "{} is owned by uid {} and not by you (uid {}); refusing to use it as the \
                 server registry, because an entry planted there would point every client on \
                 this machine at someone else's socket",
                path.display(),
                meta.uid(),
                current_uid()
            ));
        }
        // Tighten a directory that exists with looser permissions, rather than
        // trusting whatever a umask or an earlier version left behind.
        let mut perms = meta.permissions();
        if perms.mode() & 0o077 != 0 {
            perms.set_mode(0o700);
            let _ = fs::set_permissions(path, perms);
        }
        return Ok(());
    }
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(|e| format!("cannot create {}: {e}", path.display()))
}

#[cfg(not(unix))]
fn ensure_private(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("cannot create {}: {e}", path.display()))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Register a live server. The caller must already own the home lock, so this
/// cannot create a second entry for one store.
impl Registry {
    pub fn register(&self, home: &Path, transport: &Transport) -> Result<PathBuf, String> {
        let dir = &self.dir;
        // Timestamped and pid-tagged: unique without coordination, and legible in a
        // directory listing, which is what makes a stray entry easy to recognise.
        let path = dir.join(format!("{}-{}.server", now(), std::process::id()));
        let body = format!(
            "# A live chat server. Written by `chat serve`; deleted when it stops.\n\
         # This file is PRESENCE, not policy: it says where a bus is, not how it\n\
         # should be exposed - that is <home>/config, which outlives a reboot.\n\
         #\n\
         # An entry is NOT proof of life. A crashed server leaves one behind, so a\n\
         # client decides by connecting and evicts this file when nothing answers.\n\
         home={}\n\
         pid={}\n\
         started={}\n\
         {}",
            home.display(),
            std::process::id(),
            now(),
            Self::transport_keys(transport)
        );
        let tmp = path.with_extension("tmp");
        {
            let mut f = fs::File::create(&tmp)
                .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
            f.write_all(body.as_bytes())
                .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        }
        // Renamed into place, so a scanning client never reads half an entry.
        fs::rename(&tmp, &path).map_err(|e| format!("cannot place {}: {e}", path.display()))?;
        Ok(path)
    }

    fn transport_keys(t: &Transport) -> String {
        match t {
            Transport::Socket(p) => format!("transport=socket\nsocket={}\n", p.display()),
            Transport::Tcp { bind, port } => format!("transport=tcp\nbind={bind}\nport={port}\n"),
        }
    }

    /// Every entry currently in the registry, live or not.
    ///
    /// Unreadable and unparseable files are skipped rather than fatal: this directory
    /// is shared with nothing, but a half-written or hand-mangled file must not make
    /// `chat` unusable.
    pub fn all(&self) -> Result<Vec<Entry>, String> {
        let mut out = Vec::new();
        let listing = match fs::read_dir(&self.dir) {
            Ok(l) => l,
            Err(_) => return Ok(out),
        };
        for item in listing.flatten() {
            let path = item.path();
            if path.extension().and_then(|e| e.to_str()) != Some("server") {
                continue;
            }
            if let Some(e) = Self::parse_entry(&path) {
                out.push(e);
            }
        }
        out.sort_by_key(|e| e.started);
        Ok(out)
    }

    fn parse_entry(path: &Path) -> Option<Entry> {
        let text = fs::read_to_string(path).ok()?;
        let mut home = None;
        let mut pid = 0u32;
        let mut started = 0u64;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (k, v) = line.split_once('=')?;
            match k.trim() {
                "home" => home = Some(PathBuf::from(v.trim())),
                "pid" => pid = v.trim().parse().unwrap_or(0),
                "started" => started = v.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
        let home = home?;
        // The transport keys are deliberately the same as the config's, so one
        // parser serves both and the two files cannot drift on how a transport is
        // spelled.
        let transport = config::parse(&text, &home).ok()?;
        Some(Entry {
            home,
            transport,
            pid,
            started,
            path: path.to_path_buf(),
        })
    }
}

/// Remove an entry. Used when a dial fails, and by an explicit stop.
///
/// Free rather than a method: an entry carries its own path, and evicting one
/// needs no directory handle. That also means a caller holding a stale entry can
/// always clean up, even if the registry has since become untrustworthy.
pub fn evict(entry: &Entry) {
    let _ = fs::remove_file(&entry.path);
}

impl Registry {
    /// The live server for a store, if there is one.
    ///
    /// "Live" means it answered. A registered entry that does not answer is evicted
    /// here, so the caller sees the same thing it would have seen had the crashed
    /// server never registered — no error to interpret, and no knowledge of this
    /// directory required.
    pub fn live_for(&self, home: &Path) -> Result<Option<Entry>, String> {
        let mut found = None;
        for entry in self.all()? {
            if entry.home != home {
                continue;
            }
            if answers(&entry.transport) {
                // Newest wins if two ever claim one home: `all()` is sorted by start
                // time, and the older one is the one that failed to clean up.
                found = Some(entry);
            } else {
                evict(&entry);
            }
        }
        Ok(found)
    }
}

/// Does something accept a connection there?
///
/// A connect, not a pid check: a pid can be reused, a socket file can outlive
/// its process, and neither tells you whether the thing listening is ours.
fn answers(transport: &Transport) -> bool {
    crate::net::connect(transport, false).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each test gets its own registry directory, passed in as a value.
    ///
    /// Deliberately NOT by setting XDG_RUNTIME_DIR: that is process-global, cargo
    /// runs these in parallel, and the earlier version of these tests failed only
    /// in a full run because they overwrote each other's directory. Every test
    /// sets every input it depends on.
    fn reg(tag: &str) -> Registry {
        let mut p = std::env::temp_dir();
        p.push(format!("chat-reg-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        Registry::at(p).unwrap()
    }

    fn home(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("chat-reg-home-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// Nothing listens on port 1, so this transport is registrable but never
    /// live — which is precisely the crashed-server case.
    fn dead_tcp() -> Transport {
        Transport::Tcp {
            bind: "127.0.0.1".to_string(),
            port: 1,
        }
    }

    #[test]
    #[cfg(unix)]
    fn the_registry_directory_is_created_private() {
        use std::os::unix::fs::PermissionsExt;
        let r = reg("mode");
        let mode = fs::metadata(r.dir()).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o700,
            "/tmp is world-writable, so a shared registry is a security question"
        );
    }

    /// A directory left behind with loose permissions is tightened rather than
    /// trusted: an earlier version's umask must not decide this.
    #[test]
    #[cfg(unix)]
    fn a_loosely_permissioned_directory_is_tightened() {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::env::temp_dir();
        p.push(format!("chat-reg-loose-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o777)).unwrap();
        let r = Registry::at(p).unwrap();
        let mode = fs::metadata(r.dir()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "loose permissions must be corrected");
    }

    /// The security case: /tmp is world-writable, so another user could plant a
    /// directory and every client on the box would talk to their socket.
    #[test]
    #[cfg(unix)]
    fn a_symlinked_registry_directory_is_refused() {
        let scratch = home("symlink");
        let target = scratch.join("elsewhere");
        fs::create_dir_all(&target).unwrap();
        let link = scratch.join("chat");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let e = Registry::at(link).unwrap_err();
        assert!(e.contains("symlink"), "unhelpful: {e}");
    }

    #[test]
    fn xdg_runtime_dir_is_used_when_it_is_set() {
        // Reads the variable rather than writing it: whatever the environment
        // says, open() must agree with it.
        let expected = match std::env::var_os("XDG_RUNTIME_DIR") {
            Some(x) if !x.is_empty() => PathBuf::from(x).join("chat"),
            _ => std::env::temp_dir().join(format!("chat-{}", current_uid())),
        };
        match Registry::open() {
            Ok(r) => assert_eq!(r.dir(), expected),
            // A refusal is a legitimate outcome here (someone else's directory);
            // what must not happen is silently using a different path.
            Err(e) => assert!(e.contains(&expected.display().to_string()), "{e}"),
        }
    }

    #[test]
    fn a_registered_server_is_found_by_its_home() {
        let r = reg("find");
        let h = home("find");
        let t = dead_tcp();
        r.register(&h, &t).unwrap();
        let entries = r.all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].home, h);
        assert_eq!(entries[0].transport, t);
        assert_eq!(entries[0].pid, std::process::id());
    }

    #[test]
    #[cfg(unix)]
    fn an_entry_records_a_socket_transport_the_same_way_the_config_does() {
        let r = reg("sockentry");
        let h = home("sockentry");
        let t = Transport::Socket(h.join("chat.sock"));
        r.register(&h, &t).unwrap();
        assert_eq!(r.all().unwrap()[0].transport, t);
    }

    /// The one that matters most. A crashed server's entry must not send a client
    /// at a socket nothing is listening on, and the client must not have to know
    /// this file exists.
    #[test]
    fn a_registered_server_that_does_not_answer_is_not_live_and_is_evicted() {
        let r = reg("stale");
        let h = home("stale");
        r.register(&h, &dead_tcp()).unwrap();
        assert_eq!(
            r.all().unwrap().len(),
            1,
            "the entry is there to begin with"
        );
        assert_eq!(
            r.live_for(&h).unwrap(),
            None,
            "an entry is not proof of life: nothing answers there"
        );
        assert!(
            r.all().unwrap().is_empty(),
            "a dead entry must be evicted, not left for the next caller to re-discover"
        );
    }

    #[test]
    fn an_entry_for_another_home_is_not_offered_to_this_one() {
        let r = reg("otherhome");
        let mine = home("otherhome-mine");
        let theirs = home("otherhome-theirs");
        r.register(&theirs, &dead_tcp()).unwrap();
        assert_eq!(r.live_for(&mine).unwrap(), None);
        // Evicting is scoped to the home being asked about: another store's stale
        // entry is not this caller's business.
        assert_eq!(
            r.all().unwrap().len(),
            1,
            "another home's entry is left alone"
        );
    }

    #[test]
    fn an_unparseable_file_is_skipped_rather_than_fatal() {
        let r = reg("junk");
        fs::write(r.dir().join("nonsense.server"), "this is not an entry\n").unwrap();
        fs::write(r.dir().join("ignored.txt"), "not an entry file at all\n").unwrap();
        let h = home("junk");
        r.register(&h, &dead_tcp()).unwrap();
        assert_eq!(
            r.all().unwrap().len(),
            1,
            "one good entry among the rubbish must still be found"
        );
    }

    #[test]
    fn eviction_removes_the_file() {
        let r = reg("evict");
        let h = home("evict");
        r.register(&h, &dead_tcp()).unwrap();
        let e = r.all().unwrap().remove(0);
        evict(&e);
        assert!(r.all().unwrap().is_empty());
        // Evicting twice is not an error: two clients can both find it dead.
        evict(&e);
    }
}
