// MODE: DEV
// PACKAGE: PROD
use crate::auth;
use crate::protocol::{validate_ndjson, Envelope, ProtocolError};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::net::UnixStream;

#[derive(Debug, Clone)]
pub enum Endpoint {
    #[cfg(unix)]
    Unix(PathBuf),
    Tcp(String),
}

#[derive(Debug, Clone)]
pub struct Session {
    pub endpoint: Endpoint,
    pub auth_token: Option<String>,
    pub session_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EndpointMetadata {
    pub endpoint: Endpoint,
    pub pid: Option<u32>,
    pub generation: Option<String>,
}

impl Endpoint {
    pub fn parse(value: &str) -> Self {
        #[cfg(unix)]
        if let Some(path) = value.strip_prefix("unix:") {
            return Self::Unix(PathBuf::from(path));
        }
        Self::Tcp(value.strip_prefix("tcp:").unwrap_or(value).to_owned())
    }

    pub fn display(&self) -> String {
        match self {
            #[cfg(unix)]
            Self::Unix(path) => format!("unix:{}", path.display()),
            Self::Tcp(address) => format!("tcp:{address}"),
        }
    }

    /// Whether anything can accept a connection on this endpoint *right
    /// now*. An endpoint file surviving a killed server still parses and
    /// reads fine while its socket has no listener, so discovery callers
    /// must probe before handing it to a request — otherwise `open` never
    /// autostarts a replacement and every later call dies on a connection
    /// error naming nothing the agent can act on.
    pub fn is_live(&self) -> bool {
        match self {
            #[cfg(unix)]
            Self::Unix(path) => UnixStream::connect(path).is_ok(),
            Self::Tcp(address) => address
                .to_socket_addrs()
                .ok()
                .and_then(|mut addresses| addresses.next())
                .is_some_and(|address| {
                    TcpStream::connect_timeout(&address, std::time::Duration::from_millis(500))
                        .is_ok()
                }),
        }
    }
}

/// The canonical form a path would have if it existed: a missing file
/// resolves through its existing parent directory. `fs::canonicalize`
/// refuses nonexistent paths wholesale, which made opening a file before
/// its first save impossible — the discovery key, the tab key, and the
/// server all need one stable path for a path that is about to exist.
pub fn canonical_or_near(path: &Path) -> io::Result<PathBuf> {
    match fs::canonicalize(path) {
        Ok(canonical) => Ok(canonical),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty());
            let name = path.file_name().ok_or(error)?;
            Ok(fs::canonicalize(parent.unwrap_or_else(|| Path::new(".")))?.join(name))
        }
        Err(error) => Err(error),
    }
}

pub fn endpoint_for_file(path: &Path) -> PathBuf {
    let configured_root = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("tsch-ai-skills-editor");
    let identity = canonical_or_near(path).unwrap_or_else(|_| path.to_path_buf());
    let key = blake3::hash(identity.to_string_lossy().as_bytes())
        .to_hex()
        .to_string();
    // Unix domain socket paths have a small platform-defined limit. Keep a
    // collision-resistant prefix so long runtime directories still work.
    let key = &key[..32];
    let configured = configured_root.join(format!("{key}.endpoint"));
    if configured.to_string_lossy().len() < 96 {
        return configured;
    }
    #[cfg(unix)]
    let short_root = PathBuf::from("/tmp/tsch-ai-skills-editor");
    #[cfg(not(unix))]
    let short_root = std::env::temp_dir().join("tsch-ai-skills-editor");
    short_root.join(format!("{key}.endpoint"))
}

pub fn socket_for_file(path: &Path) -> PathBuf {
    let discovery = endpoint_for_file(path);
    discovery.with_extension("sock")
}

pub fn write_endpoint(path: &Path, endpoint: &Endpoint) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
    }
    atomic_write(path, endpoint.display().as_bytes())
}

pub fn read_endpoint(path: &Path) -> io::Result<Endpoint> {
    Ok(read_endpoint_metadata(path)?.endpoint)
}

pub fn write_endpoint_metadata(
    path: &Path,
    endpoint: &Endpoint,
    pid: u32,
    generation: &str,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
    }
    let content = serde_json::to_vec(&json!({
        "endpoint": endpoint.display(),
        "pid": pid,
        "generation": generation,
        "status": "active"
    }))
    .map_err(io::Error::other)?;
    atomic_write(path, &content)
}

