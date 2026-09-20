// MODE: DEV
// PACKAGE: PROD

//! Signal-safe cleanup, adapted from `src/run-tests/src/scratch.rs`'s own
//! already-proven `install_signal_cleanup` (self-pipe + background thread --
//! a bare `Drop` guard does not fire on a default-disposition SIGINT/SIGTERM,
//! since the process terminates without unwinding). EXTENDED here to also
//! catch SIGHUP, since this script's own real `trap` list is
//! `EXIT INT TERM HUP`, one signal wider than the run-tests precedent
//! covers.

#[cfg(unix)]
use std::sync::atomic::{AtomicI32, Ordering};
#[cfg(unix)]
use std::sync::OnceLock;

#[cfg(unix)]
static SIGNAL_PIPE_WRITE: OnceLock<i32> = OnceLock::new();
#[cfg(unix)]
static RECEIVED_SIGNAL: AtomicI32 = AtomicI32::new(0);

#[cfg(unix)]
extern "C" fn handle_signal(sig: libc::c_int) {
    RECEIVED_SIGNAL.store(sig, Ordering::SeqCst);
    if let Some(&fd) = SIGNAL_PIPE_WRITE.get() {
        let byte = [1u8];
        unsafe {
            libc::write(fd, byte.as_ptr() as *const libc::c_void, 1);
        }
    }
}

/// Installs a SIGINT/SIGTERM/SIGHUP handler that runs `cleanup` on a
/// background thread (never inside the signal handler itself, which only
/// performs the one async-signal-safe `write()` to a self-pipe) and then
/// exits with the conventional `128 + signal` code.
#[cfg(unix)]
pub fn install_signal_cleanup(cleanup: impl FnOnce() + Send + 'static) {
    let mut fds = [0i32; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return;
    }
    let (read_fd, write_fd) = (fds[0], fds[1]);
    if SIGNAL_PIPE_WRITE.set(write_fd).is_err() {
        return;
    }
    unsafe {
        for sig in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            libc::signal(sig, handle_signal as *const () as libc::sighandler_t);
        }
    }
    std::thread::spawn(move || {
        let mut buf = [0u8; 1];
        let n = unsafe { libc::read(read_fd, buf.as_mut_ptr() as *mut libc::c_void, 1) };
        if n > 0 {
            cleanup();
            let sig = RECEIVED_SIGNAL.load(Ordering::SeqCst);
            std::process::exit(128 + sig);
        }
    });
}

#[cfg(windows)]
mod console {
    use std::sync::Mutex;

    type Cleanup = Box<dyn FnOnce() + Send>;
    pub static CLEANUP: Mutex<Option<Cleanup>> = Mutex::new(None);

    extern "system" {
        pub fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> i32>,
            add: i32,
        ) -> i32;
    }

    const CTRL_C_EVENT: u32 = 0;
    const CTRL_BREAK_EVENT: u32 = 1;
    const CTRL_CLOSE_EVENT: u32 = 2;

    /// Ctrl-C, Ctrl-Break and a closing console window: the Windows
    /// counterparts of SIGINT, SIGTERM-ish and SIGHUP. Windows runs this on a
    /// thread of its own, so it may do the cleanup itself before ending the
    /// process with the code a shell reports for that signal.
    pub unsafe extern "system" fn on_ctrl(kind: u32) -> i32 {
        let code = match kind {
            CTRL_C_EVENT => 128 + 2,
            CTRL_BREAK_EVENT => 128 + 15,
            CTRL_CLOSE_EVENT => 128 + 1,
            _ => return 0,
        };
        let cleanup = CLEANUP.lock().ok().and_then(|mut slot| slot.take());
        if let Some(cleanup) = cleanup {
            cleanup();
        }
        std::process::exit(code)
    }
}

/// The Windows counterpart of the signal cleanup above: a console control
/// handler that removes the worktree, its parent and the logs on Ctrl-C.
#[cfg(windows)]
pub fn install_signal_cleanup(cleanup: impl FnOnce() + Send + 'static) {
    if let Ok(mut slot) = console::CLEANUP.lock() {
        *slot = Some(Box::new(cleanup));
    }
    unsafe {
        console::SetConsoleCtrlHandler(Some(console::on_ctrl), 1);
    }
}

#[cfg(not(any(unix, windows)))]
pub fn install_signal_cleanup(_cleanup: impl FnOnce() + Send + 'static) {}
