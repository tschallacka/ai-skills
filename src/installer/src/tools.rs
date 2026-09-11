// MODE: DEV
// PACKAGE: PROD
//! Install-hint text for a missing runtime tool -- ported from
//! installer/tools.tsv's `hint`/`default` rows and the
//! `runtime_tool_install_hint()` function installer/build.sh generates from
//! them.
//!
//! Unlike requires.tsv and integration.tsv (both `MODE: PROD`, shipped per
//! skill in the release payload and read from the extracted source tree at
//! runtime), tools.tsv itself is `MODE: DEV` and never ships -- install.sh
//! only ever carries the table's ALREADY-GENERATED bash form, baked in at
//! `installer/build.sh` time. This module does the equivalent for the Rust
//! binary: `include_str!` embeds tools.tsv's bytes into the compiled
//! `installer` binary at build time (which always runs from a full dev
//! checkout that has the file, the same way `installer/build.sh` does), so
//! the binary carries the table even though the release tree it installs
//! from does not.

const TOOLS_TSV: &str = include_str!("../../../installer/tools.tsv");

struct HintRow {
    tool: String,
    condition: String,
    group: u32,
    probe: String,
    text: String,
}

struct DefaultRow {
    condition: String,
    probe: String,
    text: String,
}

fn parse_rows() -> (Vec<HintRow>, Vec<DefaultRow>) {
    let mut hints = Vec::new();
    let mut defaults = Vec::new();
    for line in TOOLS_TSV.lines() {
        if line.is_empty() || line.starts_with('#') || line.starts_with("kind\t") {
            continue;
        }
        let mut cols = line.splitn(6, '\t');
        let (Some(kind), Some(tool), Some(condition), Some(group), Some(probe), Some(text)) = (
            cols.next(),
            cols.next(),
            cols.next(),
            cols.next(),
            cols.next(),
            cols.next(),
        ) else {
            continue;
        };
        let Ok(group) = group.parse::<u32>() else {
            continue;
        };
        match kind {
            "hint" => hints.push(HintRow {
                tool: tool.to_string(),
                condition: condition.to_string(),
                group,
                probe: probe.to_string(),
                text: text.to_string(),
            }),
            "default" if tool == "*" => defaults.push(DefaultRow {
                condition: condition.to_string(),
                probe: probe.to_string(),
                text: text.to_string(),
            }),
            _ => {}
        }
    }
    (hints, defaults)
}

fn condition_matches(condition: &str, os: &str, arch: &str) -> bool {
    let haystack_matches = |pattern: &str| -> bool {
        match pattern.split_once(':') {
            Some((os_pattern, arch_pattern)) => {
                glob_match(os_pattern, os) && glob_match(arch_pattern, arch)
            }
            None => false,
        }
    };
    condition.split('|').any(|alt| haystack_matches(alt.trim()))
}

fn glob_match(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((prefix, suffix)) => {
            text.len() >= prefix.len() + suffix.len()
                && text.starts_with(prefix)
                && text.ends_with(suffix)
        }
    }
}

fn probe_resolves(probe: &str) -> bool {
    probe == "-" || crate::requirements::tool_on_path(probe)
}

fn host_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "Darwin"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else if cfg!(windows) {
        "Windows_NT"
    } else {
        "unknown"
    }
}

fn host_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        other => other,
    }
}

/// The install instruction for a missing `tool` on this host: one line per
/// group (ascending), each the first row in that group whose condition
/// matches this host and whose probe (a command that must itself be on
/// PATH, or `-` for a row that always applies) resolves. Falls back to the
/// `default` row (`  install %s via your system package manager`) when
/// `tool` has no hint rows of its own at all.
pub fn install_hint(tool: &str) -> Vec<String> {
    let (hints, defaults) = parse_rows();
    let os = host_os();
    let arch = host_arch();
    let mut rows: Vec<&HintRow> = hints
        .iter()
        .filter(|r| r.tool == tool && condition_matches(&r.condition, os, arch))
        .collect();
    if rows.is_empty() {
        return defaults
            .iter()
            .filter(|r| condition_matches(&r.condition, os, arch) && probe_resolves(&r.probe))
            .map(|r| r.text.replace("%s", tool))
            .collect();
    }
    rows.sort_by_key(|r| r.group);
    let mut out = Vec::new();
    let mut groups: Vec<u32> = rows.iter().map(|r| r.group).collect();
    groups.dedup();
    for group in groups {
        if let Some(row) = rows
            .iter()
            .filter(|r| r.group == group)
            .find(|r| probe_resolves(&r.probe))
        {
            out.push(row.text.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_tool_has_at_least_one_hint_line() {
        let hint = install_hint("bash");
        assert!(!hint.is_empty());
    }

    #[test]
    fn an_unknown_tool_falls_back_to_the_default_row() {
        let hint = install_hint("definitely-not-a-real-tool-xyz");
        assert_eq!(hint.len(), 1);
        assert!(hint[0].contains("definitely-not-a-real-tool-xyz"));
        assert!(hint[0].contains("system package manager"));
    }

    #[test]
    fn memlimit_carries_more_than_one_group_line() {
        // memlimit's rows in tools.tsv span groups 1-3 (install command,
        // alternative, and a closing note) all with condition *:*, so all
        // three should resolve regardless of host.
        let hint = install_hint("memlimit");
        assert!(hint.len() >= 2, "expected multiple lines, got: {hint:?}");
    }

    #[test]
    fn condition_matching_is_os_and_arch_aware() {
        assert!(condition_matches("Darwin:*", "Darwin", "arm64"));
        assert!(!condition_matches("Darwin:*", "Linux", "x86_64"));
        assert!(condition_matches("*:*", "Linux", "x86_64"));
    }
}
