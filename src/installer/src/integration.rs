// MODE: DEV
// PACKAGE: PROD
//! Per-skill integration mode (`skill` vs `mcp`) -- ported from
//! installer/src/05-config.sh's `integration_mode_for` and
//! installer/src/50-manifest.sh's `integration_binary_mode`/
//! `integration_modes`/`integration_installed_mode`/`integration_file_allowed`/
//! `remove_stale_integration_binaries`, reading each skill's own
//! `integration.tsv` directly at runtime rather than replicating
//! install.sh's build-time code generation -- same reasoning as
//! requirements.rs reading requires.tsv directly.
//!
//! Only `ai-text-editor` and `chat` ship an `integration.tsv` today; every
//! other skill has exactly one mode (`skill`) and every function here is a
//! no-op for it. A binary named in `integration.tsv` installs only in its
//! declared mode; everything else (SKILL.md, schemas, a mode-free adapter
//! like `ai-text-editor-server`/`chat-server-rs` that both modes share) is
//! mode-free and always installs.
//!
//! `mcp_adapter_path` also lives here rather than in mcp.rs: it is the
//! integration-mode question "which binary, if any, did this install leave
//! in mcp mode" answered from the INSTALLED directory (not the source tree,
//! which the mode gate has already decided against) -- ported from
//! installer/src/72-mcp-registration.sh's function of the same name.

use std::fs;
use std::path::{Path, PathBuf};

fn parse_rows(source_root: &Path, skill: &str) -> Vec<(String, String)> {
    let path = source_root.join(skill).join("integration.tsv");
    let Ok(content) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in content.lines() {
        if line.is_empty() || line.starts_with('#') || line.starts_with("mode\t") {
            continue;
        }
        let mut cols = line.split('\t');
        let (Some(mode), Some(binary)) = (cols.next(), cols.next()) else {
            continue;
        };
        out.push((mode.to_string(), binary.to_string()));
    }
    out
}

/// The mode a `bin/` filename declares, or `None` when it is mode-free (not
/// named in `integration.tsv` at all, including every file when the skill
/// has no `integration.tsv`). A trailing `.exe` is stripped before matching,
/// same as install.sh's generated table listing both the bare and `.exe`
/// name for every declared binary.
pub fn binary_mode(source_root: &Path, skill: &str, filename: &str) -> Option<String> {
    let bare = filename.strip_suffix(".exe").unwrap_or(filename);
    parse_rows(source_root, skill)
        .into_iter()
        .find(|(_, binary)| binary == bare)
        .map(|(mode, _)| mode)
}

/// Every mode this skill declares, in `integration.tsv` order with
/// duplicates removed (`ai-text-editor`/`chat` -> `["skill", "mcp"]`; every
/// other skill -> `[]`, meaning it offers no choice at all).
pub fn modes(source_root: &Path, skill: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (mode, _) in parse_rows(source_root, skill) {
        if !out.contains(&mode) {
            out.push(mode);
        }
    }
    out
}

/// Which mode's binaries are already on disk at `destination`, or `None`
/// when there are none (a first install). `Some(None)` is not a case this
/// returns: finding binaries for two different modes -- a half-finished
/// earlier switch -- is reported on stderr and treated the same as finding
/// none, matching install.sh's `integration_installed_mode` refusing to
/// guess rather than picking one.
pub fn installed_mode(source_root: &Path, skill: &str, destination: &Path) -> Option<String> {
    let bin_dir = destination.join("bin");
    let Ok(triples) = fs::read_dir(&bin_dir) else {
        return None;
    };
    let mut found: Option<String> = None;
    for triple in triples.filter_map(|e| e.ok()) {
        let Ok(files) = fs::read_dir(triple.path()) else {
            continue;
        };
        for file in files.filter_map(|e| e.ok()) {
            if !file.path().is_file() {
                continue;
            }
            let filename = file.file_name();
            let Some(mode) = binary_mode(source_root, skill, &filename.to_string_lossy()) else {
                continue;
            };
            match &found {
                Some(existing) if *existing != mode => {
                    eprintln!(
                        "installer: {} has binaries for both {existing} and {mode} modes; \
                         a previous switch may be unfinished. Pass --integration to say which \
                         mode to keep.",
                        destination.display()
                    );
                    return None;
                }
                _ => found = Some(mode),
            }
        }
    }
    found
}

