// MODE: DEV
// PACKAGE: PROD
//! Persisted custom install roots a user typed once, offered again on a
//! later interactive run -- ported from install.sh's
//! `save_custom_location`/`load_custom_locations`
//! (`${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/custom-locations`,
//! installer/src/05-config.sh/06-*.sh). One absolute path per line; a line
//! starting with `#` or naming a directory that no longer exists is
//! dropped silently on load rather than offered as a dead choice.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn file_path(home: &Path) -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    base.join("tsch-ai-skills").join("custom-locations")
}

/// Every saved location that still exists as a directory, deduplicated in
/// file order -- `contains` in install.sh's own `load_custom_locations`.
pub fn load(home: &Path) -> Vec<PathBuf> {
    let Ok(content) = fs::read_to_string(file_path(home)) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let path = PathBuf::from(line);
        if !path.is_dir() {
            continue;
        }
        if !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

/// Appends `path` if it is not already recorded -- `grep -Fqx` before the
/// append in install.sh's own `save_custom_location`.
pub fn save(home: &Path, path: &Path) -> io::Result<()> {
    let file = file_path(home);
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = fs::read_to_string(&file).unwrap_or_default();
    let path_str = path.to_string_lossy();
    if existing.lines().any(|line| line == path_str) {
        return Ok(());
    }
    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&path_str);
    content.push('\n');
    fs::write(&file, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_with_no_file_is_empty() {
        let home = tempfile::tempdir().unwrap();
        assert!(load(home.path()).is_empty());
    }

    #[test]
    fn save_then_load_round_trips() {
        let home = tempfile::tempdir().unwrap();
        let custom = home.path().join("my-root");
        fs::create_dir_all(&custom).unwrap();
        save(home.path(), &custom).unwrap();
        assert_eq!(load(home.path()), vec![custom]);
    }

    #[test]
    fn saving_the_same_path_twice_does_not_duplicate_it() {
        let home = tempfile::tempdir().unwrap();
        let custom = home.path().join("my-root");
        fs::create_dir_all(&custom).unwrap();
        save(home.path(), &custom).unwrap();
        save(home.path(), &custom).unwrap();
        assert_eq!(load(home.path()), vec![custom]);
    }

    #[test]
    fn a_saved_location_that_no_longer_exists_is_dropped_on_load() {
        let home = tempfile::tempdir().unwrap();
        let custom = home.path().join("gone");
        fs::create_dir_all(&custom).unwrap();
        save(home.path(), &custom).unwrap();
        fs::remove_dir(&custom).unwrap();
        assert!(load(home.path()).is_empty());
    }

    #[test]
    fn a_commented_line_is_ignored() {
        let home = tempfile::tempdir().unwrap();
        let file = file_path(home.path());
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "# a comment\n").unwrap();
        assert!(load(home.path()).is_empty());
    }
}
