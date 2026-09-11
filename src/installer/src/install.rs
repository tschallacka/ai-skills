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
//! `package_dev` gates whether a raw dev checkout's own maintainer-only
//! content ships along with a skill -- install.sh's `--package prod|dev`/
//! `PACKAGE_SELECTION` (default `prod`). This installer never reads or runs
//! install.sh at all (no dependency on the original bash installer, present
//! or not): `collect_relative_files` decides independently, from three
//! sources, in order --
//!   1. a `bin/<target-triple>/` directory ships only the CURRENT host's own
//!      triple subdirectory (`installer_platform::current()`), not every
//!      platform's binaries -- a directory-layout convention, not anything
//!      read out of install.sh;
//!   2. `load_mode_manifest`'s per-skill `MODE-MANIFEST.tsv` override
//!      (`ModeOverride::Dev`/`Prod`/`Never`, exact path or `dir/` prefix),
//!      for files whose format has no comment syntax to carry a marker, or
//!      that must never ship at all (a compiler input, a maintainer-only
//!      inventory) -- something an unmarked file's should_ship default, and
//!      `package_dev` alone bypassing the marker scan, cannot express;
//!   3. `should_ship`'s own `# MODE: DEV` header scan for everything else,
//!      plus a whole-`tests/`-directory exclusion for `package_dev`.
//!
//! install.sh's separate `--dev-build`/`DEV_BUILD` ("prefer this host's
//! freshly-built binary over the shipped one") has no port here: this
//! installer's own copy step (below) only ever reads from
//! `source_root.join(skill)` -- one location, not two -- so there is no
//! second binary source to choose between.

use crate::backup;
use crate::digest;
use crate::integration;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Per-skill override file for `should_ship`'s own MODE-marker heuristic --
/// see the module doc comment. Never itself shipped in a `prod` build: it
/// carries its own `# MODE: DEV` header, same as any other maintainer-only
/// file.
const MODE_MANIFEST_FILENAME: &str = "MODE-MANIFEST.tsv";

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
    package_dev: bool,
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
    let relative_paths = relative_paths_for(source_root, skill, package_dev)?;

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
pub fn skill_relative_files(source_root: &Path, skill: &str, package_dev: bool) -> io::Result<Vec<String>> {
    relative_paths_for(source_root, skill, package_dev)
}

/// `collect_relative_files`'s answer for `skill`, with `MODE-MANIFEST.tsv`
/// (if the skill has one) loaded as an override first -- see the module doc
/// comment for the three-source order this follows.
fn relative_paths_for(source_root: &Path, skill: &str, package_dev: bool) -> io::Result<Vec<String>> {
    let skill_dir = source_root.join(skill);
    let overrides = load_mode_manifest(&skill_dir);
    collect_relative_files(&skill_dir, &PathBuf::new(), package_dev, &overrides)
}

/// `MODE-MANIFEST.tsv`'s three possible overrides for a file `should_ship`
/// would otherwise decide by header: `Dev` ships only in a `--package dev`
/// build (the usual `# MODE: DEV` meaning), `Prod` always ships (the usual
/// `# MODE: PROD`/no-marker default), and `Never` ships in neither tier --
/// a compiler input or maintainer-only inventory file `should_ship` has no
/// way to say "not even in dev" for, since an unmarked file always ships
/// somewhere and `package_dev` alone bypasses the marker scan entirely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModeOverride {
    Dev,
    Prod,
    Never,
}

/// `<skill>/MODE-MANIFEST.tsv`'s overrides, in two forms: an exact relative
/// path, or a directory prefix (a row whose path ends in `/`) applying to
/// everything under it -- `scripts/lib/` is one row rather than enumerating
/// every compiler-input source file by hand, and a name added under it
/// later needs no new row.
#[derive(Default)]
struct ModeManifest {
    exact: HashMap<String, ModeOverride>,
    prefixes: Vec<(String, ModeOverride)>,
}

impl ModeManifest {
    fn lookup(&self, relative: &str) -> Option<ModeOverride> {
        if let Some(mode) = self.exact.get(relative) {
            return Some(*mode);
        }
        self.prefixes
            .iter()
            .find(|(prefix, _)| relative.starts_with(prefix.as_str()))
            .map(|(_, mode)| *mode)
    }
}

/// Parses `<skill>/MODE-MANIFEST.tsv`'s `path\tmode` rows (`mode` is `DEV`,
/// `PROD`, or `NEVER`; anything else, and a line with no tab at all, is
/// skipped rather than rejected -- this is a maintainer-edited file, not a
/// validated format). Empty when the skill has no such file, which is
/// every skill but the ones that have needed one so far.
fn load_mode_manifest(skill_dir: &Path) -> ModeManifest {
    let mut manifest = ModeManifest::default();
    let Ok(content) = fs::read_to_string(skill_dir.join(MODE_MANIFEST_FILENAME)) else {
        return manifest;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((path, mode)) = line.split_once('\t') else {
            continue;
        };
        let path = path.trim();
        let mode = match mode.trim() {
            "DEV" => ModeOverride::Dev,
            "PROD" => ModeOverride::Prod,
            "NEVER" => ModeOverride::Never,
            _ => continue,
        };
        match path.strip_suffix('/') {
            Some(prefix) => manifest.prefixes.push((format!("{prefix}/"), mode)),
            None => {
                manifest.exact.insert(path.to_string(), mode);
            }
        }
    }
    manifest
}

