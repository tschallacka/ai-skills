// MODE: DEV
// PACKAGE: PROD
//! Meeting a register this version did not write.
//!
//! There is no compatibility layer here and there will not be one: no table of
//! known versions, no per-version branches, no code that keeps reading an older
//! shape. This repository does not carry backwards-compatible code, and a
//! converter full of version guards is exactly that with a friendlier name.
//!
//! So there is one path, and it makes one attempt:
//!
//! 1. copy the original bytes to a VERSIONED backup, before parsing anything;
//! 2. try each entry against the CURRENT types — the ones that fit and are
//!    still open are carried;
//! 3. report everything else, naming the tools to move it by hand.
//!
//! Step 3 is the design, not a fallback. The driver is an agent that can read
//! JSON and call a command, so the honest thing is to hand it the entry and the
//! two commands rather than to grow logic guessing at what an older field meant.
//! Nothing is lost by refusing: the backup holds the original, and the report
//! names every unconverted entry each time the register is read.

use crate::register::{Bug, Register};

/// The register version this binary writes.
pub const SUPPORTED: &str = "2.0.0-alpha.1";

pub struct Unconvertible {
    pub id: String,
    pub why: String,
}

/// Versioned rather than a single `.back.json`, so a second migration cannot
/// overwrite the evidence from the first and the name says which shape it holds.
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

/// The version a foreign register claims, for naming the backup. Absent is not
/// an error: an unversioned file still gets a backup, under that name.
pub fn claimed_version(value: &serde_json::Value) -> String {
    value
        .get("skill_version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// One attempt, entry by entry, against the current types.
///
/// Carried: still open, and every field the current shape needs is present and
/// in vocabulary. Archived: converts cleanly but is already closed, so it stays
/// in the backup rather than padding the live register. Unconvertible: reported
/// with the parse error, which names the field and is more use than any message
/// this function could invent.
pub fn attempt(value: &serde_json::Value) -> (Vec<Bug>, Vec<String>, Vec<Unconvertible>) {
    let mut carried = Vec::new();
    let mut archived = Vec::new();
    let mut unconvertible = Vec::new();

    let entries = value
        .get("bugs")
        .and_then(|b| b.as_array())
        .cloned()
        .unwrap_or_default();

    for entry in entries {
        let id = entry
            .get("id")
            .and_then(|i| i.as_str())
            .unwrap_or("<no id>")
            .to_string();

        match serde_json::from_value::<Bug>(entry) {
            Ok(bug) => {
                if bug.status.is_open() {
                    carried.push(bug);
                } else {
                    archived.push(bug.id);
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
pub fn rebuilt(source: &serde_json::Value, carried: Vec<Bug>) -> Register {
    let mut register = Register {
        skill: source
            .get("skill")
            .and_then(|v| v.as_str())
            .unwrap_or("bug-report")
            .to_string(),
        skill_version: SUPPORTED.to_string(),
        comment: source
            .get("comment")
            .and_then(|v| v.as_str())
            .unwrap_or("Defects found in this project.")
            .to_string(),
        bugs: carried,
    };
    register.sort();
    register
}

/// What to tell the agent about what did not convert. Names the tools rather
/// than describing the old shape: the backup is the source of truth, and
/// `bugs add` is the only writer that validates.
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
        // The backup is plain JSON, so an agent can read it directly. rjq is
        // offered as a convenience and not required: this skill declares no
        // tools, and an instruction that assumed one would put the dependency
        // straight back.
        out.push_str(&format!("    read {backup} and find \"{}\"\n", item.id));
        out.push_str(&format!(
            "      (or, with rjq: rjq -r '.bugs[] | select(.id == \"{}\")' {})\n",
            item.id, backup
        ));
    }
    out.push_str(
        "\nThen, with the fields that entry actually had:\n\
         \n  bugs add --title \"...\" --reproduce \"...\" --observed \"...\" \\\n\
         \x20     --expected \"...\" --severity <blocking|major|minor|cosmetic> \\\n\
         \x20     --status <reported|confirmed> [--mechanism \"...\"]\n",
    );
    out
}
