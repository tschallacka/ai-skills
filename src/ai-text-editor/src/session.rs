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
    /// T118: the shortest prefix of `tab_uuid` this tab was assigned the
    /// FIRST time it ever registered -- fixed for its whole lifetime.
    /// `register`'s own doc comment explains why: recomputing "the shortest
    /// prefix unambiguous right now" on every response let a tab's own
    /// already-issued id go stale the moment some unrelated LATER tab
    /// happened to share its leading characters. Assigning it once, first
    /// come first served, means an id already handed to a caller can never
    /// be invalidated by anything that registers afterward.
    pub short_id: String,
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
            short_id: value.get("short_id")?.as_str()?.to_owned(),
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
            "short_id": self.short_id,
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

/// T118: assigns `record` its permanent short id here, under the same lock
/// this function already takes to read-modify-write the registry, so two
/// tabs registering at once can never both claim the same one. A tab that
/// already has a row (a genuine re-registration -- the same tab_uuid) keeps
/// exactly the short id it was given the first time; only a tab_uuid never
/// seen before gets a freshly computed one, via `shortest_available_prefix`
/// against every OTHER already-registered tab's own assigned id. Read the
/// assigned value back afterward with `short_id_for`.
pub fn register(record: &SessionRecord) -> io::Result<()> {
    register_at(&registry_path(), record)
}

fn register_at(path: &Path, record: &SessionRecord) -> io::Result<()> {
    let _lock = acquire_registry_lock(path)?;
    let mut records = read_records(path)?;
    let short_id = records
        .iter()
        .find(|existing| existing.tab_uuid == record.tab_uuid)
        .map(|existing| existing.short_id.clone())
        .unwrap_or_else(|| {
            let taken: Vec<&str> = records
                .iter()
                .filter(|existing| existing.tab_uuid != record.tab_uuid)
                .map(|existing| existing.short_id.as_str())
                .collect();
            shortest_available_prefix(&record.tab_uuid, &taken)
        });
    let mut record = record.clone();
    record.short_id = short_id;
    records.retain(|candidate| {
        candidate.token_id != record.token_id
            && (candidate.endpoint != record.endpoint
                || candidate.server_generation == record.server_generation)
    });
    records.push(record);
    write_records(path, &records)
}

/// The short id `register` assigned this tab_uuid when it first registered.
/// A plain read-after-write of what `register` just persisted -- called
/// right after it, never used to COMPUTE an assignment itself (that only
/// ever happens inside `register`'s own lock), so there is nothing here for
/// two callers to race on.
pub fn short_id_for(tab_uuid: &str) -> Option<String> {
    short_id_for_at(&registry_path(), tab_uuid)
}

