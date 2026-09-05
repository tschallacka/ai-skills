// MODE: DEV
// PACKAGE: PROD
//! Server-owned coordination records for resolving agent editor tabs.

use serde_json::{json, Value};
use stale_lock::StaleLock;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

/// The session registry's own lock file: same 30s staleness window a
/// crashed writer's lock is reclaimed under everywhere else in this module.
fn acquire_registry_lock(registry: &Path) -> io::Result<StaleLock> {
    StaleLock::acquire(
        &registry.with_extension("json.lock"),
        Duration::from_secs(30),
    )
}

pub fn register(record: &SessionRecord) -> io::Result<()> {
    let path = registry_path();
    let _lock = acquire_registry_lock(&path)?;
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
    let _lock = acquire_registry_lock(&path)?;
    let token_id = blake3::hash(session_token.as_bytes()).to_hex().to_string();
    let mut records = read_records(&path)?;
    records.retain(|record| record.token_id != token_id);
    write_records(&path, &records)
}

/// B189: a server leaving (idle shutdown, last tab closed) takes its tabs'
/// reachability with it, but nothing else ever removed the records they
/// registered — the registry kept growing ghosts that every later identity
/// lookup had to probe and reject. Retire the whole generation on the way
/// out; records of other, still-live servers are untouched.
pub fn retire_generation(generation: &str) -> io::Result<()> {
    retire_generation_at(&registry_path(), generation)
}

fn retire_generation_at(path: &Path, generation: &str) -> io::Result<()> {
    let _lock = acquire_registry_lock(path)?;
    let records = read_records(path)?;
    let kept: Vec<SessionRecord> = records
        .into_iter()
        .filter(|record| record.server_generation != generation)
        .collect();
    write_records(path, &kept)
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

/// Find the workspace-server most recently associated with `identity` —
/// matched the same way `resolve` matches, but tolerant of more than one tab
/// sharing that identity. `resolve` exists to resume one specific,
/// unambiguous tab (`open --agent NAME` with no file to disambiguate with),
/// and rightly refuses to guess when several tabs qualify. This exists for a
/// different question: `open --file X` reconnecting to an agent's already-
/// running server to add or find X as a tab there, where a file-routed
/// request never depends on which tab this lookup happened to return — every
/// tab on one server shares its endpoint, so any reachable record under this
/// identity answers "where is my workspace" correctly, without needing the
/// tab-level guarantee `resolve` provides.
pub fn resolve_workspace(identity: &str) -> Result<SessionRecord, String> {
    let path = registry_path();
    let mut candidates: Vec<SessionRecord> = read_records(&path)
        .map_err(|error| format!("cannot read session registry: {error}"))?
        .into_iter()
        .filter(|record| {
            record.token_id == identity || record.agent_id.as_deref() == Some(identity)
        })
        .collect();
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.start_time_ns));
    let mut tried_endpoints = std::collections::HashSet::new();
    for record in candidates.drain(..) {
        if !tried_endpoints.insert(record.endpoint.clone()) {
            continue; // a more recent tab on this same server already ruled it out
        }
        if endpoint_is_reachable(&record.endpoint) {
            return Ok(record);
        }
    }
    Err(format!(
        "session_stale: no live session record matches {identity}"
    ))
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

fn endpoint_is_reachable(endpoint: &str) -> bool {
    if endpoint.starts_with("unix:") {
        #[cfg(unix)]
        {
            let path = endpoint.strip_prefix("unix:").unwrap_or_default();
            return std::os::unix::net::UnixStream::connect(path).is_ok();
        }
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

    #[test]
    fn retiring_a_generation_clears_its_records_and_keeps_other_servers() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("sessions.json");
        let dying = new_record(
            "unix:/tmp/dying.sock",
            "generation-a",
            "token-a",
            None,
            None,
        );
        let living = new_record(
            "unix:/tmp/living.sock",
            "generation-b",
            "token-b",
            None,
            None,
        );
        write_records(&path, &[dying, living]).unwrap();
        retire_generation_at(&path, "generation-a").unwrap();
        let kept = read_records(&path).unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].server_generation, "generation-b");
    }
}