/// The adapter binary an mcp-mode install of `skill` left in place under
/// `installed_dir` (`installed_dir/bin/<triple>/<file>`), or `None` when
/// this skill is not in mcp mode there -- read from what the mode gate
/// already decided to leave on disk, not by asking `integration.tsv` a
/// second time.
pub fn mcp_adapter_path(source_root: &Path, skill: &str, installed_dir: &Path) -> Option<PathBuf> {
    let bin_dir = installed_dir.join("bin");
    let triples = fs::read_dir(&bin_dir).ok()?;
    for triple in triples.filter_map(|e| e.ok()) {
        let Ok(files) = fs::read_dir(triple.path()) else {
            continue;
        };
        for file in files.filter_map(|e| e.ok()) {
            let path = file.path();
            if !path.is_file() {
                continue;
            }
            let filename = file.file_name();
            if binary_mode(source_root, skill, &filename.to_string_lossy()).as_deref() == Some("mcp") {
                return Some(path);
            }
        }
    }
    None
}

/// The mode to install `skill` in at `destination`: an explicit choice for
/// this run outranks whatever is already on disk, which outranks the
/// `skill` default. `skill` is the default only on a first install --
/// install.sh's T109: an unattended update with no flag must carry an
/// existing mcp install forward, not silently revert it.
pub fn resolve_mode(
    source_root: &Path,
    skill: &str,
    destination: Option<&Path>,
    explicit: Option<&str>,
) -> String {
    if let Some(mode) = explicit {
        return mode.to_string();
    }
    if let Some(destination) = destination {
        if let Some(mode) = installed_mode(source_root, skill, destination) {
            return mode;
        }
    }
    "skill".to_string()
}

