// MODE: DEV
// PACKAGE: PROD

//! The class of bug this release actually found twice (B1, B2): a plugin's
//! own tracked, shippable file missing from one or both of the two
//! hand-maintained file lists that decide what an install actually
//! delivers -- installer/build-release.sh's own `<plugin>_files()` function
//! (the tarball) and src/installer/src/plugins.rs's own `<PLUGIN>_FILES`
//! constant (what the compiled installer copies, regardless of which
//! artifact supplied the source tree). Neither test-skill-files-manifest.sh
//! nor test-mode-markers.sh look at these five directories at all -- they
//! are not a skill -- which is exactly why both bugs went uncaught until a
//! real tarball install was checked by hand.
//!
//! Folds in one more comparison, at no extra design cost: a real
//! `npm pack --dry-run --json`, so a MODE: DEV file silently leaking into
//! the npm package (test-rjq-active-references.sh's own concern, generalised
//! past the one file it happened to catch) is caught here too.

use crate::report::Report;
use regex::Regex;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// A shippable file that is legitimately absent from a plugin's own
/// `<PLUGIN>_FILES` Rust constant, because a *different* function installs
/// it by its own fixed path instead of from that list. Confirmed, not
/// guessed: `install_tui_hint_plugin_opencode` reads
/// `tui-hint-plugin/opencode/tui-hint-plugin.js` directly and registers it
/// with opencode's own config, entirely separately from
/// `install_tui_hint_plugin_claude`/`TUI_HINT_PLUGIN_CLAUDE_FILES`. Without
/// this, the gate below reports a false positive for the one plugin with
/// two install targets instead of one.
const PLUGINS_RS_CONST_EXCEPTIONS: &[&str] = &["tui-hint-plugin/opencode/tui-hint-plugin.js"];

struct Plugin {
    dir: &'static str,
    build_release_fn: &'static str,
    plugins_rs_const: &'static str,
}

const PLUGINS: &[Plugin] = &[
    Plugin {
        dir: "tui-hint-plugin",
        build_release_fn: "tui_hint_plugin_files",
        plugins_rs_const: "TUI_HINT_PLUGIN_CLAUDE_FILES",
    },
    Plugin {
        dir: "editor-gate-plugin",
        build_release_fn: "editor_gate_plugin_files",
        plugins_rs_const: "EDITOR_GATE_PLUGIN_FILES",
    },
    Plugin {
        dir: "agent-identity-plugin",
        build_release_fn: "agent_identity_plugin_files",
        plugins_rs_const: "AGENT_IDENTITY_PLUGIN_FILES",
    },
    Plugin {
        dir: "decision-reminder-plugin",
        build_release_fn: "decision_reminder_plugin_files",
        plugins_rs_const: "DECISION_REMINDER_PLUGIN_FILES",
    },
    Plugin {
        dir: "chat-interrupt-plugin",
        build_release_fn: "chat_interrupt_plugin_files",
        plugins_rs_const: "CHAT_INTERRUPT_PLUGIN_FILES",
    },
];

/// The marker on the file's own first 25 lines, in whichever comment syntax
/// the extension uses -- the same four forms test-mode-markers.sh reads.
fn marker_of(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let head: String = text.lines().take(25).collect::<Vec<_>>().join("\n");
    for pattern in [
        r"^# MODE: ([A-Z]+)$",
        r"^<!-- MODE: ([A-Z]+) -->$",
        r"^// MODE: ([A-Z]+)$",
    ] {
        let re = Regex::new(&format!("(?m){pattern}")).expect("valid regex");
        if let Some(c) = re.captures(&head) {
            return Some(c[1].to_string());
        }
    }
    None
}

/// Whether `path` is expected to ship: PROD-marked, or a format with no
/// comment syntax a marker could sit in (.json, .js) -- the same two cases
/// every plugin's own file-list function already special-cases.
fn should_ship(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some("json") | Some("js") => true,
        _ => marker_of(path).as_deref() == Some("PROD"),
    }
}

fn expected_files(repo_root: &Path, plugin_dir: &str) -> BTreeSet<String> {
    crate::repo_root::tracked_files_under(repo_root, plugin_dir)
        .into_iter()
        .filter(|relative| should_ship(&repo_root.join(relative)))
        .collect()
}

/// Every `printf '<path>\n'` literal inside the named bash function's body.
fn paths_from_bash_function(text: &str, fn_name: &str) -> BTreeSet<String> {
    let start = match text.find(&format!("{fn_name}() {{")) {
        Some(i) => i,
        None => return BTreeSet::new(),
    };
    let end = text[start..]
        .find("\n}")
        .map(|i| start + i)
        .unwrap_or(text.len());
    let body = &text[start..end];
    let re = Regex::new(r"printf '([^'\n]+)\\n'").expect("valid regex");
    re.captures_iter(body).map(|c| c[1].to_string()).collect()
}

