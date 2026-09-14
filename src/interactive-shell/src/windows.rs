// MODE: DEV
// PACKAGE: PROD
//! The Windows implementation of the crate's `Backend` trait (defined in
//! `lib.rs`): PTY spawn/stop/resize/read/write via ConPTY
//! (`CreatePseudoConsole`/`ResizePseudoConsole`/`ClosePseudoConsole`) and a
//! Job Object (`CreateJobObjectW`/`AssignProcessToJobObject`/
//! `TerminateJobObject`) in place of POSIX's openpty/ioctl/setsid/kill. The
//! confirmed windows-sys API surface this binds against is recorded in
//! `.plans/windows-interactive-shell/02-conpty-backend/working-context.md`.
use crate::{Backend, Listener, Transport};
use std::ffi::c_void;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::mem::size_of;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::ptr::{null, null_mut};
use std::time::Duration;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, STILL_ACTIVE};
#[cfg(test)]
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows_sys::Win32::System::Console::{
    ClosePseudoConsole, CreatePseudoConsole, FreeConsole, ResizePseudoConsole, COORD, HPCON,
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
        // By default a console process inherits its parent's console
        // (independent of bInheritHandles, which only governs the HANDLE
        // table); if this process is itself attached to one -- as it can
        // be here, launched via a shell that has its own conhost session
        // -- the ConPTY-attached child can bind to THAT console instead of
        // the pseudo one, rendering its prompt into it rather than our
        // pipes. FreeConsole() detaches from any inherited console before
        // CreatePseudoConsole runs; it is a harmless no-op when there
        // isn't one to detach from.
        unsafe { FreeConsole() };

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

// --- Transport: loopback TCP + a per-start nonce (goal 3, W14/W15) ----------
//
// The Unix socket transport does not port to Windows: there is no Unix
// domain socket, and its sun_path-length concerns are moot anyway. This
// mirrors ai-text-editor's already-shipped, already-CI-proven pattern for
// non-Unix platforms: bind a loopback TCP listener on an ephemeral port,
// write that port plus a per-start nonce to the SAME path `session_socket()`
// already computes (on Unix, a literal socket path; here, just a small text
// file -- `run()`'s `socket: PathBuf` parameter is reinterpreted, not
// repurposed, per the Listener trait's own doc comment), and require a
// connecting client to prove it read that file by echoing the nonce back as
// the first line, before its connection is handed to the shared client()/
// run() dispatch. A Unix socket's 0600 file permission is the access
// control there; a random per-start nonce plays the same role here, where
// there is no filesystem-permission equivalent for a TCP port.
//
// `install_interrupt_handler()` stays a no-op: no work unit in this plan
// wires up a real `SetConsoleCtrlHandler`-based one, so Ctrl-C handling on
// Windows remains a known gap, not silently claimed as done.
pub(crate) fn install_interrupt_handler() {}

impl Transport for TcpStream {
    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        TcpStream::set_read_timeout(self, dur)
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        TcpStream::shutdown(self, how)
    }
}

/// An unpredictable, hex-encoded challenge a connecting client must echo
/// back as its first line. Loopback-only and single-user, so this does not
/// need the HMAC challenge/response ai-text-editor's TCP transport uses for
/// its own, differently-shaped multi-client server -- a random per-start
/// value a local socket file conveys is already the same trust boundary a
/// Unix socket's 0600 permission bit provides on the POSIX side.
fn nonce() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| error.to_string())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// The Windows counterpart to `posix::connect_in_directory`: reads the
/// port+nonce `WindowsListener::bind` wrote to `socket`, connects over
/// loopback TCP, and sends the nonce as the first line so `accept()` admits
/// it. Returns the CONCRETE `TcpStream`, not `impl Transport`: this is the
/// one function `bin/interactive-shell-input.rs` (frozen, AR-06) calls
/// unconditionally, and calling a trait method via dot-syntax on an opaque
/// `impl Trait` value needs that trait in scope at the call site, which the
/// frozen binary can never import. `TcpStream`, exactly like `UnixStream`
/// on the existing Unix path, exposes `shutdown`/`set_read_timeout` as
/// INHERENT methods needing no `Transport` import at all.
pub fn connect_in_directory(socket: &Path) -> Result<TcpStream, String> {
    let contents = fs::read_to_string(socket)
        .map_err(|error| format!("read discovery file {}: {error}", socket.display()))?;
    let mut lines = contents.lines();
    let port: u16 = lines
        .next()
        .ok_or_else(|| format!("discovery file {} is missing a port", socket.display()))?
        .parse()
        .map_err(|error| {
            format!(
                "discovery file {} has an invalid port: {error}",
                socket.display()
            )
        })?;
    let nonce = lines
        .next()
        .ok_or_else(|| format!("discovery file {} is missing a nonce", socket.display()))?;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).map_err(|error| error.to_string())?;
    writeln!(stream, "{nonce}").map_err(|error| error.to_string())?;
    Ok(stream)
}