pub fn read_endpoint_metadata(path: &Path) -> io::Result<EndpointMetadata> {
    let content = fs::read_to_string(path)?;
    if let Ok(value) = serde_json::from_str::<Value>(&content) {
        if let Some(endpoint) = value.get("endpoint").and_then(Value::as_str) {
            return Ok(EndpointMetadata {
                endpoint: Endpoint::parse(endpoint),
                pid: value
                    .get("pid")
                    .and_then(Value::as_u64)
                    .map(|pid| pid as u32),
                generation: value
                    .get("generation")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            });
        }
    }
    Ok(EndpointMetadata {
        endpoint: Endpoint::parse(content.trim()),
        pid: None,
        generation: None,
    })
}

fn atomic_write(path: &Path, content: &[u8]) -> io::Result<()> {
    let temporary = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::write(&temporary, content)?;
    restrict_private(&temporary)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

pub fn write_session_token(path: &Path, endpoint: &Endpoint) -> io::Result<()> {
    write_session(path, endpoint, None, None)
}

pub fn write_session(
    path: &Path,
    endpoint: &Endpoint,
    auth_token: Option<&str>,
    session_token: Option<&str>,
) -> io::Result<()> {
    // A bare filename has an empty parent; create_dir_all("") is an ENOENT
    // that used to fail the whole `open` (B196). The current directory is
    // the parent the caller meant.
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
    }
    fs::write(
        path,
        serde_json::to_vec(&json!({"endpoint": endpoint.display(), "auth_token": auth_token, "session_token": session_token}))
            .map_err(io::Error::other)?,
    )?;
    restrict_private(path)?;
    Ok(())
}

