// MODE: DEV
// PACKAGE: PROD
//! Install `source/<skill>` into `target/<skill>`. Every file is written to
//! a sibling temp file and renamed into place -- what B292 fixed in
//! install.sh (`cp` onto a running binary aborted the update); every write
//! here goes through it unconditionally, so there is no separate "existing
//! file" branch to get wrong.
//!
//! A re-install does not silently clobber a user's edits: before overwriting
//! an existing file that differs from the source, this checks whether the
//! file is unchanged since the LAST install (digest.rs) and backs it up
//! first (backup.rs) if not -- the same problem install.sh's
//! record_digests/unmodified_since_install/backup_file solve
//! (installer/src/60-install.sh), ported with a different digest (blake3,
//! not cksum) since this manifest is this installer's own.

use crate::backup;
use crate::digest;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn install_skill(source_root: &Path, skill: &str, target_root: &Path) -> io::Result<()> {
    let source_dir = source_root.join(skill);
    if !source_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no such skill directory: {}", source_dir.display()),
        ));
    }
    let dest_dir = target_root.join(skill);
    fs::create_dir_all(&dest_dir)?;

    let relative_paths = collect_relative_files(&source_dir, &PathBuf::new())?;
    for relative in &relative_paths {
        let source_file = source_dir.join(relative);
        let dest_file = dest_dir.join(relative);
        if dest_file.is_file()
            && !files_equal(&source_file, &dest_file)?
            && !digest::unmodified_since_install(&dest_dir, relative, &dest_file)
        {
            backup::backup_file(&dest_file)?;
        }
        if let Some(parent) = dest_file.parent() {
            fs::create_dir_all(parent)?;
        }
        copy_file_atomic(&source_file, &dest_file)?;
        #[cfg(unix)]
        preserve_executable_bit(&source_file, &dest_file)?;
    }
    digest::record_digests(&dest_dir, &relative_paths)?;
    Ok(())
}

/// Relative paths (forward-slash joined, regardless of host) of every FILE
/// under `dir`, recursively. Symlinks are neither a file nor a directory
/// here: this is an early slice and install.sh's own tree has none under a
/// skill directory, so refusing silently on one would be a worse surprise
/// than not handling it at all yet.
fn collect_relative_files(dir: &Path, prefix: &Path) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let file_type = entry.file_type()?;
        let relative = prefix.join(entry.file_name());
        if file_type.is_dir() {
            out.extend(collect_relative_files(&entry.path(), &relative)?);
        } else if file_type.is_file() {
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(out)
}

fn files_equal(a: &Path, b: &Path) -> io::Result<bool> {
    Ok(fs::read(a)? == fs::read(b)?)
}

fn copy_file_atomic(source: &Path, dest: &Path) -> io::Result<()> {
    let temp = sibling_temp_path(dest);
    fs::copy(source, &temp)?;
    fs::rename(&temp, dest)
}

fn sibling_temp_path(dest: &Path) -> PathBuf {
    let file_name = dest.file_name().unwrap_or_default().to_string_lossy();
    dest.with_file_name(format!(".{file_name}.installer-tmp.{}", std::process::id()))
}

#[cfg(unix)]
fn preserve_executable_bit(source: &Path, dest: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(source)?.permissions().mode();
    if mode & 0o111 != 0 {
        let mut perms = fs::metadata(dest)?.permissions();
        perms.set_mode(mode);
        fs::set_permissions(dest, perms)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn copies_a_skill_directory_tree() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        write(
            &source_root.path().join("todo/scripts/run.sh"),
            "#!/bin/sh\n",
        );

        let target_root = tempfile::tempdir().unwrap();
        install_skill(source_root.path(), "todo", target_root.path()).unwrap();

        assert_eq!(
            fs::read_to_string(target_root.path().join("todo/SKILL.md")).unwrap(),
            "# todo\n"
        );
        assert_eq!(
            fs::read_to_string(target_root.path().join("todo/scripts/run.sh")).unwrap(),
            "#!/bin/sh\n"
        );
    }

    #[test]
    fn missing_skill_directory_is_refused_not_silently_skipped() {
        let source_root = tempfile::tempdir().unwrap();
        let target_root = tempfile::tempdir().unwrap();
        let err = install_skill(source_root.path(), "nope", target_root.path()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn no_temp_file_survives_a_successful_copy() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        let target_root = tempfile::tempdir().unwrap();

        install_skill(source_root.path(), "todo", target_root.path()).unwrap();

        let leftovers: Vec<_> = fs::read_dir(target_root.path().join("todo"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("installer-tmp"))
            .collect();
        assert!(leftovers.is_empty(), "temp file left behind: {leftovers:?}");
    }

    #[cfg(unix)]
    #[test]
    fn executable_bit_survives_the_copy() {
        use std::os::unix::fs::PermissionsExt;
        let source_root = tempfile::tempdir().unwrap();
        let script = source_root.path().join("todo/scripts/run.sh");
        write(&script, "#!/bin/sh\n");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

        let target_root = tempfile::tempdir().unwrap();
        install_skill(source_root.path(), "todo", target_root.path()).unwrap();

        let mode = fs::metadata(target_root.path().join("todo/scripts/run.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111);
    }

    #[test]
    fn a_reinstall_over_an_untouched_file_writes_no_backup() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "v1\n");
        let target_root = tempfile::tempdir().unwrap();
        install_skill(source_root.path(), "todo", target_root.path()).unwrap();

        write(&source_root.path().join("todo/SKILL.md"), "v2\n");
        install_skill(source_root.path(), "todo", target_root.path()).unwrap();

        assert_eq!(
            fs::read_to_string(target_root.path().join("todo/SKILL.md")).unwrap(),
            "v2\n"
        );
        let backups: Vec<_> = fs::read_dir(target_root.path().join("todo"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".back"))
            .collect();
        assert!(
            backups.is_empty(),
            "unexpected backup on an untouched upgrade: {backups:?}"
        );
    }

    #[test]
    fn a_reinstall_over_a_user_edited_file_backs_it_up_first() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "v1\n");
        let target_root = tempfile::tempdir().unwrap();
        install_skill(source_root.path(), "todo", target_root.path()).unwrap();

        // The user edits the installed copy directly.
        fs::write(
            target_root.path().join("todo/SKILL.md"),
            "user's own notes\n",
        )
        .unwrap();

        write(&source_root.path().join("todo/SKILL.md"), "v2\n");
        install_skill(source_root.path(), "todo", target_root.path()).unwrap();

        assert_eq!(
            fs::read_to_string(target_root.path().join("todo/SKILL.md")).unwrap(),
            "v2\n"
        );
        let backups: Vec<_> = fs::read_dir(target_root.path().join("todo"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".back"))
            .collect();
        assert_eq!(backups.len(), 1, "expected exactly one backup: {backups:?}");
        let backed_up = fs::read_to_string(backups[0].path()).unwrap();
        assert_eq!(backed_up, "user's own notes\n");
    }
}
