// MODE: DEV
// PACKAGE: PROD
//! Reading the register.
//!
//! Reading is a helper for the same reason writing is. When every caller wrote
//! its own query, callers got it wrong: one read the wrong array and reported a
//! register as nearly empty while live entries sat in the other, and a
//! hand-rolled next-id crashed on an id with a letter suffix. The queries live
//! here, where tests hold them.

use crate::register::{Bug, Priority, Register, Severity, Status};

/// An empty filter matches everything, rather than matching the empty string.
/// "Absent" has to mean unfiltered, or `list --status ""` silently returns
/// nothing and reads as an empty register.
#[derive(Default)]
pub struct Filter {
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    pub severity: Option<Severity>,
    pub parent: Option<String>,
    pub surface: Option<String>,
    pub since: Option<String>,
}

impl Filter {
    pub fn matches(&self, bug: &Bug) -> bool {
        if let Some(status) = self.status {
            if bug.status != status {
                return false;
            }
        }
        if let Some(priority) = self.priority {
            if bug.priority != priority {
                return false;
            }
        }
        if let Some(severity) = self.severity {
            if bug.severity != severity {
                return false;
            }
        }
        if let Some(parent) = self.parent.as_deref() {
            if bug.parent.as_deref() != Some(parent) {
                return false;
            }
        }
        if let Some(surface) = self.surface.as_deref() {
            if !bug.surfaces.iter().any(|s| s.contains(surface)) {
                return false;
            }
        }
        if let Some(since) = self.since.as_deref() {
            // Lexicographic works because the timestamps are fixed-width UTC.
            // That is a property clock.rs guarantees, not a coincidence.
            let stamp = if bug.updated_at.is_empty() {
                bug.created_at.as_str()
            } else {
                bug.updated_at.as_str()
            };
            if stamp < since {
                return false;
            }
        }
        true
    }
}

pub fn select<'a>(register: &'a Register, filter: &Filter) -> Vec<&'a Bug> {
    register
        .bugs
        .iter()
        .filter(|bug| filter.matches(bug))
        .collect()
}

/// One tab-separated row per entry, so a caller can cut fields out of it
/// without parsing prose. Titles come last, being the only field that can
/// contain spaces at length.
pub fn list(register: &Register, filter: &Filter) -> String {
    let mut out = String::new();
    for bug in select(register, filter) {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            bug.id,
            token(bug.status),
            token(bug.priority),
            token(bug.severity),
            bug.title
        ));
    }
    out
}

/// What is still outstanding, in the register's own stored order — worst first.
/// A report that re-sorted would be imposing a second opinion on urgency over
/// the one the register already records.
pub fn report(register: &Register, filter: &Filter) -> String {
    let open: Vec<&Bug> = register
        .bugs
        .iter()
        .filter(|bug| bug.status.is_open())
        .filter(|bug| filter.matches(bug))
        .collect();

    let mut out = format!("{} open of {} total\n\n", open.len(), register.bugs.len());
    for bug in open {
        out.push_str(&format!(
            "{}  {}/{}  {}\n",
            bug.id,
            token(bug.priority),
            token(bug.severity),
            bug.title
        ));
        if !bug.surfaces.is_empty() {
            out.push_str(&format!("      surfaces: {}\n", bug.surfaces.join(", ")));
        }
    }
    out
}

/// The glyph a status reads as. These are the register's public face — the
/// output a user is shown — so they are part of the skill's contract rather
/// than decoration, and they must not drift.
fn glyph(status: Status) -> &'static str {
    match status {
        Status::Reported => "💤",
        Status::Confirmed => "⛔",
        Status::Fixed => "✅",
        Status::NotADefect => "✔️",
        Status::WontFix | Status::Obsolete => "🚫",
    }
}

/// The whole register as a tree, worst first, children indented under their
/// parent.
///
/// Priority then severity then id, and the pairing is deliberate: a reader needs
/// both to judge an entry, and showing them together is what stops a blocking
/// defect nobody can reach outranking a cosmetic one on every screen.
pub fn tree(register: &Register) -> String {
    let mut out = String::new();
    render(register, None, 0, &mut out);
    out
}

