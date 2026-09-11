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
//!
//! `dev_build` gates whether a raw dev checkout's own maintainer-only
//! content ships along with a skill -- install.sh's default is `prod`
//! (`--dev-build` opts out), driven by a hand-maintained `skill_files()`
//! manifest (installer/src/50-manifest.sh). This installer gets install.sh's
//! own, exact answer whenever it can: `skill_manifest::skill_files_via_install_sh`
//! extracts `skill_files()` (and its one helper) out of a real `install.sh`
//! sitting next to `source_root` and runs them with bash itself, so the
//! file list is install.sh's own, byte-for-byte, not a re-derivation of it.
//! `collect_relative_files`'s own `should_ship` (`# MODE: DEV` header /
//! `tests/` directory) is the fallback for when that is not possible -- no
//! `install.sh` next to `--source` (a `build-release.sh` tarball, which
//! never ships one; `bootstrap.sh` downloads the skill payload alone), or
//! no `bash` on PATH. That tarball is the common end-user path and is
//! already prod-only content, so the fallback is a no-op there either way;
//! the exact path only matters -- and only engages -- when `--source`
//! names a raw checkout directly.

use crate::backup;
use crate::digest;
use crate::integration;
use crate::skill_manifest;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Installs `skill` in whichever mode `integration::resolve_mode` picks for
/// it (an explicit `--integration` choice, else whatever mode is already on
/// disk at `target_root/skill`, else `skill`) -- ported from install.sh's
/// `install_skill` (installer/src/60-install.sh), which resolves the mode
/// once up front and threads it through both the copy filter
/// (`integration_file_allowed`) and the stale-binary cleanup
/// (`remove_stale_integration_binaries`). A skill with no `integration.tsv`
/// (almost all of them) has every file mode-free, so this is a no-op filter
/// for them -- same behavior as before integration modes existed.
pub fn install_skill(
    source_root: &Path,
    skill: &str,
    target_root: &Path,
    integration_choice: Option<&str>,
    dev_build: bool,
) -> io::Result<()> {
    let source_dir = source_root.join(skill);
    if !source_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no such skill directory: {}", source_dir.display()),
        ));
    }
    let dest_dir = target_root.join(skill);
    if dest_dir.is_symlink() {
        return Err(io::Error::other(format!(
            "existing symlink requires manual review: {}",
            dest_dir.display()
        )));
    }
    fs::create_dir_all(&dest_dir)?;

    let mode = integration::resolve_mode(source_root, skill, Some(&dest_dir), integration_choice);
    let relative_paths = relative_paths_for(source_root, skill, dev_build)?;

    // A symlinked destination file (or the digest manifest itself) is left
    // for manual review rather than silently replaced -- ported from
    // install.sh's own `[ -L "$destination_file" ]`/`[ -L "$destination/.
    // version" ]` refusal (installer/src/60-install.sh). Checked for every
    // file BEFORE any write happens, matching bash's own two-pass shape:
    // a collision found partway through must not leave a half-written skill.
    for relative in &relative_paths {
        let dest_file = dest_dir.join(relative);
        if dest_file.is_symlink() {
            return Err(io::Error::other(format!(
                "existing symlink requires manual review: {}",
                dest_file.display()
            )));
        }
    }
    if digest::manifest_path(&dest_dir).is_symlink() {
        return Err(io::Error::other(format!(
            "existing symlink requires manual review: {}",
            digest::manifest_path(&dest_dir).display()
        )));
    }

    let mut installed_paths = Vec::with_capacity(relative_paths.len());
    for relative in &relative_paths {
        if !integration::file_allowed(source_root, skill, relative, &mode) {
            let dest_file = dest_dir.join(relative);
            if dest_file.is_file() {
                fs::remove_file(&dest_file)?;
            }
            continue;
        }
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
        installed_paths.push(relative.clone());
    }
    digest::record_digests(&dest_dir, &installed_paths)?;
    Ok(())
}

/// Public wrapper around `relative_paths_for` for `cli_mode.rs`'s own
/// independent collision-checking install path (install.sh's
/// `cli_install_skill`), which needs the same "what would this skill ship"
/// answer without going through the backup/digest machinery this module's
/// own `install_skill` wraps it in.
pub fn skill_relative_files(source_root: &Path, skill: &str, dev_build: bool) -> io::Result<Vec<String>> {
    relative_paths_for(source_root, skill, dev_build)
}

