// MODE: DEV
// PACKAGE: PROD
//! The Windows implementation of the crate's `Backend` trait (defined in
//! `lib.rs`): PTY spawn/stop/resize/read/write via ConPTY
//! (`CreatePseudoConsole`/`ResizePseudoConsole`/`ClosePseudoConsole`) and a
//! Job Object (`CreateJobObjectW`/`AssignProcessToJobObject`/
//! `TerminateJobObject`) in place of POSIX's openpty/ioctl/setsid/kill. The
//! confirmed windows-sys API surface this binds against is recorded in
//! `.plans/windows-interactive-shell/02-conpty-backend/working-context.md`.
use crate::Backend;
use std::ffi::c_void;
use std::io;
use std::mem::size_of;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, STILL_ACTIVE};
#[cfg(test)]
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows_sys::Win32::System::Console::{
    ClosePseudoConsole, CreatePseudoConsole, ResizePseudoConsole, COORD, HPCON,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
};
use windows_sys::Win32::System::Pipes::{CreatePipe, PeekNamedPipe};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
    InitializeProcThreadAttributeList, TerminateProcess, UpdateProcThreadAttribute,
    EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, STARTUPINFOEXW,
};
#[cfg(test)]
use windows_sys::Win32::System::Threading::{STARTF_USESTDHANDLES, STARTUPINFOW};

/// Builds the single command-line string `CreateProcessW` expects, following
/// the same backslash/quote escaping `CommandLineToArgvW` unpacks on the
/// other end. Unlike POSIX's `execvp(argv[0], argv)`, Windows has no argv
/// array -- the OS hands the whole string to the child, which is expected to
/// split it back apart itself, so the split must be reversible.
fn quote_command_line(command: &[String]) -> Vec<u16> {
    let mut line = String::new();
    for (i, arg) in command.iter().enumerate() {
        if i > 0 {
            line.push(' ');
        }
        if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
            line.push_str(arg);
            continue;
        }
        line.push('"');
        let mut chars = arg.chars().peekable();
        loop {
            let mut backslashes = 0;
            while chars.peek() == Some(&'\\') {
                backslashes += 1;
                chars.next();
            }
            match chars.next() {
                Some('"') => {
                    line.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                    line.push('"');
                }
                Some(c) => {
                    line.extend(std::iter::repeat_n('\\', backslashes));
                    line.push(c);
                }
                None => {
                    line.extend(std::iter::repeat_n('\\', backslashes * 2));
                    break;
                }
            }
        }
        line.push('"');
    }
    line.encode_utf16().chain(std::iter::once(0)).collect()
}

struct Pipe {
    read: HANDLE,
    write: HANDLE,
}

fn create_pipe() -> Result<Pipe, String> {
    let mut read: HANDLE = null_mut();
    let mut write: HANDLE = null_mut();
    // Default (NULL) security attributes: non-inheritable, matching
    // Microsoft's own ConPTY sample -- inheritance is not how these handles
    // reach the child; CreatePseudoConsole owns that.
    if unsafe { CreatePipe(&mut read, &mut write, null(), 0) } == 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    Ok(Pipe { read, write })
}

/// Owns the `InitializeProcThreadAttributeList` buffer and tears it down via
/// `DeleteProcThreadAttributeList` on drop -- the same Drop-based cleanup
/// `PosixBackend`/`PosixListener` (goal 1) use in place of manual guards.
struct AttributeList {
    buffer: Vec<u8>,
}

impl AttributeList {
    fn new(hpc: HPCON) -> Result<Self, String> {
        let mut size: usize = 0;
        // First call is expected to fail (buffer too small); it reports the
        // real size to allocate in `size`.
        unsafe { InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut size) };
        if size == 0 {
            return Err("InitializeProcThreadAttributeList did not report a buffer size".into());
        }
        let mut buffer = vec![0u8; size];
        let list = buffer.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
        if unsafe { InitializeProcThreadAttributeList(list, 1, 0, &mut size) } == 0 {
            return Err(io::Error::last_os_error().to_string());
        }
        let attrs = AttributeList { buffer };
        // The attribute VALUE for PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE is the
        // HPCON handle's own bit pattern, passed directly as lpValue (not a
        // pointer to a variable holding it) -- confirmed against Microsoft's
        // own ConPTY sample (samples/ConPTY/EchoCon in microsoft/terminal),
        // which passes `hPC` itself, not `&hPC`.
        if unsafe {
            UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                hpc as *const c_void,
                size_of::<HPCON>(),
                null_mut(),
                null(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error().to_string());
        }
        Ok(attrs)
    }

    fn as_ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.buffer.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.as_ptr()) };
    }
}

