// MODE: DEV
// PACKAGE: PROD
//! Resolving duplicate ids left behind by a rebase, without a git conflict.
//!
//! `resolve.rs` handles a live merge conflict, where git hands us three named
//! stages. A rebase can also leave two array entries sharing one id with no
//! conflict marker at all — nothing textual collided, the same id was simply
//! carried twice — and `Register::findings` already reports that as `duplicate
//! ids: …`. Until now the only fix was hand-editing the JSON, which is exactly
//! how B312 happened: a manual dedup pass kept the still-open copy of B95 over
//! the one that had already been fixed and verified, silently reopening it.
//!
//! So the rule here is narrow on purpose: a closed status (anything but
//! reported/confirmed) always outranks an open one, because a closure carries
//! its own evidence (fix + verification, or a reason) and an open entry does
//! not — there is no way an open duplicate could be the more informed copy.
//! Two entries that disagree in any other way are refused rather than guessed
//! at, the same way `resolve.rs` refuses a real divergence: this tool would
//! rather print both and ask than pick the wrong one and be silently believed.

use crate::register::{Bug, Register, Status};

fn identical(a: &Bug, b: &Bug) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}

/// One id that could not be resolved automatically, with enough of each
/// duplicate shown to act on: whoever reads this should not have to reopen the
/// file just to see what disagreed.
pub struct Ambiguous {
    pub id: String,
    pub summary: Vec<String>,
}

pub struct Outcome {
    /// The register with every resolvable duplicate group collapsed to one
    /// entry. Unset when any group was ambiguous, so a caller cannot mistake a
    /// partial dedupe for a complete one.
    pub register: Option<Register>,
    pub kept: Vec<String>,
    pub ambiguous: Vec<Ambiguous>,
}

/// The register's own on-disk spelling (kebab-case), not Rust's Debug form —
/// this is user-facing output and should read like the rest of the tool.
fn status_name(status: Status) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn one_line(bug: &Bug) -> String {
    format!(
        "{} (updated_at {}, found_by {})",
        status_name(bug.status),
        bug.updated_at,
        bug.found_by
    )
}

/// Pick the winner of one id's duplicates, or explain why none can be picked.
fn resolve_group(id: &str, entries: Vec<&Bug>) -> Result<(Bug, String), Ambiguous> {
    if entries.windows(2).all(|pair| identical(pair[0], pair[1])) {
        let kept = entries[0].clone();
        return Ok((
            kept,
            format!("{id}: {} identical copies, kept one", entries.len()),
        ));
    }

    let closed: Vec<&&Bug> = entries.iter().filter(|b| !b.status.is_open()).collect();
    let open: Vec<&&Bug> = entries.iter().filter(|b| b.status.is_open()).collect();

    if closed.len() == 1 && !open.is_empty() {
        let kept = (*closed[0]).clone();
        return Ok((
            kept,
            format!(
                "{id}: kept the {} entry, dropped {} still-open duplicate{}",
                status_name(closed[0].status),
                open.len(),
                if open.len() == 1 { "" } else { "s" }
            ),
        ));
    }

    Err(Ambiguous {
        id: id.to_string(),
        summary: entries.iter().map(|b| one_line(b)).collect(),
    })
}

/// Group by id, resolve each group, and rebuild the register if every group
/// resolved. Order is first-seen, matching how the array read off disk.
pub fn dedupe(register: &Register) -> Outcome {
    let mut order: Vec<&str> = Vec::new();
    for bug in &register.bugs {
        if !order.contains(&bug.id.as_str()) {
            order.push(&bug.id);
        }
    }

    let mut kept = Vec::new();
    let mut ambiguous = Vec::new();
    let mut survivors = Vec::new();

    for id in order {
        let entries: Vec<&Bug> = register.bugs.iter().filter(|b| b.id == id).collect();
        if entries.len() == 1 {
            survivors.push(entries[0].clone());
            continue;
        }
        match resolve_group(id, entries) {
            Ok((bug, note)) => {
                kept.push(note);
                survivors.push(bug);
            }
            Err(problem) => ambiguous.push(problem),
        }
    }

    if !ambiguous.is_empty() {
        return Outcome {
            register: None,
            kept,
            ambiguous,
        };
    }

    let mut out = register.clone();
    out.bugs = survivors;
    out.sort();
    Outcome {
        register: Some(out),
        kept,
        ambiguous,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::{Priority, Severity, Status};

    fn bug(id: &str, status: Status, updated_at: &str) -> Bug {
        Bug {
            id: id.to_string(),
            title: "t".to_string(),
            status,
            severity: Severity::Major,
            priority: Priority::Normal,
            parent: None,
            reproduce: "r".to_string(),
            observed: "o".to_string(),
            expected: "e".to_string(),
            mechanism: None,
            surfaces: Vec::new(),
            fix: None,
            verification: None,
            found_by: "test".to_string(),
            notes: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: updated_at.to_string(),
        }
    }

    fn register(bugs: Vec<Bug>) -> Register {
        Register {
            skill: "bug-report".to_string(),
            skill_version: "2.0.0-alpha.1".to_string(),
            comment: "c".to_string(),
            bugs,
        }
    }

    #[test]
    fn a_closed_duplicate_beats_an_open_one_regardless_of_order() {
        let reg = register(vec![
            bug("B1", Status::Confirmed, "2026-01-02T00:00:00Z"),
            bug("B1", Status::Fixed, "2026-01-01T00:00:00Z"),
        ]);
        let outcome = dedupe(&reg);
        let out = outcome.register.expect("resolvable");
        assert_eq!(out.bugs.len(), 1);
        assert_eq!(out.bugs[0].status, Status::Fixed);
        assert!(outcome.ambiguous.is_empty());
    }

    #[test]
    fn two_entries_with_different_closed_statuses_are_refused() {
        let reg = register(vec![
            bug("B1", Status::Fixed, "2026-01-01T00:00:00Z"),
            bug("B1", Status::WontFix, "2026-01-02T00:00:00Z"),
        ]);
        let outcome = dedupe(&reg);
        assert!(outcome.register.is_none());
        assert_eq!(outcome.ambiguous.len(), 1);
        assert_eq!(outcome.ambiguous[0].id, "B1");
    }

    #[test]
    fn identical_duplicates_collapse_without_a_note_about_status() {
        let reg = register(vec![
            bug("B1", Status::Confirmed, "2026-01-01T00:00:00Z"),
            bug("B1", Status::Confirmed, "2026-01-01T00:00:00Z"),
        ]);
        let outcome = dedupe(&reg);
        let out = outcome.register.expect("resolvable");
        assert_eq!(out.bugs.len(), 1);
    }

    #[test]
    fn a_non_duplicate_entry_passes_through_untouched() {
        let reg = register(vec![bug("B1", Status::Reported, "2026-01-01T00:00:00Z")]);
        let outcome = dedupe(&reg);
        let out = outcome.register.expect("resolvable");
        assert_eq!(out.bugs.len(), 1);
        assert!(outcome.kept.is_empty());
    }
}