/// install.sh's own `skill_files()` answer when it can be gotten (a real
/// `install.sh` sits next to `source_root`), else `collect_relative_files`'s
/// MODE-marker heuristic. A path the exact answer names but that does not
/// actually exist on disk is dropped with a note on stderr rather than
/// failing the whole install -- install.sh's own interactive install_skill
/// has no existence check either (only its CLI-mode handlers do, where a
/// missing source is a documented `die`), so a missing file here reads as
/// "this row does not apply on this host" the same way a platform-gated
/// binary row already does.
fn relative_paths_for(source_root: &Path, skill: &str, dev_build: bool) -> io::Result<Vec<String>> {
    if let Some(exact) = skill_manifest::skill_files_via_install_sh(source_root, skill, dev_build) {
        let source_dir = source_root.join(skill);
        return Ok(exact
            .into_iter()
            .filter(|relative| {
                let exists = source_dir.join(relative).is_file();
                if !exists {
                    eprintln!(
                        "{skill}: install.sh's own manifest names {relative}, which does not exist here; skipping"
                    );
                }
                exists
            })
            .collect());
    }
    collect_relative_files(&source_root.join(skill), &PathBuf::new(), dev_build)
}

/// Relative paths (forward-slash joined, regardless of host) of every FILE
/// under `dir`, recursively. Symlinks are neither a file nor a directory
/// here: this is an early slice and install.sh's own tree has none under a
/// skill directory, so refusing silently on one would be a worse surprise
/// than not handling it at all yet.
fn collect_relative_files(dir: &Path, prefix: &Path, dev_build: bool) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let file_type = entry.file_type()?;
        let relative = prefix.join(entry.file_name());
        if file_type.is_dir() {
            if !dev_build && entry.file_name() == "tests" {
                continue;
            }
            out.extend(collect_relative_files(&entry.path(), &relative, dev_build)?);
        } else if file_type.is_file() && (dev_build || should_ship(&entry.path())) {
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(out)
}

