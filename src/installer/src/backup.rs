// MODE: DEV
// PACKAGE: PROD
//! A backup only earns its clutter where nothing else can recover the file.
//! Inside a git work tree the user already has history, so this says what
//! happened and lets git be the recovery path; outside one, the `.back` file
//! IS the only path. Mirrors install.sh's backup_file/recoverable_from_git
//! (installer/src/60-install.sh).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn recoverable_from_git(directory: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A dotfile beside the original: visible to `ls -a`, skipped by a plain
/// `grep -r` of the installed tree. Returns the backup path written, or
/// `None` when the directory is a git work tree and no file was written.
pub fn backup_file(file: &Path) -> io::Result<Option<PathBuf>> {
    let directory = file.parent().unwrap_or_else(|| Path::new("."));
    if recoverable_from_git(directory) {
        eprintln!("  Replaced (recoverable with git): {}", file.display());
        return Ok(None);
    }
    let base = file
        .file_name()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "backup target has no file name",
            )
        })?
        .to_string_lossy()
        .into_owned();
    let stamp = timestamp();
    let mut backup = directory.join(format!(".{base}.{stamp}.back"));
    let mut suffix = 1;
    while backup.exists() || backup.symlink_metadata().is_ok() {
        backup = directory.join(format!(".{base}.{stamp}.{suffix}.back"));
        suffix += 1;
    }
    fs::copy(file, &backup)?;
    eprintln!("  Backup: {}", backup.display());
    Ok(Some(backup))
}

fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // A minimal UTC YYYYMMDDTHHMMSSZ render with no chrono dependency: this
    // only has to be a unique, sortable label, not a calendar-correct date.
    format!("{secs:020}Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn outside_a_git_tree_a_dotfile_backup_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("SKILL.md");
        fs::write(&file, "original").unwrap();

        let backup = backup_file(&file).unwrap().expect("a backup path");

        assert!(backup
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with(".SKILL.md."));
        assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
    }

    #[test]
    fn inside_a_git_tree_no_backup_file_is_written() {
        let dir = tempfile::tempdir().unwrap();
        Command::new("git")
            .arg("init")
            .arg("-q")
            .arg(dir.path())
            .status()
            .unwrap();
        let file = dir.path().join("SKILL.md");
        fs::write(&file, "original").unwrap();

        let backup = backup_file(&file).unwrap();

        assert!(backup.is_none());
        let stray: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".back"))
            .collect();
        assert!(
            stray.is_empty(),
            "a .back file was written inside a git tree: {stray:?}"
        );
    }

    #[test]
    fn a_second_backup_the_same_second_does_not_clobber_the_first() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("SKILL.md");
        fs::write(&file, "v1").unwrap();
        let first = backup_file(&file).unwrap().unwrap();
        fs::write(&file, "v2").unwrap();
        let second = backup_file(&file).unwrap().unwrap();

        assert_ne!(first, second);
        assert_eq!(fs::read_to_string(&first).unwrap(), "v1");
        assert_eq!(fs::read_to_string(&second).unwrap(), "v2");
    }
}
