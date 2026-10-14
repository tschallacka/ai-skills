// MODE: DEV
// PACKAGE: PROD
//! Puts the shared bin directory (`shared_bin::shared_bin_dir`) at the
//! FRONT of `PATH`, so an installed binary runs as a bare command instead
//! of needing its full resolved path spelled out every time.
//!
//! "At the front" is deliberate, and so is WHERE the edit goes: on Unix this
//! writes one small POSIX `sh` snippet (`env_file_path`) that prepends the
//! directory, and sources it from a one-line guard inserted at the very TOP
//! of each shell's own startup file -- before anything already there. A
//! shell always lets a LATER `PATH` assignment win over an earlier one, so
//! putting our prepend first means anything the user's own dotfile does
//! afterward (their own prepend, oh-my-zsh, direnv, pyenv, ...) still comes
//! out ahead of ours in the final `PATH` -- exactly backwards from the usual
//! installer habit of appending `export PATH="$PATH:$DIR"` at the file's
//! end, which both suffixes the new directory (so anything already on
//! `PATH` is found first, defeating the point of adding it) and can never
//! be overridden by anything earlier in the same file.
//!
//! One env file, sourced from several rc files, rather than writing the
//! `case`/`export` logic into each one separately: `rustup`'s own
//! `~/.cargo/env` is the same shape, for the same reason -- one place to
//! read or change the actual logic, no risk of the copies drifting.

use crate::backup;
use crate::permissions;
use crate::shared_bin;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MARKER_OPEN: &str = "# >>> tsch-ai-skills path >>>";
const MARKER_CLOSE: &str = "# <<< tsch-ai-skills path <<<";

/// `${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/env` -- the one file
/// every rc guard below sources. Not itself on `PATH`; it is `.`-sourced.
pub fn env_file_path(home: &Path) -> PathBuf {
    shared_bin::xdg_config_home_or(home)
        .join("tsch-ai-skills")
        .join("env")
}

fn env_file_content(home: &Path) -> String {
    let bin = shared_bin::shared_bin_dir(home);
    let bin = bin.to_string_lossy();
    format!(
        "# tsch-ai-skills: puts the shared bin directory at the front of PATH.\n\
         # Written by the ai-skills installer -- safe to re-source any number of times.\n\
         case \":$PATH:\" in\n    \
             *\":{bin}:\"*) ;;\n    \
             *) export PATH=\"{bin}:$PATH\" ;;\n\
         esac\n"
    )
}

/// Writes (or rewrites) the env file with today's shared-bin path. Returns
/// whether the content actually changed -- a stale copy from an older
/// install (a different `$HOME`, a renamed shared-bin convention) is exactly
/// the case this must still catch, so this always recomputes and compares
/// rather than skipping because the file merely exists.
fn write_env_file(home: &Path) -> io::Result<bool> {
    let path = env_file_path(home);
    let content = env_file_content(home);
    if let Ok(existing) = fs::read_to_string(&path) {
        if existing == content {
            return Ok(false);
        }
        backup::backup_file(&path)?;
        permissions::write_preserving_mode(&path, &content)?;
        return Ok(true);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, content)?;
    Ok(true)
}

fn guard_block(env_file: &Path) -> String {
    let path = env_file.to_string_lossy();
    format!("{MARKER_OPEN}\n[ -f \"{path}\" ] && . \"{path}\"\n{MARKER_CLOSE}\n")
}

/// Inserts the sourcing guard at the top of `path` -- after a leading
/// shebang line if the file has one, otherwise as the very first line --
/// unless the marker is already there, in which case this touches nothing.
/// Creates the file (and its parent directory) if it does not exist yet.
fn ensure_guard_at_top(path: &Path, env_file: &Path) -> io::Result<bool> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing.contains(MARKER_OPEN) {
        return Ok(false);
    }
    let block = guard_block(env_file);
    let new_content = if let Some(rest) = existing.strip_prefix("#!") {
        let split_at = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
        let (shebang_rest, tail) = rest.split_at(split_at);
        format!("#!{shebang_rest}{block}{tail}")
    } else {
        format!("{block}{existing}")
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.is_file() {
        backup::backup_file(path)?;
        permissions::write_preserving_mode(path, &new_content)?;
    } else {
        fs::write(path, &new_content)?;
    }
    Ok(true)
}

