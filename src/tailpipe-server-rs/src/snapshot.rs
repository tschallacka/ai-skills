// MODE: DEV
// PACKAGE: PROD
//! Flushing a stream's retained lines to a gzip-compressed file, the "saving
//! to file must be gzipped" requirement. `eviction::sweep_idle_streams` and
//! the Hub's own explicit Save dispatch both call this; neither re-implements
//! gzip writing.

use crate::store::StreamLog;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Writes `log`'s retained lines, one per line, to
/// `snapshot_dir/<stream_name>-<unix_timestamp>.gz` and returns that path.
pub fn write_gzip_snapshot(
    snapshot_dir: &Path,
    stream_name: &str,
    log: &StreamLog,
) -> io::Result<PathBuf> {
    std::fs::create_dir_all(snapshot_dir)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let path = snapshot_dir.join(format!("{stream_name}-{timestamp}.gz"));
    let file = std::fs::File::create(&path)?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    for line in log.lines() {
        writeln!(encoder, "{}", line.text)?;
    }
    encoder.finish()?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use std::io::Read;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tp-snapshot-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_written_snapshot_matches_the_naming_pattern_and_decompresses_to_the_original_lines() {
        let dir = scratch("roundtrip");
        let mut log = StreamLog::new();
        log.ingest("first".into());
        log.ingest("second".into());

        let path = write_gzip_snapshot(&dir, "mystream", &log).unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("mystream-") && name.ends_with(".gz"));

        let mut decoder = GzDecoder::new(std::fs::File::open(&path).unwrap());
        let mut text = String::new();
        decoder.read_to_string(&mut text).unwrap();
        assert_eq!(text, "first\nsecond\n");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