/// The Windows counterpart to `PosixListener`: a loopback `TcpListener` plus
/// the nonce `bind()` generated and wrote out, checked on every `accept()`.
pub struct WindowsListener {
    listener: TcpListener,
    nonce: String,
}

impl Listener for WindowsListener {
    type Stream = TcpStream;

    fn bind(socket: &Path) -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|error| error.to_string())?;
        listener
            .set_nonblocking(true)
            .map_err(|error| error.to_string())?;
        let port = listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .port();
        let nonce = nonce()?;
        fs::write(socket, format!("{port}\n{nonce}\n"))
            .map_err(|error| format!("write discovery file {}: {error}", socket.display()))?;
        Ok(WindowsListener { listener, nonce })
    }

    fn accept(&self) -> io::Result<Option<Self::Stream>> {
        let stream = match self.listener.accept() {
            Ok((stream, _addr)) => stream,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(error) => return Err(error),
        };
        // Blocking, like PosixListener's own accept() explicitly sets
        // (Linux already hands back a blocking socket regardless; macOS
        // does not) -- but bounded just for the nonce line, so a connection
        // that never sends one can't hang the shared accept loop.
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut line = String::new();
        let read = BufReader::new(&stream).read_line(&mut line);
        if read.is_ok() && line.trim_end() == self.nonce {
            stream.set_read_timeout(None)?;
            Ok(Some(stream))
        } else {
            // Wrong or missing nonce, or the line never arrived in time:
            // drop it silently, exactly like a rejected connection attempt
            // never reaching client() on the Unix side.
            Ok(None)
        }
    }
}

impl Drop for WindowsListener {
    fn drop(&mut self) {
        // No cleanup needed: the OS reclaims the ephemeral port on process
        // exit, and the discovery file is simply overwritten by the next
        // start -- unlike PosixListener, there is no socket-file identity
        // to remove.
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
    /// sometimes alive past 20s). Ruled out as the cause: Defender's
    /// real-time/IOAV/behavior monitoring disabled and confirmed off via
    /// `Get-MpComputerStatus` (no change); an explicit `WinSta0\Default`
    /// desktop (no change); `FreeConsole()` before `CreatePseudoConsole`
    /// (no change) -- despite catching this process's own inherited
    /// console rendering the child's real prompt directly into the CI
    /// step's log, the smoking-gun symptom of a known, externally
    /// reported, unresolved ConPTY bug on recent Windows builds (see
    /// `working-context.md`'s "Known issue" section). Not a bug in
    /// `spawn()` itself -- a parallel isolation test proves plain
    /// (non-ConPTY) `CreateProcessW` survives reliably in this same
    /// environment, and every run where the ConPTY child DID survive
    /// showed fully correct output. Retries a fresh spawn on an early
    /// death since it does sometimes work.
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

    /// Goal 2's W13 verification. On this crate's target CI (real
    /// windows-latest runners as of Windows Server 2025, build 10.0.26100,
    /// image windows-2025-vs2026), this test is flaky-to-failing: `cmd.exe`
    /// via ConPTY dies (code 0, no prompt) almost immediately on this OS
    /// build essentially every time, and `powershell.exe` does so
    /// intermittently. See `spawn_surviving`'s doc comment for the full
    /// elimination trail; the short version is that this matches a known,
    /// externally reported, currently unresolved Windows/ConPTY bug on
    /// recent Windows builds (github.com/egarim/telekinesis#49 has the
    /// identical symptom on a different machine/architecture, using
    /// Microsoft's own reference implementation), not a defect in
    /// `windows.rs`. `#[ignore]`d (goal 3, W16) so this doesn't fail
    /// `cargo test --workspace` on every future PR's Windows leg once the
    /// crate stops being excluded from it -- `debug_plain_createprocess_
    /// without_conpty_survives`, below, is the always-run Windows test for
    /// this module. Run explicitly with `cargo test -- --ignored` (real
    /// Windows only) to re-check whether a platform update has fixed this.
    #[ignore = "ConPTY intermittently/consistently fails to bind the child \
                to the pseudo console on this runner's OS (Windows Server \
                2025); see working-context.md's Known issue section and \
                github.com/egarim/telekinesis#49"]
    #[test]
    fn spawn_write_read_resize_stop_round_trip() {
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
