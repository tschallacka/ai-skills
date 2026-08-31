// MODE: DEV
// PACKAGE: PROD
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

pub const SCAN_INTERVAL: Duration = Duration::from_millis(250);
pub const DEBOUNCE_WINDOW: Duration = Duration::from_millis(100);
pub const MAX_FILES: usize = 4096;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    files: BTreeMap<PathBuf, (u64, u64, u64)>,
}

impl Snapshot {
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeEvent {
    pub changed: Vec<PathBuf>,
    pub observed_at: Instant,
}

fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(1469598103934665603u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(1099511628211)
    })
}

fn collect(root: &Path, current: &mut BTreeMap<PathBuf, (u64, u64, u64)>) -> io::Result<()> {
    if current.len() >= MAX_FILES {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect(&path, current)?;
        } else if metadata.is_file() {
            let bytes = fs::read(&path)?;
            let modified = metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            current.insert(path, (modified, metadata.len(), fingerprint(&bytes)));
            if current.len() >= MAX_FILES {
                break;
            }
        }
    }
    Ok(())
}

pub fn snapshot(root: &Path) -> io::Result<Snapshot> {
    let mut files = BTreeMap::new();
    collect(root, &mut files)?;
    Ok(Snapshot { files })
}

pub fn scan_plan_dir(
    root: &Path,
    previous: &Snapshot,
) -> io::Result<(Snapshot, Option<ChangeEvent>)> {
    let next = snapshot(root)?;
    let changed: Vec<PathBuf> = next
        .files
        .keys()
        .chain(previous.files.keys())
        .filter(
            |path| match (next.files.get(*path), previous.files.get(*path)) {
                (Some(next), Some(previous)) => next.1 != previous.1 || next.2 != previous.2,
                _ => true,
            },
        )
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let event = (!changed.is_empty()).then(|| ChangeEvent {
        changed,
        observed_at: Instant::now(),
    });
    Ok((next, event))
}

pub fn coalesce_events(events: &[ChangeEvent], window: Duration) -> Vec<ChangeEvent> {
    let mut result: Vec<ChangeEvent> = Vec::new();
    for event in events {
        if let Some(last) = result.last_mut() {
            if event.observed_at.duration_since(last.observed_at) <= window {
                for path in &event.changed {
                    if !last.changed.contains(path) {
                        last.changed.push(path.clone());
                    }
                }
                last.observed_at = event.observed_at;
                continue;
            }
        }
        result.push(event.clone());
    }
    result
}

pub fn watch_plan_dir(root: PathBuf) -> io::Result<(Receiver<ChangeEvent>, Sender<()>)> {
    let initial = snapshot(&root)?;
    let (output, events) = mpsc::channel();
    let (stop, stopped) = mpsc::channel();
    thread::spawn(move || {
        let mut previous = initial;
        loop {
            if stopped.try_recv().is_ok() {
                break;
            }
            thread::sleep(SCAN_INTERVAL);
            match scan_plan_dir(&root, &previous) {
                Ok((next, Some(event))) => {
                    previous = next;
                    if output.send(event).is_err() {
                        break;
                    }
                }
                Ok((next, None)) => previous = next,
                Err(_) => break,
            }
        }
    });
    Ok((events, stop))
}