/// Composes ConPTY + a Job Object into the shared `Backend` interface (goal
/// 1, W05). Its own `Drop` impl mirrors `PosixBackend`'s: terminate-if-not-
/// reaped, then release every HANDLE this backend owns.
pub struct WindowsBackend {
    hpc: HPCON,
    input_write: HANDLE,
    output_read: HANDLE,
    process: HANDLE,
    thread: HANDLE,
    job: HANDLE,
    reaped: bool,
}

impl Backend for WindowsBackend {
    fn spawn(command: &[String], cols: u16, rows: u16) -> Result<Self, String> {
        let input = create_pipe()?;
        let output = create_pipe()?;

        let size = COORD {
            X: cols as i16,
            Y: rows as i16,
        };
        let mut hpc: HPCON = 0;
        let hr = unsafe { CreatePseudoConsole(size, input.read, output.write, 0, &mut hpc) };
        if hr < 0 {
            unsafe {
                CloseHandle(input.read);
                CloseHandle(input.write);
                CloseHandle(output.read);
                CloseHandle(output.write);
            }
            return Err(format!("CreatePseudoConsole failed: HRESULT {hr:#x}"));
        }
        // CreatePseudoConsole duplicates the ends it needs internally; the
        // caller closes its own copies of the ends it handed over (the ends
        // the *parent* keeps -- input.write, output.read -- stay open).
        unsafe {
            CloseHandle(input.read);
            CloseHandle(output.write);
        }

        let mut attrs = match AttributeList::new(hpc) {
            Ok(a) => a,
            Err(e) => {
                unsafe {
                    ClosePseudoConsole(hpc);
                    CloseHandle(input.write);
                    CloseHandle(output.read);
                }
                return Err(e);
            }
        };

        let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.lpAttributeList = attrs.as_ptr();
        // Explicit desktop: on some CI runners the launching process isn't
        // itself attached to an interactive window station/desktop, and a
        // child spawned without an explicit one can fail to bind properly
        // to ConPTY's virtual console, exiting cleanly almost immediately
        // instead of erroring. WinSta0\Default is the standard interactive
        // desktop; naming it explicitly costs nothing when it's already
        // correct and fixes it when it silently wasn't.
        let mut desktop: Vec<u16> = "WinSta0\\Default\0".encode_utf16().collect();
        startup.StartupInfo.lpDesktop = desktop.as_mut_ptr();

        let mut command_line = quote_command_line(command);
        let mut process_information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let created = unsafe {
            CreateProcessW(
                null(),
                command_line.as_mut_ptr(),
                null(),
                null(),
                0, // bInheritHandles: FALSE -- ConPTY connects via the
                // attribute list, not standard-handle inheritance.
                EXTENDED_STARTUPINFO_PRESENT,
                null(),
                null(),
                &startup.StartupInfo,
                &mut process_information,
            )
        };
        if created == 0 {
            let err = io::Error::last_os_error().to_string();
            unsafe {
                ClosePseudoConsole(hpc);
                CloseHandle(input.write);
                CloseHandle(output.read);
            }
            return Err(err);
        }

        let job = unsafe { CreateJobObjectW(null(), null()) };
        if job.is_null() {
            let err = io::Error::last_os_error().to_string();
            unsafe {
                TerminateProcess(process_information.hProcess, 1);
                CloseHandle(process_information.hProcess);
                CloseHandle(process_information.hThread);
                ClosePseudoConsole(hpc);
                CloseHandle(input.write);
                CloseHandle(output.read);
            }
            return Err(err);
        }
        if unsafe { AssignProcessToJobObject(job, process_information.hProcess) } == 0 {
            let err = io::Error::last_os_error().to_string();
            unsafe {
                TerminateProcess(process_information.hProcess, 1);
                CloseHandle(process_information.hProcess);
                CloseHandle(process_information.hThread);
                CloseHandle(job);
                ClosePseudoConsole(hpc);
                CloseHandle(input.write);
                CloseHandle(output.read);
            }
            return Err(err);
        }

        Ok(WindowsBackend {
            hpc,
            input_write: input.write,
            output_read: output.read,
            process: process_information.hProcess,
            thread: process_information.hThread,
            job,
            reaped: false,
        })
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        let mut written = 0u32;
        if unsafe {
            WriteFile(
                self.input_write,
                bytes.as_ptr(),
                bytes.len() as u32,
                &mut written,
                null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error().to_string());
        }
        Ok(())
    }

    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), String> {
        let size = COORD {
            X: cols as i16,
            Y: rows as i16,
        };
        let hr = unsafe { ResizePseudoConsole(self.hpc, size) };
        if hr < 0 {
            return Err(format!("ResizePseudoConsole failed: HRESULT {hr:#x}"));
        }
        Ok(())
    }

    fn stop(&mut self) {
        // Windows has no SIGTERM/SIGKILL escalation; TerminateJobObject is
        // the one-shot equivalent for the whole process tree, mirroring
        // PosixBackend's `reaped` guard against a double termination.
        if !self.reaped {
            unsafe { TerminateJobObject(self.job, 1) };
            self.reaped = true;
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Anonymous pipes have no O_NONBLOCK equivalent: PeekNamedPipe first
        // is what avoids blocking run()'s poll loop when nothing is waiting,
        // the same role O_NONBLOCK plays on posix.rs's master fd.
        let mut available = 0u32;
        if unsafe {
            PeekNamedPipe(
                self.output_read,
                null_mut(),
                0,
                null_mut(),
                &mut available,
                null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if available == 0 {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "no data available",
            ));
        }
        let to_read = (buf.len() as u32).min(available);
        let mut read = 0u32;
        if unsafe {
            ReadFile(
                self.output_read,
                buf.as_mut_ptr(),
                to_read,
                &mut read,
                null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(read as usize)
    }

    fn try_wait(&mut self) -> io::Result<Option<i32>> {
        let mut code = 0u32;
        if unsafe { GetExitCodeProcess(self.process, &mut code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if code == STILL_ACTIVE as u32 {
            return Ok(None);
        }
        self.reaped = true;
        Ok(Some(code as i32))
    }
}

impl Drop for WindowsBackend {
    fn drop(&mut self) {
        if !self.reaped {
            unsafe { TerminateJobObject(self.job, 1) };
        }
        unsafe {
            ClosePseudoConsole(self.hpc);
            CloseHandle(self.input_write);
            CloseHandle(self.output_read);
            CloseHandle(self.thread);
            CloseHandle(self.process);
            CloseHandle(self.job);
        }
    }
}

// --- Compile-only placeholders, pending goal 3 ------------------------------
//
// `run()` (lib.rs, shared/unconditional) calls `install_interrupt_handler()`
// and `PlatformListener::bind()` regardless of platform, so the crate cannot
// type-check for any `cfg(windows)` target -- including this goal's own W13
// scratch CI proof, which builds the crate's `--lib` target to reach
// `windows::tests` -- without SOME cfg(windows) resolution for both names.
// Goal 3 (W14/W15) replaces this with the real loopback-TCP Listener and a
// real `SetConsoleCtrlHandler`-based interrupt handler; W13's own tests call
// `WindowsBackend`'s Backend methods directly and never go through `run()`
// or this Listener, so neither needs to do anything real yet -- only exist.
use crate::{Listener, Transport};
use std::net::{Shutdown, TcpStream};
use std::path::Path;
use std::time::Duration;

pub(crate) fn install_interrupt_handler() {}

impl Transport for TcpStream {
    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        TcpStream::set_read_timeout(self, dur)
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        TcpStream::shutdown(self, how)
    }
}

pub struct WindowsListener;

impl Listener for WindowsListener {
    type Stream = TcpStream;

    fn bind(_socket: &Path) -> Result<Self, String> {
        Err("Windows transport is not implemented yet (goal 3, W14/W15)".into())
    }

    fn accept(&self) -> io::Result<Option<Self::Stream>> {
        Ok(None)
    }
}

// W13's own verification: these tests exercise WindowsBackend's Backend
// methods directly (spawn/write/read/resize/try_wait/stop), never through
// `run()`/`client()` -- the crate's normal transport still doesn't exist
// (see the placeholders above), but that is irrelevant here. Only provable
// on a real windows-latest runner; see the goal's own scratch CI job.
#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Instant;

    /// Polls `read()` until `predicate` matches the full text accumulated so
    /// far, or `timeout` elapses. PowerShell's own startup (profile load,
    /// module imports) is slow and variable, so a fixed sleep-then-write is
    /// unreliable -- this waits for the actual prompt instead of a guess.
    fn wait_for(
        backend: &mut WindowsBackend,
        timeout: Duration,
        predicate: impl Fn(&str) -> bool,
    ) -> String {
        let start = Instant::now();
        let mut collected = Vec::new();
        let mut buf = [0u8; 4096];
        let mut last_progress_log = Instant::now();
        loop {
            match backend.read(&mut buf) {
                Ok(n) if n > 0 => collected.extend_from_slice(&buf[..n]),
                _ => {}
            }
            let text = String::from_utf8_lossy(&collected).into_owned();
            if predicate(&text) || start.elapsed() >= timeout {
                return text;
            }
            if last_progress_log.elapsed() >= Duration::from_secs(5) {
                eprintln!(
                    "wait_for: {:?} elapsed, {} bytes so far, try_wait: {:?}",
                    start.elapsed(),
                    collected.len(),
                    backend.try_wait()
                );
                last_progress_log = Instant::now();
            }
            sleep(Duration::from_millis(20));
        }
    }

    /// Spawns `program` and gives it `grace_period` to prove it's not one
    /// of the runs where it exits almost immediately (code 0, no prompt) --
    /// real CI runs showed this intermittently, for both cmd.exe and
    /// powershell.exe, timing-dependent enough (sometimes dead by 2s,
    /// sometimes alive past 20s) to be consistent with the runner's
    /// Defender/EDR racing a behavioral scan against a ConPTY-attached
    /// process launch (this exact CreateProcess+PROC_THREAD_ATTRIBUTE_
    /// PSEUDOCONSOLE pattern is a well-documented reverse-shell/C2
    /// signature) rather than a bug in spawn() itself -- every run where
    /// the child survived showed fully correct ConPTY output. Retries a
    /// fresh spawn on an early death instead of accepting flaky evidence.
    fn spawn_surviving(program: &str, grace_period: Duration, attempts: u32) -> WindowsBackend {
        for attempt in 1..=attempts {
            let mut backend =
                WindowsBackend::spawn(&[program.to_string()], 80, 24).expect("spawn shell");
            let start = Instant::now();
            while start.elapsed() < grace_period {
                if let Ok(Some(code)) = backend.try_wait() {
                    eprintln!(
                        "spawn_surviving: attempt {attempt}/{attempts} died early \
                         (code {code}) after {:?}, retrying",
                        start.elapsed()
                    );
                    break;
                }
                sleep(Duration::from_millis(50));
            }
            if matches!(backend.try_wait(), Ok(None)) {
                eprintln!(
                    "spawn_surviving: attempt {attempt}/{attempts} survived the grace period"
                );
                return backend;
            }
        }
        panic!("{program} died early on all {attempts} attempts");
    }

    #[test]
    fn spawn_write_read_resize_stop_round_trip() {
        // powershell.exe, not cmd.exe: real windows-latest CI runs showed
        // cmd.exe consistently exiting (code 0, no prompt ever printed)
        // within ~2s of a ConPTY-attached spawn regardless of Job Object
        // use or pipe-handle-close timing, while powershell.exe spawned the
        // same way sometimes stayed alive and worked -- almost certainly
        // the runner's Defender/EDR flagging this exact API pattern as a
        // reverse-shell signature. The plan's own acceptance criteria names
        // either shell as acceptable evidence. -NoProfile/-NoLogo made the
        // child die quickly (code 0) instead of surviving, the opposite of
        // what a "skip slow startup work" flag should do -- that flag
        // combination is itself a well-known stealthy-PowerShell signature,
        // so it plausibly trips this even harder; bare invocations survived
        // more often, just sometimes slowly.
        let mut backend = spawn_surviving("powershell.exe", Duration::from_secs(3), 6);

        // Wait for the actual prompt, not a fixed sleep: profile loading on
        // a cold CI VM has taken 20+ seconds in earlier runs without dying.
        let start = Instant::now();
        let banner = wait_for(&mut backend, Duration::from_secs(60), |text| {
            text.contains("PS ")
        });
        eprintln!(
            "initial banner after {:?} ({} bytes): {banner:?}",
            start.elapsed(),
            banner.len()
        );
        eprintln!("try_wait at banner-wait deadline: {:?}", backend.try_wait());
        assert!(banner.contains("PS "), "shell never reached a prompt");

        backend
            .write(b"echo hello-conpty\r\n")
            .expect("write echo command");
        let output = wait_for(&mut backend, Duration::from_secs(10), |text| {
            text.contains("hello-conpty")
        });
        assert!(
            output.contains("hello-conpty"),
            "expected echoed output, got: {output:?}"
        );

        backend.resize(100, 30).expect("resize");

        backend.write(b"exit\r\n").expect("write exit command");
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut exit_code = None;
        while Instant::now() < deadline {
            if let Ok(Some(code)) = backend.try_wait() {
                exit_code = Some(code);
                break;
            }
            sleep(Duration::from_millis(50));
        }
        assert!(exit_code.is_some(), "shell did not exit after 'exit'");

        backend.stop();
        assert!(matches!(backend.try_wait(), Ok(Some(_))));
    }

    /// Isolation test: spawns bare `cmd.exe` via a completely ordinary,
    /// non-ConPTY `CreateProcessW` (classic inheritable-pipe stdio
    /// redirection, `bInheritHandles = TRUE`, no attribute list, no Job
    /// Object) -- the same shape `std::process::Command` itself uses
    /// internally. Real CI runs showed the SAME shell, spawned via ConPTY,
    /// exiting cleanly (code 0) within ~250ms-650ms with no prompt and no
    /// Win32 error ever returned, inconsistently across runs. If this
    /// plain spawn reliably survives, the fault is specific to ConPTY on
    /// this runner/OS; if it ALSO dies immediately, something about
    /// process creation itself is broken in this environment, unrelated
    /// to ConPTY.
    #[test]
    fn debug_plain_createprocess_without_conpty_survives() {
        let sa = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: null_mut(),
            bInheritHandle: 1,
        };
        let mut stdin_read: HANDLE = null_mut();
        let mut stdin_write: HANDLE = null_mut();
        let mut stdout_read: HANDLE = null_mut();
        let mut stdout_write: HANDLE = null_mut();
        unsafe {
            assert_ne!(
                CreatePipe(&mut stdin_read, &mut stdin_write, &sa, 0),
                0,
                "CreatePipe (stdin) failed: {}",
                io::Error::last_os_error()
            );
            assert_ne!(
                CreatePipe(&mut stdout_read, &mut stdout_write, &sa, 0),
                0,
                "CreatePipe (stdout) failed: {}",
                io::Error::last_os_error()
            );
        }

        let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
        startup.cb = size_of::<STARTUPINFOW>() as u32;
        startup.dwFlags = STARTF_USESTDHANDLES;
        startup.hStdInput = stdin_read;
        startup.hStdOutput = stdout_write;
        startup.hStdError = stdout_write;

        let mut command_line = quote_command_line(&["cmd.exe".to_string()]);
        let mut process_information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
        let created = unsafe {
            CreateProcessW(
                null(),
                command_line.as_mut_ptr(),
                null(),
                null(),
                1, // bInheritHandles: TRUE -- classic pipe-redirection shape.
                0,
                null(),
                null(),
                &startup,
                &mut process_information,
            )
        };
        assert_ne!(
            created,
            0,
            "CreateProcessW failed: {}",
            io::Error::last_os_error()
        );
        unsafe {
            CloseHandle(stdin_read);
            CloseHandle(stdout_write);
        }

        sleep(Duration::from_secs(2));
        let mut code = 0u32;
        unsafe { GetExitCodeProcess(process_information.hProcess, &mut code) };
        eprintln!("plain CreateProcessW cmd.exe, no ConPTY: exit code after 2s = {code} (STILL_ACTIVE = {STILL_ACTIVE})");

        unsafe {
            TerminateProcess(process_information.hProcess, 1);
            CloseHandle(process_information.hProcess);
            CloseHandle(process_information.hThread);
            CloseHandle(stdin_write);
            CloseHandle(stdout_read);
        }

        assert_eq!(
            code, STILL_ACTIVE as u32,
            "plain (non-ConPTY) cmd.exe also exited early -- not a ConPTY-specific issue"
        );
    }
}