fn restrict_private(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub fn read_session_token(path: &Path) -> io::Result<Endpoint> {
    Ok(read_session(path)?.endpoint)
}

pub fn read_session(path: &Path) -> io::Result<Session> {
    let content = fs::read_to_string(path)?;
    if let Ok(value) = serde_json::from_str::<Value>(&content) {
        if let Some(endpoint) = value.get("endpoint").and_then(Value::as_str) {
            return Ok(Session {
                endpoint: Endpoint::parse(endpoint),
                auth_token: value
                    .get("auth_token")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                session_token: value
                    .get("session_token")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            });
        }
    }
    Ok(Session {
        endpoint: Endpoint::parse(content.trim()),
        auth_token: None,
        session_token: None,
    })
}

pub fn request(endpoint: &Endpoint, envelope: &Envelope) -> Result<Vec<Value>, String> {
    let line = serde_json::to_vec(envelope).map_err(|error| error.to_string())?;
    let mut framed = line;
    framed.push(b'\n');
    let mut responses = Vec::new();
    match endpoint {
        #[cfg(unix)]
        Endpoint::Unix(path) => {
            let stream = UnixStream::connect(path).map_err(|error| error.to_string())?;
            exchange(stream, &framed, &mut responses)?;
        }
        Endpoint::Tcp(address) => {
            let address = address
                .to_socket_addrs()
                .map_err(|error| error.to_string())?
                .next()
                .ok_or_else(|| "endpoint did not resolve".to_owned())?;
            let stream = TcpStream::connect(address).map_err(|error| error.to_string())?;
            let wire_envelope = Envelope {
                auth_token: None,
                ..envelope.clone()
            };
            let mut wire_request =
                serde_json::to_vec(&wire_envelope).map_err(|error| error.to_string())?;
            wire_request.push(b'\n');
            exchange_tcp(stream, envelope, &wire_request, &mut responses)?;
        }
    }
    // Zero frames is never an answer.
    if responses.is_empty() {
        return Err(format!(
            "the server at {} closed the connection without answering",
            endpoint.display()
        ));
    }
    Ok(responses)
}

fn exchange_tcp(
    stream: TcpStream,
    envelope: &Envelope,
    request: &[u8],
    responses: &mut Vec<Value>,
) -> Result<(), String> {
    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    reader
        .read_until(b'\n', &mut line)
        .map_err(|error| error.to_string())?;
    let challenge = validate_ndjson(&line).map_err(|error| error.to_string())?;
    if challenge.get("type").and_then(Value::as_str) != Some("challenge") {
        return Err("TCP server did not provide an authentication challenge".into());
    }
    let payload = challenge
        .get("payload")
        .and_then(Value::as_object)
        .ok_or_else(|| "authentication challenge has no payload".to_owned())?;
    let nonce = payload
        .get("nonce")
        .and_then(Value::as_str)
        .ok_or_else(|| "authentication challenge has no nonce".to_owned())?;
    let generation = payload
        .get("generation")
        .and_then(Value::as_str)
        .ok_or_else(|| "authentication challenge has no generation".to_owned())?;
    let secret = envelope
        .auth_token
        .as_deref()
        .ok_or_else(|| "TCP endpoints require --auth-token or a saved session token".to_owned())?;
    let nonce = auth::decode_nonce(nonce).map_err(|error| error.to_string())?;
    let auth_envelope = Envelope {
        version: envelope.version,
        request_id: envelope.request_id.clone(),
        method: "authenticate".into(),
        revision: None,
        auth_token: None,
        session_token: None,
        payload: json!({
            "nonce": payload.get("nonce").cloned().unwrap_or(Value::Null),
            "proof": auth::proof(secret.as_bytes(), &nonce, &envelope.request_id, generation)
                .map_err(|error| error.to_string())?
        }),
    };
    let mut auth_bytes = serde_json::to_vec(&auth_envelope).map_err(|error| error.to_string())?;
    auth_bytes.push(b'\n');
    let writer = reader.get_mut();
    writer
        .write_all(&auth_bytes)
        .and_then(|_| writer.write_all(request))
        .and_then(|_| writer.flush())
        .map_err(|error| error.to_string())?;
    loop {
        line.clear();
        let read = reader
            .read_until(b'\n', &mut line)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        let value = validate_ndjson(&line).map_err(|error| error.to_string())?;
        let complete = value.get("type").and_then(Value::as_str) == Some("complete");
        let failed = value.get("type").and_then(Value::as_str) == Some("error");
        responses.push(value);
        if complete || failed {
            break;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn exchange<S: io::Read + Write>(
    mut stream: S,
    request: &[u8],
    responses: &mut Vec<Value>,
) -> Result<(), String> {
    stream
        .write_all(request)
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = reader
            .read_until(b'\n', &mut line)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        let value = validate_ndjson(&line).map_err(|error| error.to_string())?;
        let complete = value.get("type").and_then(Value::as_str) == Some("complete");
        let failed = value.get("type").and_then(Value::as_str) == Some("error");
        responses.push(value);
        if complete || failed {
            break;
        }
    }
    Ok(())
}

pub fn response(request_id: &str, payload: Value) -> Value {
    json!({"version": crate::PROTOCOL_VERSION, "request_id": request_id, "type": "data", "payload": payload})
}

pub fn complete(request_id: &str, generation: &str) -> Value {
    json!({"version": crate::PROTOCOL_VERSION, "request_id": request_id, "type": "complete", "result_generation": generation})
}

pub fn error(request_id: &str, code: &str, message: impl Into<String>) -> Value {
    json!({"version": crate::PROTOCOL_VERSION, "request_id": request_id, "type": "error", "code": code, "message": message.into(), "retryable": false})
}

pub fn error_details(
    request_id: &str,
    code: &str,
    message: impl Into<String>,
    details: Value,
) -> Value {
    json!({"version": crate::PROTOCOL_VERSION, "request_id": request_id, "type": "error", "code": code, "message": message.into(), "details": details, "retryable": false})
}

pub fn validate_request(value: Value) -> Result<Envelope, ProtocolError> {
    let envelope: Envelope =
        serde_json::from_value(value).map_err(|error| ProtocolError::Json(error.to_string()))?;
    envelope.validate()?;
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_metadata_round_trips_and_is_private() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("file.endpoint");
        let endpoint = Endpoint::Tcp("127.0.0.1:43210".into());
        write_endpoint_metadata(&path, &endpoint, 1234, "generation-1").unwrap();

        let metadata = read_endpoint_metadata(&path).unwrap();
        assert_eq!(metadata.endpoint.display(), endpoint.display());
        assert_eq!(metadata.pid, Some(1234));
        assert_eq!(metadata.generation.as_deref(), Some("generation-1"));
        assert!(fs::read_to_string(&path)
            .unwrap()
            .contains("\"status\":\"active\""));
    }

    #[test]
    fn legacy_plain_endpoint_remains_readable() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("file.endpoint");
        fs::write(&path, "tcp:127.0.0.1:43210\n").unwrap();

        let metadata = read_endpoint_metadata(&path).unwrap();
        assert_eq!(metadata.endpoint.display(), "tcp:127.0.0.1:43210");
        assert_eq!(metadata.pid, None);
        assert_eq!(metadata.generation, None);
    }
}