/// Every rc file that gets the sourcing guard, for exactly the reason named
/// beside each: between them, an interactive OR login shell, bash OR zsh,
/// on Linux OR macOS, reads at least one. `~/.bash_profile` is the one
/// exception -- touched only if it already exists, never created, because
/// bash reads the FIRST of `.bash_profile` / `.bash_login` / `.profile` it
/// finds as a login shell and ignores the rest entirely: creating a
/// `.bash_profile` where none existed would silently stop that shell from
/// ever reading `.profile` again, for anything else that file does.
fn unix_rc_targets(home: &Path) -> Vec<PathBuf> {
    let mut targets = vec![
        home.join(".zshenv"),  // every zsh invocation: login, interactive, script
        home.join(".bashrc"),  // interactive non-login bash (the common terminal case)
        home.join(".profile"), // POSIX login fallback (sh, and bash with no .bash_profile)
    ];
    let bash_profile = home.join(".bash_profile");
    if bash_profile.is_file() {
        targets.push(bash_profile);
    }
    targets
}

pub enum PathEnvOutcome {
    /// Every rc target already had the guard and the env file's content was
    /// already current; nothing was written.
    AlreadyConfigured,
    /// At least one file was created or updated.
    Configured { touched: Vec<PathBuf> },
}

#[cfg(not(windows))]
pub fn ensure_shared_bin_on_path(home: &Path) -> io::Result<PathEnvOutcome> {
    let env_file = env_file_path(home);
    let mut touched = Vec::new();
    if write_env_file(home)? {
        touched.push(env_file.clone());
    }
    for target in unix_rc_targets(home) {
        if ensure_guard_at_top(&target, &env_file)? {
            touched.push(target);
        }
    }
    if touched.is_empty() {
        Ok(PathEnvOutcome::AlreadyConfigured)
    } else {
        Ok(PathEnvOutcome::Configured { touched })
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::process::Command;

    /// Pure string logic, platform-independent on purpose: given the
    /// current `HKCU\Environment\Path` value (empty if the name does not
    /// exist yet) and the directory to add, returns the new value to write,
    /// or `None` when `dir` is already in the list. A Windows path is
    /// case-insensitive, so the membership check folds case; the stored
    /// value itself is never case-folded.
    pub(super) fn prepend_to_path_list(existing: &str, dir: &str) -> Option<String> {
        let already_present = existing
            .split(';')
            .any(|entry| entry.trim().eq_ignore_ascii_case(dir));
        if already_present {
            return None;
        }
        Some(if existing.trim().is_empty() {
            dir.to_string()
        } else {
            format!("{dir};{existing}")
        })
    }

    fn query_current_path() -> io::Result<String> {
        let output = Command::new("reg")
            .args(["query", "HKCU\\Environment", "/v", "Path"])
            .output()?;
        if !output.status.success() {
            // Most likely: the value has never been set for this user.
            return Ok(String::new());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        // A line shaped "    Path    REG_SZ    C:\foo;C:\bar" (or
        // REG_EXPAND_SZ) -- the value is everything after the third
        // whitespace-separated column, verbatim (it may itself contain
        // spaces, so this joins the remainder rather than taking one token).
        for line in stdout.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("Path") {
                let rest = rest.trim_start();
                if let Some(value) = rest
                    .strip_prefix("REG_EXPAND_SZ")
                    .or_else(|| rest.strip_prefix("REG_SZ"))
                {
                    return Ok(value.trim_start().to_string());
                }
            }
        }
        Ok(String::new())
    }

    fn write_path(new_value: &str) -> io::Result<()> {
        let status = Command::new("reg")
            .args([
                "add",
                "HKCU\\Environment",
                "/v",
                "Path",
                "/t",
                "REG_EXPAND_SZ",
                "/d",
                new_value,
                "/f",
            ])
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "reg.exe add HKCU\\Environment /v Path failed",
            ))
        }
    }

    /// No `WM_SETTINGCHANGE` broadcast: that needs a WinAPI call this crate
    /// has no dependency for today, and the practical effect without it is
    /// only that an ALREADY-OPEN terminal does not see the change -- a new
    /// one does, since it reads the persisted value at startup like any
    /// other process. Callers are told to open a new terminal for exactly
    /// this reason.
    pub(super) fn ensure_shared_bin_on_path(home: &Path) -> io::Result<PathEnvOutcome> {
        let bin = shared_bin::shared_bin_dir(home);
        let bin = bin.to_string_lossy().into_owned();
        let current = query_current_path()?;
        match prepend_to_path_list(&current, &bin) {
            None => Ok(PathEnvOutcome::AlreadyConfigured),
            Some(new_value) => {
                write_path(&new_value)?;
                Ok(PathEnvOutcome::Configured {
                    touched: vec![PathBuf::from("HKCU\\Environment\\Path")],
                })
            }
        }
    }
}