fn short_id_for_at(path: &Path, tab_uuid: &str) -> Option<String> {
    read_records(path)
        .ok()?
        .into_iter()
        .find(|record| record.tab_uuid == tab_uuid)
        .map(|record| record.short_id)
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

/// The record for one tab, found by the `tab_uuid` every `open` answer
/// reports (T96).
///
/// This is what makes a tab id sufficient addressing on its own: a caller
/// holding an id needs no path, because the registry already knows which
/// endpoint serves that tab and which session token authorizes it. Every tab
/// registers, not only the first, so a tab added to a running workspace is
/// findable this way too.
///
/// The id is not a weaker credential than the token it stands for:
/// `tab_uuid_for` is a blake3 of the session token and the server generation,
/// so guessing one is guessing the other. It is the *stable* half of the pair —
/// short, safe to quote back in a response, and the thing an agent can carry
/// in its own notes.
///
/// T118: `tab_uuid` here may be a PREFIX, matched two ways. First, an EXACT
/// match against some tab's own officially assigned `short_id` (see
/// `register`) always wins outright, however many OTHER tabs' real hashes
/// also happen to start with the same characters -- those tabs were forced
/// to claim something longer specifically because this one registered
/// first, so they are never real candidates for this exact string. Failing
/// that, it falls back to raw prefix matching against every tab's full
/// hash (grouped by distinct uuid, since several records can share one
/// across re-registrations) -- what a full 64-char id, or a caller-supplied
/// prefix longer than anything ever assigned, still needs; two or more
/// DISTINCT uuids matching there is refused as `tab_ambiguous`, naming each
/// by its own assigned id, rather than silently picking one. A full id
/// still resolves exactly as before either way, since it can only ever
/// equal, never merely prefix, one real record.
pub fn resolve_tab(tab_uuid: &str) -> Result<SessionRecord, String> {
    resolve_tab_at(&registry_path(), tab_uuid)
}

fn resolve_tab_at(path: &Path, tab_uuid: &str) -> Result<SessionRecord, String> {
    if tab_uuid.is_empty() {
        return Err("tab_unknown: tab_id must not be empty".to_owned());
    }
    let records =
        read_records(path).map_err(|error| format!("cannot read session registry: {error}"))?;

    if let Some(record) = records
        .iter()
        .filter(|record| record.short_id == tab_uuid)
        .max_by_key(|record| record.start_time_ns)
    {
        if !endpoint_is_reachable(&record.endpoint) {
            return Err(format!(
                "tab_stale: tab {tab_uuid} belonged to a server that is no longer reachable; `open` the file again for a current id (the journal replays)"
            ));
        }
        return Ok(record.clone());
    }

    let mut by_uuid: std::collections::BTreeMap<String, Vec<SessionRecord>> =
        std::collections::BTreeMap::new();
    for record in records
        .into_iter()
        .filter(|record| record.tab_uuid.starts_with(tab_uuid))
    {
        by_uuid
            .entry(record.tab_uuid.clone())
            .or_default()
            .push(record);
    }
    if by_uuid.is_empty() {
        return Err(format!(
            "tab_unknown: no session record has tab_id {tab_uuid}; the tab may belong to a server that has since exited - `open` the file again for a current id (the journal replays)"
        ));
    }
    if by_uuid.len() > 1 {
        let candidates: Vec<String> = by_uuid
            .values()
            .map(|rows| rows[0].short_id.clone())
            .collect();
        return Err(format!(
            "tab_ambiguous: {tab_uuid} names {} open tabs; address one by its own id: {}",
            by_uuid.len(),
            candidates.join(", ")
        ));
    }
    let mut candidates = by_uuid.into_values().next().unwrap();
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.start_time_ns));
    let record = candidates.into_iter().next().unwrap();
    if !endpoint_is_reachable(&record.endpoint) {
        return Err(format!(
            "tab_stale: tab {tab_uuid} belonged to a server that is no longer reachable; `open` the file again for a current id (the journal replays)"
        ));
    }
    Ok(record)
}

