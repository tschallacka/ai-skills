// MODE: DEV
// PACKAGE: PROD
//! A fresh (non-merge) skill install: copy `source/<skill>` to
//! `target/<skill>`, one file at a time, each written to a sibling temp file
//! and renamed into place. Rename-into-place is what B292 fixed in
//! install.sh (`cp` onto a running binary aborted the update); every write
//! here goes through it unconditionally, so there is no separate "existing
//! file" branch to get wrong.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn install_skill_fresh(source_root: &Path, skill: &str, target_root: &Path) -> io::Result<()> {
    let source_dir = source_root.join(skill);
    if !source_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no such skill directory: {}", source_dir.display()),
        ));
    }
    let dest_dir = target_root.join(skill);
    fs::create_dir_all(&dest_dir)?;
    copy_tree(&source_dir, &dest_dir)?;
    Ok(())
}

fn copy_tree(source_dir: &Path, dest_dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dest_dir.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir_all(&dest_path)?;
            copy_tree(&entry.path(), &dest_path)?;
        } else if file_type.is_file() {
            copy_file_atomic(&entry.path(), &dest_path)?;
            #[cfg(unix)]
            preserve_executable_bit(&entry.path(), &dest_path)?;
        }
        // Symlinks are neither: this is an early slice and install.sh's own
        // tree has none under a skill directory, so refusing silently on one
        // here would be a worse surprise than not handling it at all yet.
    }
    Ok(())
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
        install_skill_fresh(source_root.path(), "todo", target_root.path()).unwrap();

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
        let err = install_skill_fresh(source_root.path(), "nope", target_root.path()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn no_temp_file_survives_a_successful_copy() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        let target_root = tempfile::tempdir().unwrap();

        install_skill_fresh(source_root.path(), "todo", target_root.path()).unwrap();

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
        install_skill_fresh(source_root.path(), "todo", target_root.path()).unwrap();

        let mode = fs::metadata(target_root.path().join("todo/scripts/run.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111);
    }
}