fn render(register: &Register, parent: Option<&str>, depth: usize, out: &mut String) {
    let mut children: Vec<&Bug> = register
        .bugs
        .iter()
        .filter(|bug| bug.parent.as_deref() == parent)
        .collect();
    children.sort_by_key(|bug| {
        (
            bug.priority.rank(),
            bug.severity.rank(),
            crate::register::id_number(&bug.id),
            bug.id.clone(),
        )
    });
    for bug in children {
        out.push_str(&format!(
            "{}{} {}  [{}/{}]  {}\n",
            "  ".repeat(depth),
            glyph(bug.status),
            bug.id,
            token(bug.priority),
            token(bug.severity),
            bug.title
        ));
        render(register, Some(&bug.id), depth + 1, out);
    }
}

/// The on-disk spelling of an enum, so output and input use one vocabulary.
/// Derived from the serde representation rather than a second match statement,
/// which is what keeps them from drifting apart.
fn token<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "-".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::{Priority, Severity, Status};

    fn bug(id: &str, status: Status, severity: Severity, priority: Priority) -> Bug {
        Bug {
            id: id.into(),
            title: format!("title of {id}"),
            status,
            severity,
            priority,
            parent: None,
            reproduce: "r".into(),
            observed: "o".into(),
            expected: "e".into(),
            mechanism: Some("m".into()),
            surfaces: vec!["a/b.sh".into()],
            fix: None,
            verification: None,
            found_by: "test".into(),
            notes: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-02T00:00:00Z".into(),
        }
    }

    fn register(bugs: Vec<Bug>) -> Register {
        Register {
            skill: "bug-report".into(),
            skill_version: crate::migrate::SUPPORTED.into(),
            comment: "fixture".into(),
            bugs,
        }
    }

    #[test]
    fn an_absent_filter_matches_everything() {
        let reg = register(vec![
            bug("B1", Status::Reported, Severity::Major, Priority::Normal),
            bug("B2", Status::Fixed, Severity::Minor, Priority::Low),
        ]);
        assert_eq!(select(&reg, &Filter::default()).len(), 2);
    }

    #[test]
    fn a_status_filter_selects_only_that_status() {
        let reg = register(vec![
            bug("B1", Status::Reported, Severity::Major, Priority::Normal),
            bug("B2", Status::Fixed, Severity::Minor, Priority::Low),
        ]);
        let filter = Filter {
            status: Some(Status::Fixed),
            ..Default::default()
        };
        let hits = select(&reg, &filter);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "B2");
    }

    #[test]
    fn report_counts_only_open_entries_but_totals_everything() {
        let reg = register(vec![
            bug("B1", Status::Reported, Severity::Major, Priority::Normal),
            bug("B2", Status::Confirmed, Severity::Minor, Priority::Low),
            bug("B3", Status::Fixed, Severity::Minor, Priority::Low),
            bug("B4", Status::WontFix, Severity::Cosmetic, Priority::Someday),
        ]);
        assert!(report(&reg, &Filter::default()).starts_with("2 open of 4 total"));
    }

    #[test]
    fn since_excludes_older_entries_lexicographically() {
        let mut older = bug("B1", Status::Reported, Severity::Major, Priority::Normal);
        older.updated_at = "2025-06-01T00:00:00Z".into();
        let reg = register(vec![
            older,
            bug("B2", Status::Reported, Severity::Major, Priority::Normal),
        ]);
        let filter = Filter {
            since: Some("2026-01-01T00:00:00Z".into()),
            ..Default::default()
        };
        let hits = select(&reg, &filter);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "B2");
    }

    #[test]
    fn tokens_match_the_on_disk_spelling() {
        // The kebab-case ones are the reason this is derived from serde rather
        // than written twice.
        assert_eq!(token(Status::NotADefect), "not-a-defect");
        assert_eq!(token(Status::WontFix), "wont-fix");
        assert_eq!(token(Severity::Blocking), "blocking");
        assert_eq!(token(Priority::Someday), "someday");
    }
}