/// Does `relative` (forward-slash, source-tree-relative) belong in `mode`?
/// Only `bin/` paths carry a mode; everything else always installs.
pub fn file_allowed(source_root: &Path, skill: &str, relative: &str, mode: &str) -> bool {
    if !relative.starts_with("bin/") {
        return true;
    }
    let filename = relative.rsplit('/').next().unwrap_or(relative);
    match binary_mode(source_root, skill, filename) {
        Some(declared) => declared == mode,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_integration(dir: &Path, skill: &str, content: &str) {
        let skill_dir = dir.join(skill);
        fs::create_dir_all(&skill_dir).unwrap();
        let mut f = fs::File::create(skill_dir.join("integration.tsv")).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    const SAMPLE: &str = "mode\tbinary\twhy\n\
        skill\tai-text-editor\tShort-lived client\n\
        mcp\tai-text-editor-mcp\tMCP bridge\n";

    #[test]
    fn a_skill_with_no_integration_tsv_declares_no_modes() {
        let dir = tempfile::tempdir().unwrap();
        assert!(modes(dir.path(), "todo").is_empty());
        assert_eq!(binary_mode(dir.path(), "todo", "todo"), None);
    }

    #[test]
    fn modes_lists_every_declared_mode_in_order_without_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        assert_eq!(modes(dir.path(), "ai-text-editor"), vec!["skill", "mcp"]);
    }

    #[test]
    fn binary_mode_matches_the_declared_binary_and_its_exe_variant() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        assert_eq!(
            binary_mode(dir.path(), "ai-text-editor", "ai-text-editor-mcp"),
            Some("mcp".to_string())
        );
        assert_eq!(
            binary_mode(dir.path(), "ai-text-editor", "ai-text-editor-mcp.exe"),
            Some("mcp".to_string())
        );
    }

    #[test]
    fn a_binary_not_named_in_integration_tsv_is_mode_free() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        assert_eq!(
            binary_mode(dir.path(), "ai-text-editor", "ai-text-editor-server"),
            None
        );
    }

    #[test]
    fn file_allowed_is_always_true_outside_bin() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        assert!(file_allowed(dir.path(), "ai-text-editor", "SKILL.md", "mcp"));
    }

    #[test]
    fn file_allowed_gates_a_declared_bin_file_on_the_resolved_mode() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        let path = "bin/x86_64-unknown-linux-musl/ai-text-editor-mcp";
        assert!(file_allowed(dir.path(), "ai-text-editor", path, "mcp"));
        assert!(!file_allowed(dir.path(), "ai-text-editor", path, "skill"));
    }

    #[test]
    fn file_allowed_lets_a_mode_free_bin_file_through_in_every_mode() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        let path = "bin/x86_64-unknown-linux-musl/ai-text-editor-server";
        assert!(file_allowed(dir.path(), "ai-text-editor", path, "mcp"));
        assert!(file_allowed(dir.path(), "ai-text-editor", path, "skill"));
    }

    #[test]
    fn installed_mode_is_none_on_a_first_install() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        let dest = tempfile::tempdir().unwrap();
        assert_eq!(installed_mode(dir.path(), "ai-text-editor", dest.path()), None);
    }

    #[test]
    fn installed_mode_detects_the_mode_already_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        let dest = tempfile::tempdir().unwrap();
        let bin = dest.path().join("bin/x86_64-unknown-linux-musl");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("ai-text-editor-mcp"), "").unwrap();
        assert_eq!(
            installed_mode(dir.path(), "ai-text-editor", dest.path()),
            Some("mcp".to_string())
        );
    }

    #[test]
    fn mcp_adapter_path_finds_the_installed_mcp_binary() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        let dest = tempfile::tempdir().unwrap();
        let bin = dest.path().join("bin/x86_64-unknown-linux-musl");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("ai-text-editor-mcp"), "").unwrap();
        let found = mcp_adapter_path(dir.path(), "ai-text-editor", dest.path()).unwrap();
        assert_eq!(found, bin.join("ai-text-editor-mcp"));
    }

    #[test]
    fn mcp_adapter_path_is_none_when_only_the_skill_binary_is_installed() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        let dest = tempfile::tempdir().unwrap();
        let bin = dest.path().join("bin/x86_64-unknown-linux-musl");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("ai-text-editor"), "").unwrap();
        assert!(mcp_adapter_path(dir.path(), "ai-text-editor", dest.path()).is_none());
    }

    #[test]
    fn installed_mode_refuses_to_guess_between_two_modes_present_at_once() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        let dest = tempfile::tempdir().unwrap();
        let bin = dest.path().join("bin/x86_64-unknown-linux-musl");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("ai-text-editor-mcp"), "").unwrap();
        fs::write(bin.join("ai-text-editor"), "").unwrap();
        assert_eq!(installed_mode(dir.path(), "ai-text-editor", dest.path()), None);
    }

    #[test]
    fn resolve_mode_prefers_explicit_over_detected_over_default() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        let dest = tempfile::tempdir().unwrap();
        let bin = dest.path().join("bin/x86_64-unknown-linux-musl");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("ai-text-editor-mcp"), "").unwrap();

        assert_eq!(
            resolve_mode(dir.path(), "ai-text-editor", Some(dest.path()), Some("skill")),
            "skill"
        );
        assert_eq!(
            resolve_mode(dir.path(), "ai-text-editor", Some(dest.path()), None),
            "mcp"
        );
    }

    #[test]
    fn resolve_mode_falls_back_to_skill_on_a_first_install_with_no_explicit_choice() {
        let dir = tempfile::tempdir().unwrap();
        write_integration(dir.path(), "ai-text-editor", SAMPLE);
        assert_eq!(resolve_mode(dir.path(), "ai-text-editor", None, None), "skill");
    }
}
