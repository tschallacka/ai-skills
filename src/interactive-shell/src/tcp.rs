// MODE: DEV
// PACKAGE: PROD
//! A loopback-TCP `Listener`/`Transport` for the crate's shared `client()`/
//! `run()` dispatch, available on every platform: the ONLY transport on
//! Windows (there is no Unix domain socket there -- T84 goal 3), and an
//! explicit `--tcp` opt-in alongside `posix.rs`'s Unix-domain-socket
//! transport on Unix, for a sandbox that lets a program run but blocks
//! `AF_UNIX` socket creation for it (observed directly: codex's own command
//! sandbox, even with Docker's own confinement fully opened up, refused
//! every Unix-socket bind/connect attempt while loopback TCP worked).
//!
//! Nothing here is platform-specific -- no `windows-sys` call, no `libc`
//! call -- so unlike `posix.rs`/`windows.rs` this module compiles and runs
//! unconditionally.
use crate::{Listener, Transport};
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::time::Duration;

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
/// value a local discovery file conveys is already the same trust boundary a
/// Unix socket's 0600 permission bit provides on the default transport.
fn nonce() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| error.to_string())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Reads the port+nonce `TcpTransportListener::bind` wrote to `socket`, if
/// anything parses. `None` on ANY failure -- a missing file, a real Unix
/// socket special file (fails fast and portably: opening a bound
/// `AF_UNIX` path as a plain file returns `ENXIO` on Linux, confirmed
/// directly), or malformed content -- which is exactly the signal
/// `connect_in_directory` (lib.rs) needs to fall back to the Unix-domain
/// transport on Unix instead of committing to a TCP dial that was never
/// going to succeed.
pub(crate) fn parse_discovery(socket: &Path) -> Option<(u16, String)> {
    let contents = fs::read_to_string(socket).ok()?;
    let mut lines = contents.lines();
    let port: u16 = lines.next()?.parse().ok()?;
    let nonce = lines.next()?.to_owned();
    Some((port, nonce))
}

/// Connects over loopback TCP and sends the nonce as the first line so
/// `TcpTransportListener::accept` admits it.
pub(crate) fn dial(port: u16, nonce: &str) -> Result<TcpStream, String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).map_err(|error| error.to_string())?;
    writeln!(stream, "{nonce}").map_err(|error| error.to_string())?;
    Ok(stream)
}

/// A connected, nonce-verified TCP client: the raw `TcpStream` plus whatever
/// bytes `accept()`'s own nonce-line read pulled off the wire ALONGSIDE the
/// nonce.
///
/// `BufReader::read_line` reads in whatever chunks the kernel hands back, not
/// one byte at a time -- on a fast loopback connection, a client's nonce
/// `writeln!` and its very next `write_all` (the real request) routinely
/// arrive in the SAME read, before `accept()` ever gets to look at either.
/// Handing back the bare `TcpStream` once the nonce line was found silently
/// dropped every byte already sitting in that `BufReader`'s own internal
/// buffer -- since the `BufReader` was a temporary, they went out of scope
/// with it -- so the first real request after a connection landed in one
/// read never reached `client()`'s own read loop at all. Confirmed directly:
/// this is exactly what `tcp_transport_serves_screen_events_over_a_discovery_file`
/// hit before this fix, on the very first exchange, deterministically on
/// loopback. `leftover` is drained by `Read::read` before ever touching the
/// live socket again.
pub struct TcpConnection {
    stream: TcpStream,
    leftover: Vec<u8>,
    leftover_pos: usize,
}

impl Read for TcpConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.leftover_pos < self.leftover.len() {
            let n = buf.len().min(self.leftover.len() - self.leftover_pos);
            buf[..n].copy_from_slice(&self.leftover[self.leftover_pos..self.leftover_pos + n]);
            self.leftover_pos += n;
            return Ok(n);
        }
        self.stream.read(buf)
    }
}

impl Write for TcpConnection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

impl Transport for TcpConnection {
    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.stream.set_read_timeout(dur)
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.stream.shutdown(how)
    }
}

/// A loopback `TcpListener` plus the nonce `bind()` generated and wrote out,
/// checked on every `accept()`. The only `Listener` on Windows; an explicit
/// `--tcp` opt-in alternative to `PosixListener` on Unix.
pub struct TcpTransportListener {
    listener: TcpListener,
    nonce: String,
    /// Where `bind()` wrote the port and nonce, so `drop` can take it away.
    discovery: std::path::PathBuf,
}

