// MODE: DEV
// PACKAGE: PROD
//! How the planning-server daemon and its clients talk: a Unix domain socket
//! where the platform has one, a loopback TCP port everywhere else.
//!
//! The two look the same from outside. `endpoint::socket_path()` names one
//! file; on Unix that file IS the socket, and on Windows it is a small
//! discovery file holding the port the server bound and a per-start nonce.
//! A client reads it, dials the port, and opens with the nonce as its first
//! line, so another local process that merely finds the port open cannot
//! speak to the server. Same shape as ai-text-editor's and interactive-shell's
//! own loopback transports.

use std::io::{self, Read, Write};
use std::path::Path;

#[cfg(unix)]
mod imp {
    use super::*;
    use std::os::unix::net::{UnixListener, UnixStream};

    pub struct Listener(UnixListener);
    pub struct Stream(UnixStream);

    impl Listener {
        pub fn bind(endpoint: &Path) -> io::Result<Self> {
            // A stale socket file from a prior, no-longer-running server
            // prevents bind; a live server would already hold the address.
            let _ = std::fs::remove_file(endpoint);
            UnixListener::bind(endpoint).map(Listener)
        }

        pub fn accept(&self) -> io::Result<Stream> {
            self.0.accept().map(|(stream, _)| Stream(stream))
        }
    }

    impl Stream {
        pub fn connect(endpoint: &Path) -> io::Result<Self> {
            UnixStream::connect(endpoint).map(Stream)
        }

        pub fn try_clone(&self) -> io::Result<Self> {
            self.0.try_clone().map(Stream)
        }

        /// Nothing to prove on a Unix socket: the filesystem already limited
        /// who could reach it.
        pub fn authenticate(&mut self) -> io::Result<bool> {
            Ok(true)
        }
    }

    impl Read for Stream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.0.read(buf)
        }
    }

    impl Write for Stream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
}

#[cfg(not(unix))]
mod imp {
    use super::*;
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::net::{Ipv4Addr, TcpListener, TcpStream};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// How long a fresh connection gets to send its nonce line.
    const NONCE_TIMEOUT: Duration = Duration::from_secs(5);
    /// A nonce line is 32 hex digits; anything much longer is not one.
    const NONCE_LINE_LIMIT: usize = 128;

    pub struct Listener {
        listener: TcpListener,
        nonce: String,
    }

    pub struct Stream {
        stream: TcpStream,
        /// Set on a connection the server accepted and has not yet checked.
        expect: Option<String>,
    }

    /// 128 bits from the standard library's OS-seeded hasher keys, so this
    /// needs no randomness crate of its own.
    fn nonce() -> String {
        let mut out = String::with_capacity(32);
        for salt in 0..2u64 {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u64(salt);
            hasher.write_u32(std::process::id());
            hasher.write_u128(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_nanos())
                    .unwrap_or(0),
            );
            out.push_str(&format!("{:016x}", hasher.finish()));
        }
        out
    }

    impl Listener {
        pub fn bind(endpoint: &Path) -> io::Result<Self> {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
            let port = listener.local_addr()?.port();
            let nonce = nonce();
            std::fs::write(endpoint, format!("{port}\n{nonce}\n"))?;
            Ok(Listener { listener, nonce })
        }

        pub fn accept(&self) -> io::Result<Stream> {
            let (stream, _) = self.listener.accept()?;
            Ok(Stream {
                stream,
                expect: Some(self.nonce.clone()),
            })
        }
    }

    impl Stream {
        pub fn connect(endpoint: &Path) -> io::Result<Self> {
            let text = std::fs::read_to_string(endpoint)?;
            let mut lines = text.lines();
            let port: u16 = lines
                .next()
                .and_then(|line| line.trim().parse().ok())
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "discovery file has no port")
                })?;
            let nonce = lines.next().unwrap_or("").trim().to_string();
            let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port))?;
            writeln!(stream, "{nonce}")?;
            Ok(Stream {
                stream,
                expect: None,
            })
        }

        pub fn try_clone(&self) -> io::Result<Self> {
            Ok(Stream {
                stream: self.stream.try_clone()?,
                expect: None,
            })
        }

        /// Reads the nonce line an accepted connection must open with. Done
        /// here, on the connection's own thread, rather than in `accept`, so
        /// one client that never sends it cannot stall everyone else's.
        /// Bytes are read one at a time so nothing past the newline -- the
        /// client's first request -- is consumed with it.
        pub fn authenticate(&mut self) -> io::Result<bool> {
            let Some(expected) = self.expect.take() else {
                return Ok(true);
            };
            self.stream.set_read_timeout(Some(NONCE_TIMEOUT))?;
            let mut line = Vec::new();
            let mut byte = [0u8; 1];
            while line.len() < NONCE_LINE_LIMIT {
                if self.stream.read(&mut byte)? == 0 {
                    return Ok(false);
                }
                if byte[0] == b'\n' {
                    break;
                }
                line.push(byte[0]);
            }
            self.stream.set_read_timeout(None)?;
            Ok(String::from_utf8_lossy(&line).trim_end() == expected)
        }
    }

    impl Read for Stream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.stream.read(buf)
        }
    }

    impl Write for Stream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.stream.write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.stream.flush()
        }
    }
}

pub use imp::{Listener, Stream};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};

    fn scratch(tag: &str) -> std::path::PathBuf {
        // A Unix socket path must fit sun_path, and a test runner's TMPDIR can
        // be nested deeper than that; nothing else here has such a limit.
        #[cfg(unix)]
        let base = std::path::PathBuf::from("/tmp");
        #[cfg(not(unix))]
        let base = std::env::temp_dir();
        let dir = base.join(format!("ps-transport-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_line_written_by_a_client_reaches_the_server_and_the_answer_comes_back() {
        let dir = scratch("roundtrip");
        let endpoint = dir.join("planning-server.sock");
        let listener = Listener::bind(&endpoint).unwrap();

        let server = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap();
            assert!(stream.authenticate().unwrap(), "a real client must pass");
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            writeln!(stream, "echo:{}", line.trim_end()).unwrap();
        });

        let mut client = Stream::connect(&endpoint).unwrap();
        writeln!(client, "hello").unwrap();
        let mut reply = String::new();
        BufReader::new(client).read_line(&mut reply).unwrap();
        assert_eq!(reply.trim_end(), "echo:hello");
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn binding_again_over_a_stale_endpoint_replaces_it() {
        let dir = scratch("stale");
        let endpoint = dir.join("planning-server.sock");
        drop(Listener::bind(&endpoint).unwrap());
        let listener = Listener::bind(&endpoint).unwrap();
        let server = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap();
            stream.authenticate().unwrap();
        });
        drop(Stream::connect(&endpoint).unwrap());
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(not(unix))]
    #[test]
    fn a_connection_that_lies_about_the_nonce_is_refused() {
        use std::net::{Ipv4Addr, TcpStream};
        let dir = scratch("wrong-nonce");
        let endpoint = dir.join("planning-server.sock");
        let listener = Listener::bind(&endpoint).unwrap();
        let port: u16 = std::fs::read_to_string(&endpoint)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let server = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap();
            stream.authenticate().unwrap()
        });
        let mut liar = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        writeln!(liar, "not-the-nonce").unwrap();
        assert!(!server.join().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
