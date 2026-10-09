// MODE: DEV
// PACKAGE: PROD

//! Each register skill carries its own hand-maintained SUPPORTED constant
//! (migrate.rs), bumped in lockstep with package.json's version but never
//! wired to it: confirmed the hard way this release, a bumped package.json
//! with a stale SUPPORTED refuses every CLI write, and -- with no error
//! message at all -- silently drops decided-question reporting on a read
//! path that checks the version but does not refuse on a mismatch.
//!
//! A second, related but genuinely judgement-dependent class: a test
//! fixture's own `"skill_version"` literal predating the bump. Most need
//! bumping to the current version; migrate.rs's own unit tests deliberately
//! keep an old one, to exercise the upgrade recipe FROM that version. This
//! file only lists candidates; `gate_version_consistency` is the only hard
//! pass/fail here.

use crate::repo_root::tracked_files;
use crate::report::Report;
use regex::Regex;
use std::path::Path;

struct RegisterCrate {
    name: &'static str,
    migrate_rs: &'static str,
}

const REGISTER_CRATES: &[RegisterCrate] = &[
    RegisterCrate {
        name: "bug-report",
        migrate_rs: "src/bug-report/src/migrate.rs",
    },
    RegisterCrate {
        name: "todo",
        migrate_rs: "src/todo/src/migrate.rs",
    },
    RegisterCrate {
        name: "decisions",
        migrate_rs: "src/decisions/src/migrate.rs",
    },
];

fn supported_version(repo_root: &Path, migrate_rs: &str) -> Option<String> {
    let text = std::fs::read_to_string(repo_root.join(migrate_rs)).ok()?;
    let re = Regex::new(r#"pub const SUPPORTED: &str = "([^"]+)";"#).ok()?;
    re.captures(&text).map(|c| c[1].to_string())
}

pub fn gate_version_consistency(repo_root: &Path, package_version: &str, report: &mut Report) {
    for reg in REGISTER_CRATES {
        match supported_version(repo_root, reg.migrate_rs) {
            Some(v) if v == package_version => {
                report.ok(&format!(
                    "{}: SUPPORTED matches package.json ({v})",
                    reg.name
                ));
            }
            Some(v) => report.bad(&format!(
                "{}: SUPPORTED is {v}, package.json is {package_version} -- bump {} and its own test fixtures",
                reg.name, reg.migrate_rs
            )),
            None => report.bad(&format!(
                "{}: could not read SUPPORTED from {}",
                reg.name, reg.migrate_rs
            )),
        }
    }
}

/// Whether `value` is shaped like a real version ("1.0", "1.4.2",
/// "2.0.0-alpha.1") rather than something that merely matched the
/// `"skill_version": "..."` pattern incidentally -- a printf placeholder
/// (`%s`), a Rust format-string placeholder (`{}`), an unexpanded shell
/// variable (`$package_version`), or a bare word like `test`. At least two
/// dot-separated numeric segments, starting with a digit.
fn looks_like_version(value: &str) -> bool {
    let re = Regex::new(r"^[0-9]+(\.[0-9]+){1,}").expect("valid regex");
    re.is_match(value)
}

/// The first ```json fenced block in a register skill's own `SKILL.md`,
/// which is always a worked example of what THIS version writes -- unlike a
/// test fixture, it has no legitimate reason to show a stale
/// `"skill_version"`, so it is checked as its own hard gate rather than
/// folded into the judgement-call scan below.
fn skill_md_example_version(repo_root: &Path, skill_name: &str) -> Option<String> {
    let text = std::fs::read_to_string(repo_root.join(skill_name).join("SKILL.md")).ok()?;
    let mut json = String::new();
    let mut inside = false;
    for line in text.lines() {
        if !inside && line.trim() == "```json" {
            inside = true;
        } else if inside && line.trim() == "```" {
            break;
        } else if inside {
            json.push_str(line);
            json.push('\n');
        }
    }
    let re = Regex::new(r#""skill_version"\s*:\s*"([^"]+)""#).ok()?;
    re.captures(&json).map(|c| c[1].to_string())
}

/// Hard gate: each register skill's own `SKILL.md` worked example must show
/// the current version -- a stale one is documentation telling a reader to
/// write what the current binary will refuse, not a judgement call.
pub fn gate_skill_md_examples(repo_root: &Path, package_version: &str, report: &mut Report) {
    for reg in REGISTER_CRATES {
        match skill_md_example_version(repo_root, reg.name) {
            Some(v) if v == package_version => report.ok(&format!(
                "{}/SKILL.md: worked example's skill_version matches package.json ({v})",
                reg.name
            )),
            Some(v) => report.bad(&format!(
                "{}/SKILL.md: worked example's skill_version is {v}, package.json is {package_version} -- bump the example",
                reg.name
            )),
            None => report.note(&format!(
                "{}/SKILL.md: no skill_version literal found in its first json example",
                reg.name
            )),
        }
    }
}

/// Every tracked file (outside the register crates' own migrate.rs, which
/// legitimately tests upgrading FROM an old version, and their SKILL.md,
/// which `gate_skill_md_examples` already hard-gates) whose `"skill_version"`
/// field literal is version-shaped and not the current package version.
/// Reported as notes, not failures: CHECKLIST.md item -- each one needs a
/// human or agent judgement call, not a mechanical bump. Several version-
/// shaped hits are themselves expected to be benign (a fixture seeded
/// directly, never through the CLI's version-refusing write path, to test
/// something unrelated to version compatibility) -- that judgement is
/// exactly what this note defers rather than guesses at.
pub fn note_stale_version_literals(repo_root: &Path, package_version: &str, report: &mut Report) {
    let mut exempt: Vec<String> = REGISTER_CRATES
        .iter()
        .map(|r| r.migrate_rs.to_string())
        .collect();
    exempt.extend(
        REGISTER_CRATES
            .iter()
            .map(|r| format!("{}/SKILL.md", r.name)),
    );
    let re = Regex::new(r#""skill_version"\s*:\s*"([^"]+)""#).expect("valid regex");
    let mut hits: Vec<String> = Vec::new();
    for file in tracked_files(repo_root) {
        if exempt.contains(&file) || file.ends_with(".archive.json") || file.ends_with(".back.json")
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(repo_root.join(&file)) else {
            continue;
        };
        for cap in re.captures_iter(&text) {
            let value = &cap[1];
            if value != package_version && value != "unversioned" && looks_like_version(value) {
                hits.push(format!("{file}: \"{value}\""));
            }
        }
    }
    hits.sort();
    hits.dedup();
    if hits.is_empty() {
        report.ok("no non-current skill_version literal found outside a register's own migrate.rs");
        return;
    }
    report.note(&format!(
        "{} fixture reference(s) to a skill_version other than {package_version} -- CHECKLIST item",
        hits.len()
    ));
    for hit in &hits {
        report.note(&format!("  {hit}"));
    }
}

#[cfg(test)]
mod tests {
    use super::looks_like_version;

    #[test]
    fn real_versions_are_recognised() {
        assert!(looks_like_version("1.0"));
        assert!(looks_like_version("1.4.2"));
        assert!(looks_like_version("2.0.0-alpha.1"));
    }

    #[test]
    fn placeholders_are_not_mistaken_for_versions() {
        assert!(!looks_like_version("%s"));
        assert!(!looks_like_version("{}"));
        assert!(!looks_like_version("$package_version"));
        assert!(!looks_like_version("test"));
        assert!(!looks_like_version("unversioned"));
    }
}