/// Every quoted path inside the named Rust `&[&str]` constant's array body.
fn paths_from_rust_const(text: &str, const_name: &str) -> BTreeSet<String> {
    let start = match text.find(&format!("const {const_name}: &[&str] = &[")) {
        Some(i) => i,
        None => return BTreeSet::new(),
    };
    let end = text[start..]
        .find("];")
        .map(|i| start + i)
        .unwrap_or(text.len());
    let body = &text[start..end];
    let re = Regex::new(r#""([^"]+)""#).expect("valid regex");
    re.captures_iter(body)
        .map(|c| format!("{}/{}", plugin_dir_for_const(const_name), &c[1]))
        .collect()
}

/// The Rust constant's own paths are relative to the plugin directory
/// (`"hooks/lib.sh"`); the comparison set is repo-root-relative, so this
/// finds which plugin owns `const_name` to prefix it back on. O(n) over five
/// entries is simpler than threading the plugin through the extractor.
fn plugin_dir_for_const(const_name: &str) -> &'static str {
    PLUGINS
        .iter()
        .find(|p| p.plugins_rs_const == const_name)
        .map(|p| p.dir)
        .unwrap_or("")
}

fn npm_pack_files(repo_root: &Path) -> Result<BTreeSet<String>, String> {
    let cache_dir = std::env::temp_dir().join(format!(
        "pre-release-check-npm-cache-{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&cache_dir);
    let output = Command::new("npm")
        .args(["pack", "--dry-run", "--json"])
        .env("npm_config_cache", &cache_dir)
        .current_dir(repo_root)
        .output();
    let _ = std::fs::remove_dir_all(&cache_dir);
    let output = output.map_err(|e| format!("could not run npm pack: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "npm pack --dry-run exited non-zero: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("could not parse npm pack --dry-run --json output: {e}"))?;
    let files = value
        .get(0)
        .and_then(|v| v.get("files"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| "npm pack --dry-run --json: unexpected shape".to_string())?;
    Ok(files
        .iter()
        .filter_map(|f| f.get("path").and_then(|p| p.as_str()))
        .map(str::to_string)
        .collect())
}

/// `candidate.md`/`license`/`changelog`, case-insensitive: npm force-includes
/// these regardless of `files`/`.npmignore`, confirmed empirically this
/// release (three plugins' own working negation entries for everything
/// else, dead only for their README). Not a finding either direction.
fn is_npm_forced_basename(path: &str) -> bool {
    let name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    name.starts_with("readme") || name.starts_with("license") || name.starts_with("changelog")
}

fn report_missing(report: &mut Report, plugin: &str, mechanism: &str, missing: &BTreeSet<String>) {
    if missing.is_empty() {
        report.ok(&format!(
            "{plugin}: {mechanism} carries every shippable file"
        ));
    } else {
        for path in missing {
            report.bad(&format!(
                "{plugin}: {path} is shippable but missing from {mechanism}"
            ));
        }
    }
}

/// Any of `plugin_dir`'s own MODE: DEV files (outside `expected`) that
/// nonetheless appear in a real `npm pack --dry-run` -- the README/LICENSE
/// force-include quirk excepted, since that is not a finding either
/// direction.
fn check_npm_leak(
    repo_root: &Path,
    plugin_dir: &str,
    expected: &BTreeSet<String>,
    npm_files: &BTreeSet<String>,
    report: &mut Report,
) {
    let leaked: Vec<String> = crate::repo_root::tracked_files_under(repo_root, plugin_dir)
        .into_iter()
        .filter(|p| !expected.contains(p) && !is_npm_forced_basename(p) && npm_files.contains(p))
        .collect();
    if leaked.is_empty() {
        report.ok(&format!(
            "{plugin_dir}: no MODE: DEV file leaks into the npm package"
        ));
        return;
    }
    for path in &leaked {
        report.bad(&format!(
            "{path}: MODE: DEV but present in npm pack --dry-run's file list"
        ));
    }
}

pub fn gate_plugin_parity(repo_root: &Path, report: &mut Report) {
    let build_release =
        std::fs::read_to_string(repo_root.join("installer/build-release.sh")).unwrap_or_default();
    let plugins_rs =
        std::fs::read_to_string(repo_root.join("src/installer/src/plugins.rs")).unwrap_or_default();
    let npm_files = npm_pack_files(repo_root);

    for plugin in PLUGINS {
        let expected = expected_files(repo_root, plugin.dir);
        let shipped_tarball = paths_from_bash_function(&build_release, plugin.build_release_fn);
        let shipped_installer = paths_from_rust_const(&plugins_rs, plugin.plugins_rs_const);

        report_missing(
            report,
            plugin.dir,
            "build-release.sh's tarball function",
            &expected.difference(&shipped_tarball).cloned().collect(),
        );
        let missing_from_installer: BTreeSet<String> = expected
            .difference(&shipped_installer)
            .filter(|path| !PLUGINS_RS_CONST_EXCEPTIONS.contains(&path.as_str()))
            .cloned()
            .collect();
        report_missing(
            report,
            plugin.dir,
            "plugins.rs's installer constant",
            &missing_from_installer,
        );

        match &npm_files {
            Ok(npm_files) => check_npm_leak(repo_root, plugin.dir, &expected, npm_files, report),
            Err(message) => report.bad(&format!(
                "could not check npm packaging for {}: {message}",
                plugin.dir
            )),
        }
    }
}