#[cfg(windows)]
pub fn ensure_shared_bin_on_path(home: &Path) -> io::Result<PathEnvOutcome> {
    windows::ensure_shared_bin_on_path(home)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn env_file_content_prepends_the_shared_bin_dir() {
        let home = tempfile::tempdir().unwrap();
        let content = env_file_content(home.path());
        let bin = shared_bin::shared_bin_dir(home.path());
        assert!(content.contains(&format!("export PATH=\"{}:$PATH\"", bin.display())));
    }

    #[test]
    fn write_env_file_creates_it_and_reports_changed() {
        let home = tempfile::tempdir().unwrap();
        let path = env_file_path(home.path());
        assert!(!path.exists());
        assert!(write_env_file(home.path()).unwrap());
        assert!(path.is_file());
    }

    #[test]
    fn write_env_file_is_idempotent_on_unchanged_content() {
        let home = tempfile::tempdir().unwrap();
        assert!(write_env_file(home.path()).unwrap());
        assert!(
            !write_env_file(home.path()).unwrap(),
            "second write should be a no-op"
        );
    }

    #[test]
    fn write_env_file_rewrites_a_stale_copy() {
        let home = tempfile::tempdir().unwrap();
        let path = env_file_path(home.path());
        write(&path, "# stale content from a different convention\n");
        assert!(write_env_file(home.path()).unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            env_file_content(home.path())
        );
    }

    #[test]
    fn ensure_guard_at_top_creates_a_new_file_with_the_guard_first() {
        let home = tempfile::tempdir().unwrap();
        let target = home.path().join(".bashrc");
        let env_file = env_file_path(home.path());
        assert!(ensure_guard_at_top(&target, &env_file).unwrap());
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.starts_with(MARKER_OPEN));
        assert!(content.contains(&env_file.to_string_lossy().to_string()));
    }

    #[test]
    fn ensure_guard_at_top_is_idempotent() {
        let home = tempfile::tempdir().unwrap();
        let target = home.path().join(".zshenv");
        let env_file = env_file_path(home.path());
        assert!(ensure_guard_at_top(&target, &env_file).unwrap());
        assert!(!ensure_guard_at_top(&target, &env_file).unwrap());
        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content.matches(MARKER_OPEN).count(), 1);
    }

    #[test]
    fn ensure_guard_at_top_goes_before_existing_content_so_later_lines_still_win() {
        let home = tempfile::tempdir().unwrap();
        let target = home.path().join(".bashrc");
        write(&target, "export PATH=\"/my/own/override:$PATH\"\n");
        let env_file = env_file_path(home.path());
        ensure_guard_at_top(&target, &env_file).unwrap();
        let content = fs::read_to_string(&target).unwrap();
        let guard_pos = content.find(MARKER_OPEN).unwrap();
        let override_pos = content.find("/my/own/override").unwrap();
        assert!(
            guard_pos < override_pos,
            "the guard must come first so the user's own later prepend still wins"
        );
    }

    #[test]
    fn ensure_guard_at_top_preserves_a_leading_shebang() {
        let home = tempfile::tempdir().unwrap();
        let target = home.path().join(".profile");
        write(&target, "#!/bin/sh\necho hello\n");
        let env_file = env_file_path(home.path());
        ensure_guard_at_top(&target, &env_file).unwrap();
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.starts_with("#!/bin/sh\n"));
        assert!(content.contains(MARKER_OPEN));
        assert!(content.ends_with("echo hello\n"));
        let shebang_end = content.find('\n').unwrap();
        let guard_pos = content.find(MARKER_OPEN).unwrap();
        assert!(
            guard_pos > shebang_end,
            "the guard must come after the shebang line"
        );
    }

    #[test]
    fn unix_rc_targets_excludes_bash_profile_when_absent() {
        let home = tempfile::tempdir().unwrap();
        let targets = unix_rc_targets(home.path());
        assert!(!targets.iter().any(|p| p.ends_with(".bash_profile")));
        assert!(targets.iter().any(|p| p.ends_with(".zshenv")));
        assert!(targets.iter().any(|p| p.ends_with(".bashrc")));
        assert!(targets.iter().any(|p| p.ends_with(".profile")));
    }

    #[test]
    fn unix_rc_targets_includes_bash_profile_when_present() {
        let home = tempfile::tempdir().unwrap();
        write(&home.path().join(".bash_profile"), "echo existing\n");
        let targets = unix_rc_targets(home.path());
        assert!(targets.iter().any(|p| p.ends_with(".bash_profile")));
    }

    #[test]
    #[cfg(not(windows))]
    fn ensure_shared_bin_on_path_touches_every_target_on_a_fresh_home() {
        let home = tempfile::tempdir().unwrap();
        let outcome = ensure_shared_bin_on_path(home.path()).unwrap();
        match outcome {
            PathEnvOutcome::Configured { touched } => {
                // env file + at least the 3 always-present rc targets.
                assert!(
                    touched.len() >= 4,
                    "expected env file + rc targets, got {touched:?}"
                );
            }
            PathEnvOutcome::AlreadyConfigured => {
                panic!("a fresh home must not already be configured")
            }
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn ensure_shared_bin_on_path_is_idempotent() {
        let home = tempfile::tempdir().unwrap();
        ensure_shared_bin_on_path(home.path()).unwrap();
        let second = ensure_shared_bin_on_path(home.path()).unwrap();
        assert!(matches!(second, PathEnvOutcome::AlreadyConfigured));
    }

    #[test]
    #[cfg(windows)]
    fn prepend_to_path_list_adds_the_dir_to_an_empty_value() {
        assert_eq!(
            windows::prepend_to_path_list("", "C:\\bin"),
            Some("C:\\bin".to_string())
        );
    }

    #[test]
    #[cfg(windows)]
    fn prepend_to_path_list_prepends_ahead_of_existing_entries() {
        assert_eq!(
            windows::prepend_to_path_list("C:\\a;C:\\b", "C:\\bin"),
            Some("C:\\bin;C:\\a;C:\\b".to_string())
        );
    }

    #[test]
    #[cfg(windows)]
    fn prepend_to_path_list_is_case_insensitive_on_membership() {
        assert_eq!(
            windows::prepend_to_path_list("C:\\Bin;C:\\a", "c:\\bin"),
            None
        );
    }
}
