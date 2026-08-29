// MODE: DEV
// PACKAGE: PROD
//! One shape for two transports, so nothing above this module knows which it is.
//!
//! The protocol is a stream of lines and does not care what carries them, but
//! `TcpStream` and `UnixStream` share no trait that gives out an owned clone —
//! and the hub needs two independent handles per client, one for replies on the
//! reading thread and one for pushes from another client's thread. So each
//! transport is opened here and handed up as boxed halves.
//!
//! Timeouts are set here rather than at each call site because a missing one is
//! invisible until something hangs: a client whose server dies mid-reply waits
//! forever, and an agent waiting on that client waits with it.

use crate::config::Transport;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

/// How long a client waits on a reply. Far longer than a loopback exchange, far
/// shorter than a stuck agent going unnoticed.
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// One client connection, already split into the handles the hub needs.
pub struct Conn {
    pub read: Box<dyn Read + Send>,
    /// Replies, written by the thread that owns the connection.
    pub write: Box<dyn Write + Send>,
    /// Pushes, written by whichever thread broadcasts. A separate handle, so a
    /// push never interleaves with a half-written reply.
    pub push: Box<dyn Write + Send>,
}

/// A bound endpoint.
pub enum Listener {
    Tcp(TcpListener),
    #[cfg(unix)]
    Unix(UnixListener),
}

/// Why an endpoint could not be bound. The caller distinguishes these because
/// "someone else has it" and "the path is wrong" need different advice.
pub enum BindError {
    InUse,
    Other(String),
}

impl Listener {
    /// Bind the transport.
    ///
    /// For a unix socket a leftover file is removed first. That is safe only
    /// because the caller already holds the home lock — which is what proves no
    /// live server owns this path — and it is necessary because a socket file
    /// outlives a killed process and would otherwise make every later start
    /// fail with EADDRINUSE.
    pub fn bind(transport: &Transport) -> Result<Listener, BindError> {
        match transport {
            Transport::Tcp { bind, port } => match TcpListener::bind((bind.as_str(), *port)) {
                Ok(l) => Ok(Listener::Tcp(l)),
                Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => Err(BindError::InUse),
                Err(e) => Err(BindError::Other(e.to_string())),
            },
            #[cfg(unix)]
            Transport::Socket(path) => {
                if path.exists() {
                    let _ = std::fs::remove_file(path);
                }
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match UnixListener::bind(path) {
                    Ok(l) => {
                        restrict_socket(path);
                        Ok(Listener::Unix(l))
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => Err(BindError::InUse),
                    Err(e) => Err(BindError::Other(format!("{}: {e}", path.display()))),
                }
            }
            #[cfg(not(unix))]
            Transport::Socket(path) => Err(BindError::Other(format!(
                "unix sockets are not available on this platform, so {} cannot be served; \
                 record transport=tcp instead",
                path.display()
            ))),
        }
    }

    /// The port actually bound, which for `port=0` is the kernel's choice and
    /// for a unix socket is nothing at all.
    pub fn bound_port(&self) -> Option<u16> {
        match self {
            Listener::Tcp(l) => l.local_addr().ok().map(|a| a.port()),
            #[cfg(unix)]
            Listener::Unix(_) => None,
        }
    }

    /// Accept one client. `Ok(None)` means this connection failed but the
    /// listener is still good, which is the normal shape of EMFILE and
    /// ECONNABORTED — dropping one client beats exiting and dropping them all.
    pub fn accept(&self) -> Result<Option<Conn>, String> {
        match self {
            Listener::Tcp(l) => match l.accept() {
                Ok((s, _)) => Ok(split_tcp(s)),
                Err(e) => Err(e.to_string()),
            },
            #[cfg(unix)]
            Listener::Unix(l) => match l.accept() {
                Ok((s, _)) => Ok(split_unix(s)),
                Err(e) => Err(e.to_string()),
            },
        }
    }
}

fn split_tcp(s: TcpStream) -> Option<Conn> {
    let read = s.try_clone().ok()?;
    let push = s.try_clone().ok()?;
    Some(Conn {
        read: Box::new(read),
        write: Box::new(s),
        push: Box::new(push),
    })
}

#[cfg(unix)]
fn split_unix(s: UnixStream) -> Option<Conn> {
    let read = s.try_clone().ok()?;
    let push = s.try_clone().ok()?;
    Some(Conn {
        read: Box::new(read),
        write: Box::new(s),
        push: Box::new(push),
    })
}

/// A unix socket's reach is its file mode, so say what it is rather than
/// inheriting whatever the umask happened to be: owner only.
#[cfg(unix)]
fn restrict_socket(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

/// A client's half of the same abstraction: somewhere to write the request and
/// somewhere to read the replies.
pub struct Client {
    pub read: Box<dyn Read + Send>,
    pub write: Box<dyn Write + Send>,
}

/// Connect to a transport, with timeouts set.
///
/// `tail` asks for no read timeout: idling is the normal state of a
/// subscription, and a ten-second timeout would end it every quiet ten seconds.
pub fn connect(transport: &Transport, tail: bool) -> Result<Client, String> {
    match transport {
        Transport::Tcp { bind, port } => {
            // A wildcard bind is not a host: see Transport::dial_host.
            let host = Transport::dial_host(bind);
            let s = TcpStream::connect((host, *port))
                .map_err(|e| format!("cannot reach {host}:{port}: {e}"))?;
            s.set_read_timeout(if tail { None } else { Some(CLIENT_TIMEOUT) })
                .map_err(|e| e.to_string())?;
            s.set_write_timeout(Some(CLIENT_TIMEOUT))
                .map_err(|e| e.to_string())?;
            let read = s.try_clone().map_err(|e| e.to_string())?;
            Ok(Client {
                read: Box::new(read),
                write: Box::new(s),
            })
        }
        #[cfg(unix)]
        Transport::Socket(path) => {
            let s = UnixStream::connect(path)
                .map_err(|e| format!("cannot reach {}: {e}", path.display()))?;
            s.set_read_timeout(if tail { None } else { Some(CLIENT_TIMEOUT) })
                .map_err(|e| e.to_string())?;
            s.set_write_timeout(Some(CLIENT_TIMEOUT))
                .map_err(|e| e.to_string())?;
            let read = s.try_clone().map_err(|e| e.to_string())?;
            Ok(Client {
                read: Box::new(read),
                write: Box::new(s),
            })
        }
        #[cfg(not(unix))]
        Transport::Socket(path) => Err(format!(
            "unix sockets are not available on this platform, so {} cannot be reached",
            path.display()
        )),
    }
}