/// T118: the shortest prefix of `tab_uuid` not already claimed as some OTHER
/// tab's own assigned id, git-style -- the first tab ever registered claims
/// just its first character; a second tab whose real hash also starts with
/// that character is simply denied it (already taken) and searches further,
/// landing on whatever longer prefix of ITS OWN hash isn't. Only ever called
/// from inside `register`'s own lock, against `taken`: every OTHER currently
/// registered tab's own already-assigned id (not raw hash prefixes -- an
/// existing SHORT claim like "c" is never itself extended just because a
/// later tab's hash also starts with "c").
fn shortest_available_prefix(tab_uuid: &str, taken: &[&str]) -> String {
    for len in 1..=tab_uuid.len() {
        let candidate = &tab_uuid[..len];
        if !taken.contains(&candidate) {
            return candidate.to_owned();
        }
    }
    tab_uuid.to_owned()
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
        // `register` overwrites this with the real first-come assignment
        // under its own lock; the full id is a safe placeholder until then.
        short_id: tab_uuid.clone(),
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

    /// T96: the tab uuid an `open` answer reports is enough to find the tab
    /// again — the same record, by the id rather than by the token.
    #[test]
    fn a_tab_uuid_identifies_the_record_that_reported_it() {
        let record = new_record(
            "unix:/tmp/editor.sock",
            "generation",
            "secret",
            None,
            Some("agent".into()),
        );
        assert_eq!(record.tab_uuid, tab_uuid_for("secret", "generation"));
        // Distinct tabs get distinct ids even under one identity, which is
        // what makes the id addressing rather than a hint.
        let sibling = new_record(
            "unix:/tmp/editor.sock",
            "generation",
            "other-secret",
            None,
            Some("agent".into()),
        );
        assert_ne!(record.tab_uuid, sibling.tab_uuid);
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

    /// A record with a hand-picked `tab_uuid` and `short_id`, bypassing the
    /// real hash/assignment so T118's tests can control exactly which ids
    /// collide and what each tab's own official short form is.
    fn fake_record(
        tab_uuid: &str,
        short_id: &str,
        endpoint: &str,
        start_time_ns: u64,
    ) -> SessionRecord {
        SessionRecord {
            token_id: format!("token-{tab_uuid}-{start_time_ns}"),
            tab_uuid: tab_uuid.to_owned(),
            short_id: short_id.to_owned(),
            server_generation: "generation".to_owned(),
            endpoint: endpoint.to_owned(),
            pid: std::process::id(),
            start_time_ns,
            agent_id: None,
            auth_token: None,
            session_token: format!("secret-{tab_uuid}"),
        }
    }

    #[test]
    fn shortest_available_prefix_grows_only_past_what_is_taken() {
        // Nothing taken yet: the ticket's own literal example, one character.
        assert_eq!(shortest_available_prefix("c3f100", &[]), "c");
        // "c" already taken by an earlier tab -- extend past it, even though
        // nothing has claimed "c3" yet.
        assert_eq!(shortest_available_prefix("c3f100", &["c"]), "c3");
        // "c" and "c3" both already taken -- extends further still.
        assert_eq!(shortest_available_prefix("c3f100", &["c", "c3"]), "c3f");
        // A different leading character is free regardless of what else is
        // taken.
        assert_eq!(shortest_available_prefix("d50000", &["c", "c3"]), "d");
    }

    #[test]
    fn register_gives_the_first_tab_the_short_id_and_extends_the_second() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("sessions.json");
        let first = fake_record("c3f100", "", "unix:/tmp/a.sock", 1);
        register_at(&path, &first).unwrap();
        assert_eq!(short_id_for_at(&path, "c3f100").unwrap(), "c");

        // "c" is already taken by the first tab, so the second -- whose real
        // hash also starts with "c" -- must extend past it, even though
        // nothing has claimed "c3" yet.
        let second = fake_record("c3a900", "", "unix:/tmp/b.sock", 2);
        register_at(&path, &second).unwrap();
        assert_eq!(short_id_for_at(&path, "c3a900").unwrap(), "c3");

        // The first tab's own id must not have moved now that the second
        // exists -- the whole point of assigning it once, first come first
        // served, rather than recomputing "the shortest unambiguous prefix"
        // fresh on every call.
        assert_eq!(short_id_for_at(&path, "c3f100").unwrap(), "c");
    }

    #[test]
    fn register_reuses_the_same_short_id_on_a_genuine_re_registration() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("sessions.json");
        let record = fake_record("c3f100", "", "unix:/tmp/a.sock", 1);
        register_at(&path, &record).unwrap();
        let first_assignment = short_id_for_at(&path, "c3f100").unwrap();

        // The SAME tab_uuid registering again (the same token_id, e.g. after
        // a crash-replay) keeps exactly the id it was given the first time,
        // not a freshly (and possibly differently) computed one.
        let mut reregistered = fake_record("c3f100", "", "unix:/tmp/a-new.sock", 2);
        reregistered.token_id = record.token_id.clone();
        register_at(&path, &reregistered).unwrap();
        assert_eq!(short_id_for_at(&path, "c3f100").unwrap(), first_assignment);
    }

    #[test]
    fn resolve_tab_at_answers_tab_unknown_for_an_unregistered_prefix() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("sessions.json");
        write_records(&path, &[fake_record("c3f100", "c", "unix:/tmp/a.sock", 1)]).unwrap();
        let error = resolve_tab_at(&path, "zzz").unwrap_err();
        assert!(error.starts_with("tab_unknown"), "{error}");
    }

    /// T118's own point: a later tab's real hash colliding with an earlier
    /// tab's already-ASSIGNED id must never make the earlier tab's own id
    /// ambiguous. The earlier tab's exact assigned form always wins outright.
    #[test]
    fn resolve_tab_at_prefers_the_first_come_owner_over_a_raw_hash_collision() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("sessions.json");
        write_records(
            &path,
            &[
                // Registered first: claimed "c" outright.
                fake_record("c3f100", "c", "unix:/tmp/a.sock", 1),
                // Registered second: its real hash ALSO starts with "c", but
                // "c" was already taken, so it was assigned "c3" instead.
                fake_record("c3a900", "c3", "unix:/tmp/b.sock", 2),
            ],
        )
        .unwrap();
        // "c" exactly names the first tab's own assignment -- resolves
        // cleanly, never ambiguous, despite the second tab's real hash also
        // starting with "c".
        let error = resolve_tab_at(&path, "c").unwrap_err();
        assert!(error.starts_with("tab_stale"), "{error}");
        // "c3" exactly names the second tab's own assignment -- likewise.
        let error = resolve_tab_at(&path, "c3").unwrap_err();
        assert!(error.starts_with("tab_stale"), "{error}");
    }

    /// A query that matches neither tab's own assigned id, yet still
    /// raw-prefixes both of their real hashes, is genuinely under-specified
    /// -- correctly refused, naming each candidate's own real assignment.
    #[test]
    fn resolve_tab_at_refuses_genuine_ambiguity_when_no_assigned_id_matches() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("sessions.json");
        write_records(
            &path,
            &[
                fake_record("abcdef", "a", "unix:/tmp/a.sock", 1),
                fake_record("abczzz", "ab", "unix:/tmp/b.sock", 2),
            ],
        )
        .unwrap();
        // "abc" is longer than either tab's own assigned id, but still a raw
        // prefix of BOTH real hashes -- genuinely ambiguous.
        let error = resolve_tab_at(&path, "abc").unwrap_err();
        assert!(error.starts_with("tab_ambiguous"), "{error}");
        assert!(error.contains('a'), "{error}");
        assert!(error.contains("ab"), "{error}");
    }

    #[test]
    fn resolve_tab_at_narrows_to_one_uuid_despite_several_registration_rows() {
        // Multiple rows for the SAME tab_uuid (re-registrations) must not
        // be mistaken for several distinct tabs sharing a prefix. Queried by
        // something that does not exactly match the shared assigned id, so
        // this exercises the raw-prefix fallback's own grouping specifically.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("sessions.json");
        write_records(
            &path,
            &[
                fake_record("c3f100", "c3f", "unix:/tmp/old.sock", 1),
                fake_record("c3f100", "c3f", "unix:/tmp/new.sock", 2),
            ],
        )
        .unwrap();
        // Narrowed to exactly one uuid group, so this fails at the
        // reachability check (tab_stale), never as tab_ambiguous/tab_unknown.
        let error = resolve_tab_at(&path, "c3f10").unwrap_err();
        assert!(error.starts_with("tab_stale"), "{error}");
    }

    #[test]
    fn resolve_tab_at_resolves_a_full_id_even_when_a_shorter_prefix_would_collide() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("sessions.json");
        write_records(
            &path,
            &[
                fake_record("c3f100", "c", "unix:/tmp/a.sock", 1),
                fake_record("c3a900", "c3", "unix:/tmp/b.sock", 2),
            ],
        )
        .unwrap();
        // The full id is neither tab's own assigned form, but it uniquely
        // raw-prefixes only the first tab's real hash.
        let error = resolve_tab_at(&path, "c3f100").unwrap_err();
        assert!(error.starts_with("tab_stale"), "{error}");
    }

    #[test]
    fn resolve_tab_at_refuses_an_empty_id_by_name() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("sessions.json");
        write_records(&path, &[fake_record("c3f100", "c", "unix:/tmp/a.sock", 1)]).unwrap();
        let error = resolve_tab_at(&path, "").unwrap_err();
        assert!(error.starts_with("tab_unknown"), "{error}");
    }
}