/// Relative paths (forward-slash joined, regardless of host) of every FILE
/// under `dir`, recursively. Symlinks are neither a file nor a directory
/// here: this is an early slice and a skill directory has none, so refusing
/// silently on one would be a worse surprise than not handling it at all
/// yet.
///
/// A directory literally named `bin` directly under the skill root is
/// special-cased: only the current host's own `installer_platform::current()`
/// triple subdirectory is descended into (ship this platform's binary, not
/// every platform's), matching the `bin/<target-triple>/...` layout every
/// binary-shipping skill in this repository already uses. A host
/// `installer_platform` does not resolve ships no `bin/` content at all,
/// same as install.sh's own `skill_files()` refusing with no build for an
/// unknown platform (installer/src/50-manifest.sh).
fn collect_relative_files(
    dir: &Path,
    prefix: &Path,
    package_dev: bool,
    overrides: &ModeManifest,
) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let file_type = entry.file_type()?;
        let relative = prefix.join(entry.file_name());
        if file_type.is_dir() {
            if !package_dev && entry.file_name() == "tests" {
                continue;
            }
            if prefix.as_os_str().is_empty() && entry.file_name() == "bin" {
                if let Ok(target) = installer_platform::current() {
                    let triple = target.as_str();
                    let triple_dir = entry.path().join(triple);
                    if triple_dir.is_dir() {
                        out.extend(collect_relative_files(
                            &triple_dir,
                            &relative.join(triple),
                            package_dev,
                            overrides,
                        )?);
                    }
                }
                continue; // never ship another platform's bin/<triple>/ content
            }
            out.extend(collect_relative_files(&entry.path(), &relative, package_dev, overrides)?);
        } else if file_type.is_file() {
            let relative = relative.to_string_lossy().replace('\\', "/");
            let ship = match overrides.lookup(&relative) {
                Some(ModeOverride::Dev) => package_dev,
                Some(ModeOverride::Prod) => true,
                Some(ModeOverride::Never) => false,
                None => package_dev || should_ship(&entry.path()),
            };
            if ship {
                out.push(relative);
            }
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
/// every real skill; a file this ships wrongly here is what
/// `MODE-MANIFEST.tsv`'s override (above) is for.
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
    fn a_dev_marked_file_is_dropped_by_default_but_shipped_with_package_dev() {
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
    fn an_entire_tests_directory_is_dropped_by_default_but_shipped_with_package_dev() {
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

    #[test]
    fn only_the_current_hosts_own_bin_triple_ships() {
        let target = installer_platform::current().expect("test host must be a supported platform");
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        write(
            &source_root.path().join(format!("todo/bin/{target}/todo")),
            "binary",
        );
        write(&source_root.path().join("todo/bin/plan9-riscv64/todo"), "binary");
        let target_root = tempfile::tempdir().unwrap();

        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();
        assert!(target_root.path().join(format!("todo/bin/{target}/todo")).is_file());
        assert!(!target_root.path().join("todo/bin/plan9-riscv64").exists());
    }

    #[test]
    fn a_mode_manifest_override_excludes_an_unmarked_file_from_prod_but_ships_it_in_dev() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        write(&source_root.path().join("todo/.gitignore"), "plans/*\n");
        write(
            &source_root.path().join("todo/MODE-MANIFEST.tsv"),
            "# MODE: DEV\n.gitignore\tDEV\n",
        );
        let target_root = tempfile::tempdir().unwrap();

        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();
        assert!(!target_root.path().join("todo/.gitignore").exists());
        assert!(
            !target_root.path().join("todo/MODE-MANIFEST.tsv").exists(),
            "the manifest carries its own MODE: DEV marker, so should_ship excludes it from prod too"
        );

        let dev_target = tempfile::tempdir().unwrap();
        install_skill(source_root.path(), "todo", dev_target.path(), None, true).unwrap();
        assert!(dev_target.path().join("todo/.gitignore").is_file());
    }

    #[test]
    fn a_never_override_ships_in_neither_package_tier() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        write(&source_root.path().join("todo/migration-notes.tsv"), "a\tb\n");
        write(
            &source_root.path().join("todo/MODE-MANIFEST.tsv"),
            "# MODE: DEV\nmigration-notes.tsv\tNEVER\n",
        );
        let target_root = tempfile::tempdir().unwrap();
        install_skill(source_root.path(), "todo", target_root.path(), None, false).unwrap();
        assert!(!target_root.path().join("todo/migration-notes.tsv").exists());

        let dev_target = tempfile::tempdir().unwrap();
        install_skill(source_root.path(), "todo", dev_target.path(), None, true).unwrap();
        assert!(
            !dev_target.path().join("todo/migration-notes.tsv").exists(),
            "NEVER must survive package_dev's usual ship-everything default, not just should_ship's marker scan"
        );
    }

    #[test]
    fn a_never_override_prefix_excludes_every_file_under_that_directory() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo/SKILL.md"), "# todo\n");
        write(&source_root.path().join("todo/scripts/lib/a.sh"), "a\n");
        write(&source_root.path().join("todo/scripts/lib/nested/b.sh"), "b\n");
        write(&source_root.path().join("todo/scripts/run.sh"), "#!/bin/sh\n");
        write(
            &source_root.path().join("todo/MODE-MANIFEST.tsv"),
            "# MODE: DEV\nscripts/lib/\tNEVER\n",
        );
        let dev_target = tempfile::tempdir().unwrap();
        install_skill(source_root.path(), "todo", dev_target.path(), None, true).unwrap();
        assert!(!dev_target.path().join("todo/scripts/lib").exists());
        assert!(dev_target.path().join("todo/scripts/run.sh").is_file());
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
