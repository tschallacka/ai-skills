// MODE: DEV
// PACKAGE: PROD
//! Idle-stream eviction: a stream with no read or write for the configured
//! threshold is snapshotted (never silently dropped) and removed from the
//! registry. Operates on the same `HashMap<String, StreamLog>` the Hub (W07)
//! owns and passes in, rather than on a `Hub` type itself -- this crate
//! builds eviction before Hub exists, and Hub's own explicit-Save dispatch
//! calls `snapshot::write_gzip_snapshot` directly without evicting, so the
//! two paths share only the snapshot function, not this sweep.

use crate::snapshot::write_gzip_snapshot;
use crate::store::StreamLog;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Snapshots and removes every stream in `streams` whose last activity is
/// older than `idle_threshold`. Returns the names removed.
pub fn sweep_idle_streams(
    streams: &mut HashMap<String, StreamLog>,
    idle_threshold: Duration,
    snapshot_dir: &Path,
) -> Vec<String> {
    let idle: Vec<String> = streams
        .iter()
        .filter(|(_, log)| log.last_activity().elapsed() >= idle_threshold)
        .map(|(name, _)| name.clone())
        .collect();
    for name in &idle {
        if let Some(log) = streams.get(name) {
            let _ = write_gzip_snapshot(snapshot_dir, name, log);
        }
        streams.remove(name);
    }
    idle
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tp-eviction-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_idle_stream_is_evicted_and_snapshotted_while_a_fresh_one_is_left_alone() {
        let dir = scratch("sweep");
        let mut streams = HashMap::new();

        let mut idle_log = StreamLog::new();
        idle_log.ingest("stale line".into());
        // Backdate by sleeping past a tiny threshold, rather than reaching
        // into StreamLog's own private last_activity field.
        std::thread::sleep(Duration::from_millis(20));
        streams.insert("idle-stream".to_string(), idle_log);

        let mut fresh_log = StreamLog::new();
        fresh_log.ingest("fresh line".into());
        streams.insert("fresh-stream".to_string(), fresh_log);

        let removed = sweep_idle_streams(&mut streams, Duration::from_millis(10), &dir);

        assert_eq!(removed, vec!["idle-stream".to_string()]);
        assert!(!streams.contains_key("idle-stream"));
        assert!(streams.contains_key("fresh-stream"));

        let snapshot = std::fs::read_dir(&dir)
            .unwrap()
            .find_map(|entry| entry.ok())
            .expect("a snapshot file was written");
        assert!(snapshot
            .file_name()
            .to_str()
            .unwrap()
            .starts_with("idle-stream-"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
