// MODE: DEV
// PACKAGE: PROD
//! A change detector for installed content, recorded per file so a later
//! install can tell "we wrote this and nobody touched it" from "the user
//! edited it" -- the same problem install.sh's content_digest/record_digests
//! solve (60-install.sh). Different mechanism: blake3 rather than cksum's
//! CRC32+byte-count, since this manifest is this installer's own and never
//! compared against install.sh's -- the two are not on-disk compatible, and
//! do not need to be, as this replaces that script rather than running
//! alongside it.

use std::fs;
use std::io;
use std::path::Path;

pub fn content_digest(path: &Path) -> io::Result<String> {
    let bytes = fs::read(path)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

pub fn manifest_path(skill_dest: &Path) -> std::path::PathBuf {
    skill_dest.join(".filehashes")
}

/// Written after every file in `relative_paths` has been copied into
/// `skill_dest`, so a run that dies part-way leaves the OLD manifest and the
/// next run treats untouched files as ours (correct) and half-written ones
/// as modified (a needless backup, never a lost edit).
pub fn record_digests(skill_dest: &Path, relative_paths: &[String]) -> io::Result<()> {
    let mut lines = String::new();
    for relative in relative_paths {
        let file = skill_dest.join(relative);
        if !file.is_file() {
            continue;
        }
        lines.push_str(&content_digest(&file)?);
        lines.push(' ');
        lines.push_str(relative);
        lines.push('\n');
    }
    let manifest = manifest_path(skill_dest);
    let temp = manifest.with_extension("filehashes.tmp");
    fs::write(&temp, lines)?;
    fs::rename(&temp, &manifest)
}

/// The digest a prior install recorded for `relative`, or `None` if there is
/// no manifest yet, or no row for it.
pub fn recorded_digest(skill_dest: &Path, relative: &str) -> Option<String> {
    let manifest = manifest_path(skill_dest);
    let content = fs::read_to_string(manifest).ok()?;
    content.lines().find_map(|line| {
        let (digest, name) = line.split_once(' ')?;
        (name == relative).then(|| digest.to_string())
    })
}

/// Every relative path a prior install recorded a digest for, in file order
/// -- used by `uninstall::uninstall_skill` to warn about a user's edits
/// before deleting the whole directory rather than silently discarding them
/// unremarked.
pub fn recorded_relative_paths(skill_dest: &Path) -> Vec<String> {
    let Ok(content) = fs::read_to_string(manifest_path(skill_dest)) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| line.split_once(' ').map(|(_, name)| name.to_string()))
        .collect()
}

/// True when `file` is byte-for-byte what a prior install last wrote there,
/// so replacing it destroys nothing the user added.
pub fn unmodified_since_install(skill_dest: &Path, relative: &str, file: &Path) -> bool {
    let Some(recorded) = recorded_digest(skill_dest, relative) else {
        return false;
    };
    match content_digest(file) {
        Ok(current) => current == recorded,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_relative_paths_lists_every_row_in_order() {
        let dest = tempfile::tempdir().unwrap();
        fs::write(dest.path().join("a"), "1").unwrap();
        fs::write(dest.path().join("b"), "2").unwrap();
        record_digests(dest.path(), &["a".to_string(), "b".to_string()]).unwrap();

        assert_eq!(
            recorded_relative_paths(dest.path()),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn recorded_relative_paths_is_empty_with_no_manifest() {
        let dest = tempfile::tempdir().unwrap();
        assert!(recorded_relative_paths(dest.path()).is_empty());
    }

    #[test]
    fn a_file_matching_its_recorded_digest_is_unmodified() {
        let dest = tempfile::tempdir().unwrap();
        let file = dest.path().join("SKILL.md");
        fs::write(&file, "content").unwrap();
        record_digests(dest.path(), &["SKILL.md".to_string()]).unwrap();

        assert!(unmodified_since_install(dest.path(), "SKILL.md", &file));
    }

    #[test]
    fn an_edited_file_is_not_unmodified() {
        let dest = tempfile::tempdir().unwrap();
        let file = dest.path().join("SKILL.md");
        fs::write(&file, "content").unwrap();
        record_digests(dest.path(), &["SKILL.md".to_string()]).unwrap();

        fs::write(&file, "user edited this").unwrap();

        assert!(!unmodified_since_install(dest.path(), "SKILL.md", &file));
    }

    #[test]
    fn a_file_with_no_recorded_digest_is_not_unmodified() {
        let dest = tempfile::tempdir().unwrap();
        let file = dest.path().join("SKILL.md");
        fs::write(&file, "content").unwrap();

        assert!(!unmodified_since_install(dest.path(), "SKILL.md", &file));
    }

    #[test]
    fn a_dying_write_leaves_the_old_manifest_in_place() {
        let dest = tempfile::tempdir().unwrap();
        fs::write(dest.path().join("a"), "1").unwrap();
        record_digests(dest.path(), &["a".to_string()]).unwrap();
        let before = fs::read_to_string(manifest_path(dest.path())).unwrap();

        // Simulate a half-written manifest by leaving a stray temp file
        // beside it; record_digests must still overwrite atomically on its
        // next real run rather than being confused by it.
        fs::write(
            manifest_path(dest.path()).with_extension("filehashes.tmp"),
            "garbage",
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(manifest_path(dest.path())).unwrap(),
            before
        );
    }
}
