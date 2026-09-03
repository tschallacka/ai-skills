// MODE: DEV
// PACKAGE: PROD
//! Server-owned coordination records for resolving agent editor tabs.

use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub token_id: String,
    pub tab_uuid: String,
    pub server_generation: String,
    pub endpoint: String,
    pub pid: u32,
    pub start_time_ns: u64,
    pub agent_id: Option<String>,
    pub auth_token: Option<String>,
    pub session_token: String,
}

impl SessionRecord {
    fn from_value(value: &Value) -> Option<Self> {
        Some(Self {
            token_id: value.get("token_id")?.as_str()?.to_owned(),
            tab_uuid: value.get("tab_uuid")?.as_str()?.to_owned(),
            server_generation: value.get("server_generation")?.as_str()?.to_owned(),
            endpoint: value.get("endpoint")?.as_str()?.to_owned(),
            pid: value.get("pid")?.as_u64()?.try_into().ok()?,
            start_time_ns: value.get("start_time_ns")?.as_u64()?,
            agent_id: value
                .get("agent_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            auth_token: value
                .get("auth_token")
                .and_then(Value::as_str)
                .map(str::to_owned),
            session_token: value.get("session_token")?.as_str()?.to_owned(),
        })
    }

    fn value(&self) -> Value {
        json!({
            "token_id": self.token_id,
            "tab_uuid": self.tab_uuid,
            "server_generation": self.server_generation,
            "endpoint": self.endpoint,
            "pid": self.pid,
            "start_time_ns": self.start_time_ns,
            "agent_id": self.agent_id,
            "auth_token": self.auth_token,
            "session_token": self.session_token,
        })
    }
}

pub fn metadata_root() -> PathBuf {
    std::env::var_os("TSCH_AI_EDITOR_METADATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config/tsch-ai-skills/editor"))
        })
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(|home| PathBuf::from(home).join("codex/tsch-ai-skills/editor"))
        })
        .unwrap_or_else(|| std::env::temp_dir().join("tsch-ai-skills/editor"))
}

pub fn registry_path() -> PathBuf {
    metadata_root().join("sessions.json")
}

pub fn register(record: &SessionRecord) -> io::Result<()> {
    let path = registry_path();
    let _lock = RegistryLock::acquire(&path)?;
    let mut records = read_records(&path)?;
    records.retain(|candidate| {
        candidate.token_id != record.token_id
            && (candidate.endpoint != record.endpoint
                || candidate.server_generation == record.server_generation)
    });
    records.push(record.clone());
    write_records(&path, &records)
}

pub fn unregister(session_token: &str) -> io::Result<()> {
    let path = registry_path();
    let _lock = RegistryLock::acquire(&path)?;
    let token_id = blake3::hash(session_token.as_bytes()).to_hex().to_string();
    let mut records = read_records(&path)?;
    records.retain(|record| record.token_id != token_id);
    write_records(&path, &records)
}

pub fn resolve(identity: &str) -> Result<SessionRecord, String> {
    let path = registry_path();
    let records =
        read_records(&path).map_err(|error| format!("cannot read session registry: {error}"))?;
    let exact: Vec<_> = records
        .iter()
        .filter(|record| record.token_id == identity)
        .cloned()
        .collect();
    let mut candidates: Vec<_> = if exact.len() == 1 {
        exact.clone()
    } else {
        records
            .into_iter()
            .filter(|record| record.agent_id.as_deref() == Some(identity))
            .collect()
    };
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.start_time_ns));
    if candidates.is_empty() {
        return Err(format!(
            "session_stale: no live session record matches {identity}"
        ));
    }
    if exact.is_empty() && candidates.len() > 1 {
        return Err(format!(
            "session_ambiguous: multiple sessions match {identity}; choose an explicit token_id"
        ));
    }
    let record = candidates.remove(0);
    if !endpoint_is_reachable(&record.endpoint) {
        return Err(format!(
            "session_stale: session {} is no longer reachable",
            record.token_id
        ));
    }
    Ok(record)
}

fn read_records(path: &Path) -> io::Result<Vec<SessionRecord>> {
    let Ok(content) = fs::read_to_string(path) else {
        return Ok(Vec::new());
    };
    let value: Value = serde_json::from_str(&content).map_err(io::Error::other)?;
    Ok(value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(SessionRecord::from_value)
        .collect())
}

fn write_records(path: &Path, records: &[SessionRecord]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
    }
    let temporary = path.with_extension(format!("json.tmp-{}", std::process::id()));
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&records.iter().map(SessionRecord::value).collect::<Vec<_>>())
            .map_err(io::Error::other)?,
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(temporary, path)
}

struct RegistryLock {
    path: PathBuf,
}

impl RegistryLock {
    fn acquire(registry: &Path) -> io::Result<Self> {
        let path = registry.with_extension("json.lock");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        for _ in 0..500 {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(file) => {
                    let _ = file.set_len(0);
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    if let Ok(metadata) = fs::metadata(&path) {
                        if metadata
                            .modified()
                            .ok()
                            .and_then(|modified| modified.elapsed().ok())
                            .is_some_and(|age| age > std::time::Duration::from_secs(30))
                        {
                            let _ = fs::remove_file(&path);
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "session registry is busy",
        ))
    }
}

impl Drop for RegistryLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn endpoint_is_reachable(endpoint: &str) -> bool {
    if let Some(path) = endpoint.strip_prefix("unix:") {
        #[cfg(unix)]
        return std::os::unix::net::UnixStream::connect(path).is_ok();
        #[cfg(not(unix))]
        return false;
    }
    let address = endpoint.strip_prefix("tcp:").unwrap_or(endpoint);
    std::net::TcpStream::connect(address).is_ok()
}

pub fn new_record(
    endpoint: &str,
    server_generation: &str,
    session_token: &str,
    auth_token: Option<&str>,
    agent_id: Option<String>,
) -> SessionRecord {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let tab_uuid = tab_uuid_for(session_token, server_generation);
    let token_id = blake3::hash(session_token.as_bytes()).to_hex().to_string();
    SessionRecord {
        token_id,
        tab_uuid,
        server_generation: server_generation.to_owned(),
        endpoint: endpoint.to_owned(),
        pid: std::process::id(),
        start_time_ns: now.as_nanos() as u64,
        agent_id,
        auth_token: auth_token.map(str::to_owned),
        session_token: session_token.to_owned(),
    }
}

pub fn tab_uuid_for(session_token: &str, server_generation: &str) -> String {
    blake3::hash(format!("{session_token}:{server_generation}").as_bytes())
        .to_hex()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_round_trips_without_losing_identity_fields() {
        let record = new_record(
            "unix:/tmp/editor.sock",
            "generation",
            "secret",
            None,
            Some("agent".into()),
        );
        let value = record.value();
        assert_eq!(SessionRecord::from_value(&value), Some(record));
    }

    #[test]
    fn malformed_registry_rows_are_ignored() {
        assert!(SessionRecord::from_value(&json!({"token_id": "incomplete"})).is_none());
    }
}