impl Listener for TcpTransportListener {
    type Stream = TcpConnection;

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
        Ok(TcpTransportListener {
            listener,
            nonce,
            discovery: socket.to_path_buf(),
        })
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
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        let read = reader.read_line(&mut line);
        // Whatever the nonce-line read pulled in past the newline -- see
        // TcpConnection's own doc comment -- must travel with the
        // connection, not vanish with this temporary BufReader.
        let leftover = reader.buffer().to_vec();
        let stream = reader.into_inner();
        if read.is_ok() && line.trim_end() == self.nonce {
            stream.set_read_timeout(None)?;
            Ok(Some(TcpConnection {
                stream,
                leftover,
                leftover_pos: 0,
            }))
        } else {
            // Wrong or missing nonce, or the line never arrived in time:
            // drop it silently, exactly like a rejected connection attempt
            // never reaching client() on the Unix side.
            Ok(None)
        }
    }
}

impl Drop for TcpTransportListener {
    fn drop(&mut self) {
        // The OS reclaims the ephemeral port on process exit, but the
        // discovery file would outlive it and point every later client at a
        // dead port -- and make a restart look ready the moment it begins,
        // because the file already exists. Remove it, but only if it is still
        // the one this listener wrote: a newer session for the same path
        // rewrites it with its own nonce and must keep it.
        if fs::read_to_string(&self.discovery).is_ok_and(|text| text.contains(&self.nonce)) {
            let _ = fs::remove_file(&self.discovery);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hand-rolled per-test temp dirs (std::env::temp_dir() + pid + a
    // test-specific prefix), matching posix.rs's own #[cfg(test)] convention
    // rather than pulling in the tempfile crate this crate does not
    // otherwise depend on.
    fn scratch_dir(prefix: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("is-tcp-{prefix}-{}", std::process::id()));
        make_usable(&dir);
        dir
    }

    /// Creates `dir` and makes sure it is searchable and writable by us.
    /// posix.rs binds its sockets under a temporary `umask(0o177)`, which is
    /// process-wide, so a directory another test thread creates in that window
    /// comes out mode 0600 and every write into it fails with EACCES. chmod is
    /// not subject to the umask.
    fn make_usable(dir: &std::path::Path) {
        fs::create_dir_all(dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
        }
    }

    #[test]
    fn bind_and_connect_round_trip_a_byte() {
        let dir = scratch_dir("roundtrip");
        let discovery = dir.join("session.sock");
        let listener = TcpTransportListener::bind(&discovery).unwrap();

        let (port, nonce) = parse_discovery(&discovery).expect("discovery file should parse");
        let mut client = dial(port, &nonce).unwrap();

        let mut server_end = loop {
            if let Some(stream) = listener.accept().unwrap() {
                break stream;
            }
        };
        server_end.write_all(b"hello").unwrap();
        drop(server_end);

        let mut buf = [0u8; 5];
        client.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"hello");

        drop(listener);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_wrong_nonce_is_never_accepted() {
        let dir = scratch_dir("wrong-nonce");
        let discovery = dir.join("session.sock");
        let listener = TcpTransportListener::bind(&discovery).unwrap();
        let (port, _nonce) = parse_discovery(&discovery).expect("discovery file should parse");

        let _client = dial(port, "not-the-real-nonce").unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(listener.accept().unwrap().is_none());

        drop(listener);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_discovery_rejects_a_missing_file() {
        let dir = scratch_dir("missing-file");
        assert!(parse_discovery(&dir.join("nope.sock")).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_discovery_rejects_malformed_content() {
        let dir = scratch_dir("malformed");
        let discovery = dir.join("session.sock");
        fs::write(&discovery, "not-a-port\n").unwrap();
        assert!(parse_discovery(&discovery).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn parse_discovery_rejects_a_real_unix_socket_path() {
        // A bound AF_UNIX path must fit sun_path (104 bytes on macOS, 108 on
        // Linux), and a test runner's $TMPDIR can be nested deeper than that.
        // This one test needs a real socket, so it falls back to a short
        // directory rather than failing on a limit that is not under test.
        let mut dir = scratch_dir("real-unix-socket");
        if dir.join("session.sock").as_os_str().len() >= 100 {
            let _ = fs::remove_dir_all(&dir);
            dir = std::path::PathBuf::from(format!("/tmp/is-tcp-sock-{}", std::process::id()));
            make_usable(&dir);
        }
        let socket = dir.join("session.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        assert!(parse_discovery(&socket).is_none());
        drop(listener);
        let _ = fs::remove_dir_all(&dir);
    }
}
