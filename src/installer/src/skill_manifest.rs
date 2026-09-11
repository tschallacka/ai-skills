// MODE: DEV
// PACKAGE: PROD
//! The authoritative "which files does this skill ship" answer, when it can
//! be gotten: install.sh's own `skill_files()` (installer/src/50-manifest.sh)
//! is a hand-maintained, per-skill bash function -- not a data file, so
//! there is nothing for this installer to parse into a Rust table without
//! re-deriving install.sh's own logic and risking it drifting out of sync.
//! Rather than approximate it (install.rs's `should_ship` MODE-marker
//! heuristic, kept as the fallback below), this extracts `skill_files()`
//! and its one helper (`skill_artifact_files`) out of `install.sh` as text
//! and runs them with bash itself -- so the answer is install.sh's own,
//! byte-for-byte, whenever `install.sh` is sitting next to `--source`
//! (a raw checkout). Never executes the rest of install.sh: sourcing the
//! whole file would run its main flow (the picker, downloads, exit calls)
//! as a side effect of asking it a question.
//!
//! `install.sh` is never shipped inside a `build-release.sh` tarball --
//! `bootstrap.sh` downloads the skill payload alone -- so this is a no-op
//! (`None`) for the common end-user path, where install.rs's own MODE-
//! marker filter already does the right thing because that tarball is
//! already prod-only.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Pulls one `name() { ... }` function's exact source text out of
/// `content`, matched on a closing `}` alone on its own line -- true for
/// every function in this codebase's own style (confirmed for
/// `skill_files`/`skill_artifact_files` specifically), including one whose
/// body contains `case`/`esac`, heredocs, and nested `if`/`fi`, none of
/// which put a bare `}` at the start of a line.
fn extract_function(content: &str, name: &str) -> Option<String> {
    let marker = format!("\n{name}() {{\n");
    let start = content.find(&marker)? + 1;
    let body = &content[start..];
    let end = body.find("\n}\n")?;
    Some(body[..end + 2].to_string())
}

/// The exact relative paths `install.sh`'s own `skill_files(skill, package)`
/// would print for this host, or `None` when that answer cannot be
/// obtained (no `install.sh` next to `source_root`, its `skill_files`/
/// `skill_artifact_files` functions could not be extracted, or `bash` is
/// not on PATH) -- the caller falls back to its own heuristic in every
/// `None` case rather than failing the install over it.
pub fn skill_files_via_install_sh(source_root: &Path, skill: &str, dev_build: bool) -> Option<Vec<String>> {
    let install_sh = source_root.join("install.sh");
    let content = fs::read_to_string(&install_sh).ok()?;
    let skill_files_fn = extract_function(&content, "skill_files")?;
    let skill_artifact_files_fn = extract_function(&content, "skill_artifact_files")?;

    let mut script = String::new();
    script.push_str(&skill_artifact_files_fn);
    script.push('\n');
    script.push_str(&skill_files_fn);
    script.push('\n');
    script.push_str("skill_files \"$1\" \"$2\"\n");

    let package = if dev_build { "dev" } else { "prod" };
    let output = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .arg("skill_manifest") // becomes $0 inside -c's script, so $1/$2 line up
        .arg(skill)
        .arg(package)
        .env("SOURCE_ROOT", source_root)
        .env("DEV_BUILD", if dev_build { "1" } else { "0" })
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skip_without_bash() -> bool {
        std::env::split_paths(&std::env::var("PATH").unwrap_or_default())
            .any(|dir| dir.join("bash").is_file())
            .then_some(())
            .is_none()
    }

    #[test]
    fn extract_function_pulls_exactly_one_function_body() {
        let content = "\nfoo() {\n    echo bar\n}\n\nbaz() {\n    echo qux\n}\n";
        let foo = extract_function(content, "foo").unwrap();
        assert!(foo.contains("echo bar"));
        assert!(!foo.contains("echo qux"));
    }

    #[test]
    fn extract_function_handles_a_body_with_nested_braces_in_expansions() {
        let content = "\nfoo() {\n    local x=\"${1:-default}\"\n    echo \"$x\"\n}\n";
        let foo = extract_function(content, "foo").unwrap();
        assert!(foo.contains("${1:-default}"));
    }

    #[test]
    fn no_install_sh_next_to_source_root_is_none() {
        if skip_without_bash() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        assert!(skill_files_via_install_sh(dir.path(), "todo", false).is_none());
    }

    #[test]
    fn a_minimal_install_sh_produces_its_own_skill_files_answer() {
        if skip_without_bash() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("install.sh"),
            "\nskill_artifact_files() {\n    local skill=\"$1\"\n    shift\n    for relative in \"$@\"; do\n        [ -f \"$SOURCE_ROOT/$skill/$relative\" ] && printf '%s\\n' \"$relative\"\n    done\n}\n\nskill_files() {\n    local package=\"${2:-prod}\"\n    case \"$1\" in\n        widget)\n            printf '%s\\n' SKILL.md requires.tsv\n            [ \"$package\" = dev ] || return 0\n            printf '%s\\n' MAINTAINER.md\n            ;;\n    esac\n}\n",
        )
        .unwrap();
        let files = skill_files_via_install_sh(dir.path(), "widget", false).unwrap();
        assert_eq!(files, vec!["SKILL.md", "requires.tsv"]);

        let dev_files = skill_files_via_install_sh(dir.path(), "widget", true).unwrap();
        assert_eq!(dev_files, vec!["SKILL.md", "requires.tsv", "MAINTAINER.md"]);
    }
}
