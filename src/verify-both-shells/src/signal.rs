// MODE: DEV
// PACKAGE: PROD

//! Signal-safe cleanup, adapted from `src/run-tests/src/scratch.rs`'s own
//! already-proven `install_signal_cleanup` (self-pipe + background thread --
//! a bare `Drop` guard does not fire on a default-disposition SIGINT/SIGTERM,
//! since the process terminates without unwinding). EXTENDED here to also
//! catch SIGHUP, since this script's own real `trap` list is
//! `EXIT INT TERM HUP`, one signal wider than the run-tests precedent
//! covers.

use std::sync::atomic::AtomicI32;
#[cfg(unix)]
use std::sync::atomic::Ordering;
use std::sync::OnceLock;

static SIGNAL_PIPE_WRITE: OnceLock<i32> = OnceLock::new();
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

#[cfg(not(unix))]
pub fn install_signal_cleanup(_cleanup: impl FnOnce() + Send + 'static) {}
