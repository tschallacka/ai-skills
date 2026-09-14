// MODE: DEV
// PACKAGE: PROD
//! The Unix implementation of the crate's `Backend`, `Transport`, and
//! `Listener` traits (defined in `lib.rs`): PTY spawn/stop/resize/read/write
//! via `openpty`/`libc`, and socket transport via `fchdir`-held-directory
//! binds (see `bind_in_directory`'s own doc comment for why). Everything here
//! is a verbatim move from `lib.rs`'s pre-refactor body; `PosixBackend` and
//! `PosixListener` are the only new code, composing the moved functions into
//! the shared trait interface.
use crate::{Backend, Listener, Transport, INTERRUPTED};
use std::ffi::CString;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
use std::net::Shutdown;
use std::os::fd::AsRawFd;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

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
    // libc's own aliases, not u64: dev_t is i32 on macOS and u64 on Linux, so
    // hardcoding either side makes the comparison in remove_socket() a type
    // error on the other platform.
    device: libc::dev_t,
    inode: libc::ino_t,
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

/// Bind `name` inside the directory `parent_fd` holds open, by name alone.
///
/// WHY THROUGH THE FD AND NOT THE SOCKET'S OWN PATH. The open fd pins the
/// directory: once it exists, nothing can substitute another directory for it,
/// so a socket bound relative to it lands in the directory valid_dir() checked
/// and nowhere else. Binding the absolute path re-walks every component, and a
/// parent swapped in between would take the socket. Verifying afterwards --
/// which capture_socket_identity() does -- only DETECTS that, once the socket
/// already exists in the attacker's directory, which is a weaker guarantee than
/// never creating it there.
///
/// WHY NOT A PATH THROUGH /proc OR /dev/fd, WHICH IS WHAT THIS REPLACED. That
/// was `/proc/self/fd/<n>/<name>` on Linux and `/dev/fd/<n>/<name>` on macOS.
/// The Linux form works because /proc/self/fd/<n> is a magic symlink TO THE
/// DIRECTORY, so a trailing component traverses into it. macOS's /dev/fd is the
/// fdesc filesystem, whose entries stand in for the OPEN FILE rather than a
/// traversable directory entry, so `/dev/fd/<n>/<name>` does not resolve at all.
/// The prefix was swapped and the traversal assumed to port with it; it does
/// not, and every socket-dependent test failed on both macOS legs because of it.
///
/// fchdir plus a relative bind behaves identically on both, and it reduces
/// sun_path to the length of the name -- six bytes for "socket" -- which retires
/// macOS's 104-byte sun_path cap as a consideration permanently, rather than
/// leaving it as headroom to be re-measured whenever the socket moves.
///
/// THE CWD IS PROCESS-GLOBAL, so it is held for the bind alone. The wrapper
/// spawns no threads (grep the crate: only thread::sleep), so there is no
/// concurrent observer inside this process, and the restore is CHECKED rather
/// than assumed: run() execs the child immediately after this, and the child's
/// arguments are relative to the caller's directory, not to the socket's.
/// Run `action` with the process cwd inside the directory `dir_fd` holds.
///
/// The outer Result is the cwd machinery; the inner one is the action's own, so
/// each caller words its own failure. The cwd is restored on every path before
/// either outcome is reported, so no failure leaves the process sitting in the
/// socket's directory.
fn in_held_directory<T>(
    dir_fd: &File,
    dir: &Path,
    what: &str,
    action: impl FnOnce() -> io::Result<T>,
) -> Result<io::Result<T>, String> {
    let previous =
        File::open(".").map_err(|e| format!("cannot hold the current directory: {e}"))?;
    if unsafe { libc::fchdir(dir_fd.as_raw_fd()) } < 0 {
        return Err(format!(
            "cannot enter {} to {what}: {}",
            dir.display(),
            io::Error::last_os_error()
        ));
    }
    let outcome = action();
    let restored = unsafe { libc::fchdir(previous.as_raw_fd()) };
    let restore_error = io::Error::last_os_error();
    if restored < 0 {
        return Err(format!(
            "{what} in {} {}, but the previous directory could not be restored: {}",
            dir.display(),
            if outcome.is_ok() {
                "succeeded"
            } else {
                "failed"
            },
            restore_error
        ));
    }
    Ok(outcome)
}

fn bind_in_directory(parent_fd: &File, dir: &Path, name: &OsStr) -> Result<UnixListener, String> {
    let bound = in_held_directory(parent_fd, dir, "bind the socket", || {
        let old_umask = unsafe { libc::umask(0o177) };
        let bound = UnixListener::bind(Path::new(name));
        unsafe { libc::umask(old_umask) };
        bound
    })?;
    // The directory is named even though the bind was relative: without it the
    // message says which name failed and not where, which is the gap that made
    // this defect take three rounds to identify.
    bound.map_err(|e| {
        format!(
            "bind {} relative to {} failed: {e}",
            Path::new(name).display(),
            dir.display()
        )
    })
}

