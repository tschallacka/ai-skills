// MODE: DEV
// PACKAGE: PROD
//! Making a started server outlive the shell that started it, and stopping it.
//!
//! "Keeps living unless explicitly killed" is three separate things, and missing
//! any one of them leaves a server that dies at a moment nobody expects:
//!
//! 1. **A new session.** `setsid` detaches the process from the controlling
//!    terminal, so closing the terminal does not deliver `SIGHUP` to it. Without
//!    this the server dies when the shell that started it exits, which is the
//!    exact behaviour being fixed.
//! 2. **Detached standard streams.** stdin from `/dev/null`, stdout and stderr to
//!    the server log. A daemon holding the terminal's pipes keeps it from closing
//!    and writes into whatever runs there next.
//! 3. **`SIGHUP` ignored anyway.** Belt to the braces of (1): a process can
//!    acquire a controlling terminal later, and a re-parented process can be
//!    signalled by things other than a terminal closing.
//!
//! And it means there must be a way to stop it deliberately, which is `stop`
//! below — it evicts the registry entry itself rather than leaving a dead entry
//! for the next client's liveness check to find. Discovering a corpse works, but
//! it is not the same as tidying up after yourself.
//!
//! The double-launch shape is `chat serve` (the parent: decides the transport,
//! spawns, waits for registration, exits) and `chat serve --foreground` (the
//! child: the actual server). The prompt therefore always happens in the parent,
//! which is the process that has the terminal — a detached child could not ask
//! anything, and must never need to.

use std::fs::OpenOptions;
use std::path::Path;
use std::process::{Command, Stdio};

/// Signals, declared directly rather than via the libc crate — the same choice
/// as `flock` in `instance.rs` and for the same reasons.
#[cfg(unix)]
mod sys {
    pub const SIGHUP: i32 = 1;
    pub const SIGTERM: i32 = 15;
    pub const SIG_IGN: usize = 1;

    extern "C" {
        pub fn setsid() -> i32;
        pub fn signal(sig: i32, handler: usize) -> usize;
        pub fn kill(pid: i32, sig: i32) -> i32;
    }
}

/// Ignore `SIGHUP` for the life of this process. Called by the server child.
#[cfg(unix)]
pub fn ignore_hangup() {
    unsafe {
        sys::signal(sys::SIGHUP, sys::SIG_IGN);
    }
}

#[cfg(not(unix))]
pub fn ignore_hangup() {
    // Windows has no SIGHUP; a detached process there is already free of the
    // console that started it.
}

/// Spawn this executable again as the detached server.
///
/// `extra` carries the endpoint the parent resolved, so the child never has to
/// resolve it — and therefore never has to ask. Returns the child's pid.
pub fn spawn_detached(log_path: &Path, extra: &[String]) -> Result<u32, String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("cannot find my own executable to re-launch: {e}"))?;
    // Appended, not truncated: a restart should not erase the previous run's
    // last words, which are usually why it is being restarted.
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| format!("cannot open {}: {e}", log_path.display()))?;
    let log_err = log
        .try_clone()
        .map_err(|e| format!("cannot open {} twice: {e}", log_path.display()))?;

    let mut cmd = Command::new(exe);
    cmd.arg("serve")
        .arg("--foreground")
        .args(extra)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));

    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        // Between fork and exec. Only async-signal-safe calls belong here, which
        // setsid is; anything that allocates or takes a lock can deadlock a
        // forked child, so this stays a single call.
        cmd.pre_exec(|| {
            if sys::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    cmd.spawn()
        .map(|c| c.id())
        .map_err(|e| format!("cannot start the server: {e}"))
}

/// Ask a process to stop, politely.
///
/// `SIGTERM`, not `SIGKILL`: the server's lock is released by the kernel either
/// way, but a term gives it the chance to unwind and remove its adverts, and a
/// caller who needs more force can escalate knowingly.
#[cfg(unix)]
pub fn terminate(pid: u32) -> Result<(), String> {
    let rc = unsafe { sys::kill(pid as i32, sys::SIGTERM) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().to_string())
    }
}

#[cfg(not(unix))]
pub fn terminate(pid: u32) -> Result<(), String> {
    // No signals: ask the OS to end it. Windows has no orderly equivalent, which
    // is one more reason the registry entry is evicted by the stopper rather than
    // by the process being stopped.
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("cannot run taskkill: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("taskkill refused to stop pid {pid}"))
    }
}
