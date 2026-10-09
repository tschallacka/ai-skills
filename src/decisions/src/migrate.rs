// MODE: DEV
// PACKAGE: PROD
//! Meeting a register this version did not write.
//!
//! One path, the same one bug-report's and todo's own migrate modules take:
//! back up the original bytes first, carry forward every open entry that
//! still parses against the current types, and report the rest by id and
//! parse error rather than guessing at an older shape.

use crate::register::{Question, Register};

/// The register version this binary writes. Tracks package.json's own
/// version exactly, the same way bug-report's and todo's own SUPPORTED do;
/// bump it in the same change that bumps the package version.
///
/// Bumped for alpha.5: `Status::Answered` is renamed `Status::Decided` (to
/// read the same way as `todo`'s own `decided` status) and a new terminal
/// `Status::Implemented` is added, set once the chosen option has actually
/// been carried out in the code. An alpha.4 register with an `answered`
/// entry is not silently reinterpreted: that status name no longer exists
/// in this enum, so `attempt` below reports it as unconvertible rather than
/// archiving or guessing, the same honesty this module already gives any
/// other foreign shape it cannot parse.
///
/// Bumped again for alpha.6 in lockstep with every other register skill's
/// SUPPORTED and package.json's own version; this register's own shape did
/// not change this release.
pub const SUPPORTED: &str = "2.0.0-alpha.6";

pub struct Unconvertible {
    pub id: String,
    pub why: String,
}

/// Versioned rather than a single `.back.json`, so a second migration cannot
/// overwrite the evidence from the first.
pub fn backup_path(register_path: &str, version: &str) -> String {
    let stem = register_path.strip_suffix(".json").unwrap_or(register_path);
    let version = if version.is_empty() {
        "unversioned"
    } else {
        version
    };
    format!("{stem}.{version}.back.json")
}

pub fn is_current(version: &str) -> bool {
    version == SUPPORTED
}

/// The version a foreign register claims, for naming the backup. Absent is
/// not an error: an unversioned file still gets a backup, under that name.
pub fn claimed_version(value: &serde_json::Value) -> String {
    value
        .get("skill_version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// One attempt, entry by entry, against the current types. Carried: still
/// open, and every field the current shape needs is present and in
/// vocabulary. Archived: converts cleanly but is already answered or closed,
/// so it stays in the backup rather than padding the live register.
/// Unconvertible: reported with the parse error.
pub fn attempt(value: &serde_json::Value) -> (Vec<Question>, Vec<String>, Vec<Unconvertible>) {
    let mut carried = Vec::new();
    let mut archived = Vec::new();
    let mut unconvertible = Vec::new();

    let entries = value
        .get("questions")
        .and_then(|q| q.as_array())
        .cloned()
        .unwrap_or_default();

    for entry in entries {
        let id = entry
            .get("id")
            .and_then(|i| i.as_str())
            .unwrap_or("<no id>")
            .to_string();

        match serde_json::from_value::<Question>(entry) {
            Ok(question) => {
                if question.status.is_open() {
                    carried.push(question);
                } else {
                    archived.push(question.id);
                }
            }
            Err(error) => unconvertible.push(Unconvertible {
                id,
                why: error.to_string(),
            }),
        }
    }

    (carried, archived, unconvertible)
}

/// Build the register this version writes, from what converted.
pub fn rebuilt(source: &serde_json::Value, carried: Vec<Question>) -> Register {
    Register {
        skill: source
            .get("skill")
            .and_then(|v| v.as_str())
            .unwrap_or("decisions")
            .to_string(),
        skill_version: SUPPORTED.to_string(),
        comment: source
            .get("comment")
            .and_then(|v| v.as_str())
            .unwrap_or("Non-blocking questions raised during work.")
            .to_string(),
        questions: carried,
    }
}

/// What to tell the agent about what did not convert.
pub fn instructions(backup: &str, unconvertible: &[Unconvertible]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} entr{} did not convert. They are intact in {}.\n",
        unconvertible.len(),
        if unconvertible.len() == 1 { "y" } else { "ies" },
        backup
    ));
    out.push_str("Read each one and re-file it, so the register records what you decided:\n\n");
    for item in unconvertible {
        out.push_str(&format!("  {}: {}\n", item.id, item.why));
        out.push_str(&format!("    read {backup} and find \"{}\"\n", item.id));
    }
    out.push_str(
        "\nThen, with the fields that entry actually had:\n\
         \n  decisions add --title \"...\" --option a:... --option b:... \\\n\
         \x20     --priority <urgent|high|normal|low|someday> --context \"...\"\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_register_at_the_supported_version_is_current() {
        assert!(is_current(SUPPORTED));
        assert!(!is_current("1.0.0"));
    }

    #[test]
    fn attempt_carries_open_archives_closed_and_reports_malformed() {
        let value = json!({
            "skill": "decisions",
            "skill_version": "unversioned",
            "comment": "c",
            "questions": [
                {"id": "Q1", "title": "t", "status": "open", "priority": "urgent",
                 "branch": "main", "options": [{"letter": "a", "label": "Yes"}],
                 "context": "", "chosen": null, "resolution": null,
                 "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"},
                {"id": "Q2", "title": "t", "status": "closed", "priority": "normal",
                 "branch": "main", "options": [{"letter": "a", "label": "Yes"}],
                 "context": "", "chosen": "a", "resolution": "done",
                 "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"},
                {"id": "Q3", "title": "t", "status": "not-a-status"}
            ]
        });
        let (carried, archived, unconvertible) = attempt(&value);
        assert_eq!(carried.len(), 1);
        assert_eq!(carried[0].id, "Q1");
        assert_eq!(archived, vec!["Q2".to_string()]);
        assert_eq!(unconvertible.len(), 1);
        assert_eq!(unconvertible[0].id, "Q3");
    }

    #[test]
    fn an_alpha_4_answered_entry_is_reported_unconvertible_not_silently_archived() {
        // `answered` does not exist in the current Status enum (renamed to
        // `decided` for alpha.5); a question this old, already decided but
        // never closed, must not vanish into the backup the way a genuinely
        // resolved `closed` entry does -- it still has outstanding work.
        let value = json!({
            "skill": "decisions",
            "skill_version": "2.0.0-alpha.4",
            "comment": "c",
            "questions": [
                {"id": "Q1", "title": "t", "status": "answered", "priority": "normal",
                 "branch": "main", "options": [{"letter": "a", "label": "Yes"}],
                 "context": "", "chosen": "a", "resolution": null,
                 "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"}
            ]
        });
        let (carried, archived, unconvertible) = attempt(&value);
        assert!(carried.is_empty());
        assert!(archived.is_empty());
        assert_eq!(unconvertible.len(), 1);
        assert_eq!(unconvertible[0].id, "Q1");
    }

    #[test]
    fn backup_path_names_the_claimed_version() {
        assert_eq!(
            backup_path("DECISIONS.json", "1.0.0"),
            "DECISIONS.1.0.0.back.json"
        );
        assert_eq!(
            backup_path("DECISIONS.json", ""),
            "DECISIONS.unversioned.back.json"
        );
    }
}