/// Connect to `socket` by name from inside its own directory.
///
/// The mirror of bind_in_directory, and needed for the same two reasons. The
/// security one: connecting by absolute path re-walks every component, so a
/// swapped parent could hand the client a different socket than the one whose
/// directory was checked; an open fd on the directory cannot be substituted.
///
/// The portability one is what CI caught. macOS caps sun_path at 104 bytes and
/// $TMPDIR there is `/var/folders/<12>/<28>/T/`, so a session socket under it
/// overran the cap and the client failed with "path must be shorter than
/// SUN_LEN" -- on CONNECT, after the bind side had already been moved off
/// absolute paths. Both ends have to be relative or the shorter one just moves
/// the failure. A bare name is six bytes and the cap stops being a
/// consideration.
pub fn connect_in_directory(socket: &Path) -> Result<UnixStream, String> {
    let dir = socket.parent().ok_or("socket needs parent")?;
    let name = socket.file_name().ok_or("socket needs a filename")?;
    let dir_fd = File::open(dir).map_err(|e| format!("cannot open {}: {e}", dir.display()))?;
    let connected = in_held_directory(&dir_fd, dir, "connect to the socket", || {
        UnixStream::connect(Path::new(name))
    })?;
    connected.map_err(|e| {
        format!(
            "connect to {} relative to {} failed: {e}",
            Path::new(name).display(),
            dir.display()
        )
    })
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

pub extern "C" fn interrupt_handler(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

/// The cfg(unix) half of the crate's shared `install_interrupt_handler()`
/// hook (lib.rs): installs the SIGPIPE/SIGTERM/SIGINT handlers `run()` used to
/// install inline.
pub fn install_interrupt_handler() {
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
}

fn spawn(command: &[String], cols: u16, rows: u16) -> Result<(RawFd, libc::pid_t), String> {
    let mut master = 0;
    let mut slave = 0;
    let mut size = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let winp: *mut libc::winsize = &mut size;
    if unsafe {
        // Raw *mut pointers, not &mut references: macOS declares termp and
        // winp as *mut and Linux as *const. *mut coerces to *const, so the
        // mut form is the one shape both accept -- but passing `&mut size`
        // directly trips clippy::unnecessary_mut_passed on Linux, where the
        // parameter is const. Naming the pointer satisfies both.
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            winp,
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
            libc::ioctl(slave, libc::TIOCSCTTY as _, 0);
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
            // c_char, not i8: it is signed on x86_64 and UNSIGNED on both
            // aarch64 targets, so the hardcoded i8 compiled on Intel and failed
            // to compile on aarch64-unknown-linux-musl and aarch64-apple-darwin.
            let p: Vec<*const libc::c_char> = c
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
    if unsafe { libc::ioctl(fd, libc::TIOCSWINSZ as _, &size) } < 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    Ok(())
}

/// The concrete Unix implementation of the crate's `Backend` trait (lib.rs):
/// composes spawn/stop/resize_master/write_master/status above. Its own
/// `Drop` impl replaces the old `Cleanup` struct's process-cleanup half
/// (stop-if-not-reaped, close(master)) -- the socket-identity half is
/// `PosixListener`'s own `Drop`, below.
pub struct PosixBackend {
    master: RawFd,
    pid: libc::pid_t,
    reaped: bool,
}

impl Backend for PosixBackend {
    fn spawn(command: &[String], cols: u16, rows: u16) -> Result<Self, String> {
        let (master, pid) = spawn(command, cols, rows)?;
        Ok(PosixBackend {
            master,
            pid,
            reaped: false,
        })
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        write_master(self.master, bytes)
    }

    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), String> {
        resize_master(self.master, cols, rows)
    }

    fn stop(&mut self) {
        if !self.reaped {
            stop(self.pid);
            self.reaped = true;
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = unsafe { libc::read(self.master, buf.as_mut_ptr().cast(), buf.len()) };
        if n >= 0 {
            Ok(n as usize)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn try_wait(&mut self) -> io::Result<Option<i32>> {
        let mut st = 0;
        let waited = unsafe { libc::waitpid(self.pid, &mut st, libc::WNOHANG) };
        if waited == self.pid {
            self.reaped = true;
            Ok(Some(status(st)))
        } else {
            Ok(None)
        }
    }
}

impl Drop for PosixBackend {
    fn drop(&mut self) {
        if !self.reaped {
            stop(self.pid);
        }
        unsafe {
            libc::close(self.master);
        }
    }
}

/// The concrete Unix implementation of the crate's `Listener` trait (lib.rs):
/// wraps a `UnixListener` plus the `SocketIdentity` `bind()` captures. Its own
/// `Drop` impl replaces the old `Cleanup` struct's socket-identity-cleanup
/// half (`remove_socket`) -- the process-cleanup half is `PosixBackend`'s own
/// `Drop`, above. `bind()` composes `valid_dir`/`bind_in_directory`/
/// `capture_socket_identity`/`set_nonblocking`, self-cleaning via a
/// `SocketGuard`-style local guard on any internal partial failure, exactly
/// as `run()`'s own inline code did before this move.
pub struct PosixListener {
    listener: UnixListener,
    identity: SocketIdentity,
}

/// A tiny internal guard, used only inside `PosixListener::bind`: removes the
/// captured socket identity if dropped before `disarm()` is called. This is
/// the same shape the old top-level `SocketGuard` provided for `run()`'s own
/// inline bind sequence; it now lives entirely inside `bind()` since nothing
/// outside this function needs to see a partially-constructed listener.
struct BindGuard {
    identity: Option<SocketIdentity>,
}

impl BindGuard {
    fn disarm(mut self) -> SocketIdentity {
        self.identity.take().expect("disarm called at most once")
    }
}

impl Drop for BindGuard {
    fn drop(&mut self) {
        if let Some(identity) = self.identity.as_ref() {
            remove_socket(identity);
        }
    }
}

impl Listener for PosixListener {
    type Stream = UnixStream;

    fn bind(socket: &Path) -> Result<Self, String> {
        let dir = socket.parent().ok_or("socket needs parent")?;
        valid_dir(dir)?;
        let parent_fd = File::open(dir).map_err(|e| e.to_string())?;
        if socket.exists() {
            return Err("refusing existing socket".into());
        }
        let name = socket.file_name().ok_or("socket needs a filename")?;
        let listener = bind_in_directory(&parent_fd, dir, name)?;
        let identity = capture_socket_identity(socket, parent_fd)?;
        let guard = BindGuard {
            identity: Some(identity),
        };
        if let Err(error) = listener.set_nonblocking(true) {
            return Err(error.to_string());
        }
        Ok(PosixListener {
            listener,
            identity: guard.disarm(),
        })
    }

    fn accept(&self) -> io::Result<Option<UnixStream>> {
        match self.listener.accept() {
            Ok((stream, _)) => {
                // THE ACCEPTED SOCKET INHERITS O_NONBLOCK ON BSD, AND NOT ON
                // LINUX.
                //
                // The listener is non-blocking on purpose: the shared accept
                // loop polls it between reads of the PTY master. On Linux
                // accept() hands back a BLOCKING socket regardless, so
                // client() could read a request and wait for the rest of it.
                // macOS copies the listener's O_NONBLOCK onto the connection,
                // so every read returned EAGAIN before the request had
                // arrived, client() failed, and the wrapper logged
                //
                //   interactive-shell client: Resource temporarily unavailable
                //                             (os error 35)
                //
                // twice per attempt while the screen stayed empty and the
                // input the caller sent was never delivered. `os error 35` is
                // EAGAIN on macOS; Linux numbers it 11, so the errno in a CI
                // log does not even match across legs.
                //
                // Setting it explicitly is the portable form: it is what
                // Linux already did implicitly, so nothing changes there.
                stream.set_nonblocking(false)?;
                Ok(Some(stream))
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error),
        }
    }
}

impl Drop for PosixListener {
    fn drop(&mut self) {
        remove_socket(&self.identity);
    }
}

impl Transport for UnixStream {
    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        UnixStream::set_read_timeout(self, dur)
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        UnixStream::shutdown(self, how)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialises every test that moves the process directory.
    ///
    /// `in_held_directory` fchdirs the WHOLE PROCESS, and cargo runs unit tests
    /// as threads of one process, so two such tests interleave. Both halves of
    /// that were seen on the aarch64-apple-darwin leg, one per run: master's run
    /// 33868523959 failed `the_socket_is_bound_by_name_inside_the_held_directory`
    /// on "the cwd leaked", and run 33870346097 failed
    /// `a_directory_too_long_for_sun_path_still_binds_and_connects` on "the
    /// socket is not where it was asked for" -- a relative bind that landed in
    /// the other test's directory. Reproduced locally at roughly three runs in
    /// four with `interactive_shell_core-<hash> --test-threads 2 held_directory
    /// sun_path`, which is the pair on its own; the full suite hides it because
    /// nineteen tests rarely put these two on the two threads at once.
    ///
    /// Not a product defect: nothing in this crate spawns a thread (there is no
    /// `thread::spawn` in it), `run()` binds once before it does anything else,
    /// and the input CLI is a one-shot process. The hazard is real for any
    /// FUTURE concurrent caller, so it is recorded here rather than only fixed.
    static CWD: Mutex<()> = Mutex::new(());

    /// Take the cwd lock, treating a poisoned mutex as simply held.
    fn cwd_lock() -> std::sync::MutexGuard<'static, ()> {
        CWD.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The bind is RELATIVE, lands in the held directory, and gives the cwd back.
    #[test]
    fn the_socket_is_bound_by_name_inside_the_held_directory() {
        let _cwd = cwd_lock();
        let before = std::env::current_dir().unwrap();
        let dir = std::env::temp_dir().join(format!("is-bind-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        let socket = dir.join("socket");
        let _ = fs::remove_file(&socket);
        let parent_fd = File::open(&dir).unwrap();

        let listener = bind_in_directory(&parent_fd, &dir, socket.file_name().unwrap()).unwrap();

        // The mechanism: sun_path is the name, not a path through /proc or /dev/fd.
        assert_eq!(
            listener.local_addr().unwrap().as_pathname(),
            Some(Path::new("socket")),
            "the bind was not relative, so sun_path carries a path the caller never chose"
        );
        // ...and "relative" was not achieved by binding somewhere else: the
        // socket exists at the absolute path the caller asked for, and is 0600.
        let mode = fs::metadata(&socket).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the socket is not private");
        // ...and the process is back where it started, or the child run() is
        // about to exec would resolve its arguments against the wrong directory.
        assert_eq!(std::env::current_dir().unwrap(), before, "the cwd leaked");

        drop(listener);
        let _ = fs::remove_file(&socket);
        let _ = fs::remove_dir(&dir);
    }

    /// A directory whose absolute path cannot fit in sun_path still binds AND
    /// connects.
    #[test]
    fn a_directory_too_long_for_sun_path_still_binds_and_connects() {
        let _cwd = cwd_lock();
        let mut dir = std::env::temp_dir().join(format!("is-long-{}", std::process::id()));
        while dir.as_os_str().len() <= 120 {
            dir = dir.join("nested-directory-component");
        }
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        let socket = dir.join("socket");
        let _ = fs::remove_file(&socket);
        assert!(
            socket.as_os_str().len() > 108,
            "the fixture is not long enough to exercise the cap"
        );
        // The control: the absolute forms this replaced cannot do it.
        assert!(
            UnixListener::bind(&socket).is_err(),
            "sun_path accepted a {}-byte bind path, so this test proves nothing",
            socket.as_os_str().len()
        );

        let parent_fd = File::open(&dir).unwrap();
        let listener = bind_in_directory(&parent_fd, &dir, socket.file_name().unwrap()).unwrap();
        assert!(socket.exists(), "the socket is not where it was asked for");

        assert!(
            UnixStream::connect(&socket).is_err(),
            "sun_path accepted a {}-byte connect path, so the connect leg proves nothing",
            socket.as_os_str().len()
        );
        connect_in_directory(&socket).expect("a relative connect must not care about the length");

        drop(listener);
        let _ = fs::remove_file(&socket);
    }

    /// `PosixListener::bind` followed by drop, without ever calling `accept`,
    /// still removes the bound socket file (AR-11's Listener-Drop guarantee).
    #[test]
    fn posix_listener_bind_then_drop_removes_the_socket_file() {
        let _cwd = cwd_lock();
        let dir = std::env::temp_dir().join(format!("is-listener-drop-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        let socket = dir.join("socket");
        let _ = fs::remove_file(&socket);

        let listener = PosixListener::bind(&socket).unwrap();
        assert!(socket.exists(), "bind() did not create the socket file");
        drop(listener);
        assert!(
            !socket.exists(),
            "dropping the listener without ever accepting should still remove the socket file"
        );

        let _ = fs::remove_dir(&dir);
    }

    /// Constructing a `PosixBackend` and dropping it without ever calling
    /// `try_wait()` still reaps and stops the child (AR-11's Backend-Drop
    /// guarantee, the same one `Cleanup`'s own `Drop` used to provide).
    #[test]
    fn posix_backend_drop_without_try_wait_reaps_the_child() {
        let backend = PosixBackend::spawn(&["sleep".into(), "5".into()], 80, 24).unwrap();
        let pid = backend.pid;
        drop(backend);
        // ECHILD (no such child) is the expected outcome: Drop already
        // stopped and reaped it. Still running, or any other errno, is a
        // defect.
        let mut status = 0;
        let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        let errno = io::Error::last_os_error();
        assert_eq!(
            waited, -1,
            "the child should already be reaped by PosixBackend's Drop (errno: {errno})"
        );
    }
}