/// Ships unless the file's own header (first 25 lines, same window
/// build-release.sh's `declares_prod` reads) explicitly says `# MODE: DEV`
/// or `<!-- MODE: DEV -->`. Everything else ships: an explicit `# MODE:
/// PROD` marker, and -- unlike `declares_prod`, which defaults an unmarked
/// file to NOT prod -- a file with no marker at all, since most files this
/// installer would otherwise silently drop (JSON schemas, TSVs, prebuilt
/// binaries) have no comment syntax to carry one and were always meant to
/// ship. `declares_prod` can default the other way because build-release.sh
/// pairs it with a hand-maintained inclusion list (`skill_files()`) for
/// every real skill; this has no such list to fall back on.
fn should_ship(path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(path) else {
        return true; // not read as text (e.g. a binary) -- never DEV-marked
    };
    let header: String = content.lines().take(25).collect::<Vec<_>>().join("\n");
    !(header.contains("# MODE: DEV") || header.contains("<!-- MODE: DEV -->"))
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
        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();

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
        let err = install_skill(source_root.path(), "nope", target_root.path(), None, false).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn no_temp_file_survives_a_successful_copy() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        let target_root = tempfile::tempdir().unwrap();

        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();

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
        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();

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
        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();

        write(&source_root.path().join("todo/SKILL.md"), "v2\n");
        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();

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
        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();

        // The user edits the installed copy directly.
        fs::write(
            target_root.path().join("todo/SKILL.md"),
            "user's own notes\n",
        )
        .unwrap();

        write(&source_root.path().join("todo/SKILL.md"), "v2\n");
        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();

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

    fn write_integration_tsv(source_root: &Path, skill: &str) {
        write(
            &source_root.join(skill).join("integration.tsv"),
            "mode\tbinary\twhy\n\
             skill\tai-text-editor\tShort-lived client\n\
             mcp\tai-text-editor-mcp\tMCP bridge\n",
        );
    }

    #[test]
    fn a_skill_mode_install_ships_only_the_skill_binary() {
        let source_root = tempfile::tempdir().unwrap();
        write_integration_tsv(source_root.path(), "ai-text-editor");
        write(
            &source_root
                .path()
                .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor"),
            "skill binary",
        );
        write(
            &source_root
                .path()
                .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor-mcp"),
            "mcp binary",
        );
        let target_root = tempfile::tempdir().unwrap();

        install_skill(
            source_root.path(),
            "ai-text-editor",
            target_root.path(),
            Some("skill"),
            false,
        )
        .unwrap();

        assert!(target_root
            .path()
            .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor")
            .is_file());
        assert!(!target_root
            .path()
            .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor-mcp")
            .is_file());
    }

    #[test]
    fn switching_mode_removes_the_previous_modes_stale_binary() {
        let source_root = tempfile::tempdir().unwrap();
        write_integration_tsv(source_root.path(), "ai-text-editor");
        write(
            &source_root
                .path()
                .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor"),
            "skill binary",
        );
        write(
            &source_root
                .path()
                .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor-mcp"),
            "mcp binary",
        );
        let target_root = tempfile::tempdir().unwrap();

        install_skill(
            source_root.path(),
            "ai-text-editor",
            target_root.path(),
            Some("skill"),
            false,
        )
        .unwrap();
        assert!(target_root
            .path()
            .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor")
            .is_file());

        install_skill(
            source_root.path(),
            "ai-text-editor",
            target_root.path(),
            Some("mcp"),
            false,
        )
        .unwrap();

        assert!(!target_root
            .path()
            .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor")
            .is_file());
        assert!(target_root
            .path()
            .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor-mcp")
            .is_file());
    }

    #[test]
    fn an_unattended_reinstall_carries_the_installed_mode_forward() {
        let source_root = tempfile::tempdir().unwrap();
        write_integration_tsv(source_root.path(), "ai-text-editor");
        write(
            &source_root
                .path()
                .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor"),
            "skill binary",
        );
        write(
            &source_root
                .path()
                .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor-mcp"),
            "mcp binary",
        );
        let target_root = tempfile::tempdir().unwrap();

        install_skill(
            source_root.path(),
            "ai-text-editor",
            target_root.path(),
            Some("mcp"),
            false,
        )
        .unwrap();

        // No explicit choice this time -- the mcp install already on disk
        // must survive, not silently revert to the `skill` default (T109).
        install_skill(source_root.path(), "ai-text-editor", target_root.path(), None, false).unwrap();

        assert!(target_root
            .path()
            .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor-mcp")
            .is_file());
        assert!(!target_root
            .path()
            .join("ai-text-editor/bin/x86_64-unknown-linux-musl/ai-text-editor")
            .is_file());
    }

    #[test]
    fn a_dev_marked_file_is_dropped_by_default_but_shipped_with_dev_build() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        write(
            &source_root.path().join("todo/maintainer-notes.md"),
            "<!-- MODE: DEV -->\nnotes for the next maintainer\n",
        );
        let target_root = tempfile::tempdir().unwrap();

        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();
        assert!(target_root.path().join("todo/SKILL.md").is_file());
        assert!(!target_root.path().join("todo/maintainer-notes.md").is_file());

        let dev_target = tempfile::tempdir().unwrap();
        install_skill(source_root.path(), "todo", dev_target.path(), None, true).unwrap();
        assert!(dev_target.path().join("todo/maintainer-notes.md").is_file());
    }

    #[test]
    fn an_entire_tests_directory_is_dropped_by_default_but_shipped_with_dev_build() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        write(
            &source_root.path().join("todo/tests/test-todo.sh"),
            "#!/bin/sh\necho ok\n",
        );
        let target_root = tempfile::tempdir().unwrap();

        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();
        assert!(!target_root.path().join("todo/tests").exists());

        let dev_target = tempfile::tempdir().unwrap();
        install_skill(source_root.path(), "todo", dev_target.path(), None, true).unwrap();
        assert!(dev_target.path().join("todo/tests/test-todo.sh").is_file());
    }

    #[test]
    fn a_file_with_no_marker_at_all_still_ships() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        write(
            &source_root.path().join("todo/schema.json"),
            "{\"type\": \"object\"}",
        );
        let target_root = tempfile::tempdir().unwrap();

        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();
        assert!(target_root.path().join("todo/schema.json").is_file());
    }

    #[test]
    fn a_prod_marked_file_ships_by_default() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        write(
            &source_root.path().join("todo/scripts/run.sh"),
            "#!/bin/sh\n# MODE: PROD\necho ok\n",
        );
        let target_root = tempfile::tempdir().unwrap();

        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();
        assert!(target_root.path().join("todo/scripts/run.sh").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_skill_directory_is_refused_not_silently_converted() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        let target_root = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), target_root.path().join("todo")).unwrap();

        let err =
            install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap_err();
        assert!(err.to_string().contains("symlink"));
        assert!(target_root.path().join("todo").is_symlink());
        assert!(!elsewhere.path().join("SKILL.md").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_destination_file_is_refused_not_silently_converted() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        let target_root = tempfile::tempdir().unwrap();
        fs::create_dir_all(target_root.path().join("todo")).unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let bogus_target = elsewhere.path().join("not-really-skill-md");
        write(&bogus_target, "not the real file");
        std::os::unix::fs::symlink(&bogus_target, target_root.path().join("todo/SKILL.md")).unwrap();

        let err =
            install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap_err();
        assert!(err.to_string().contains("symlink"));
        assert!(target_root.path().join("todo/SKILL.md").is_symlink());
    }
}
