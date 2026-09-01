// MODE: DEV
// PACKAGE: PROD
//! The defect register as types.
//!
//! The rules live in the type system rather than in a validation pass, which is
//! the whole reason this is not a shell script any more: an out-of-vocabulary
//! severity is a parse error naming the field, not a string comparison someone
//! has to remember to run. The register that prompted this carried
//! `"severity": "critical"` past every writer because the check was a separate
//! step that the writing path had bypassed.
//!
//! Field order in `Bug` is the ON-DISK order, and serde serialises a struct in
//! declaration order, so a read-modify-write reproduces the file byte for byte
//! rather than reordering keys under the author. Do not reorder these fields to
//! taste; the register is a tracked file and a reordering is a diff on every
//! entry.

use serde::{Deserialize, Serialize};

/// Refuses an unknown key rather than dropping it. A typed reader that ignores
/// what it does not understand silently deletes a field a newer writer added,
/// and the loss only shows up in a diff nobody reads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Register {
    pub skill: String,
    pub skill_version: String,
    pub comment: String,
    pub bugs: Vec<Bug>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bug {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub severity: Severity,
    pub priority: Priority,
    pub parent: Option<String>,
    pub reproduce: String,
    pub observed: String,
    pub expected: String,
    pub mechanism: Option<String>,
    pub surfaces: Vec<String>,
    pub fix: Option<String>,
    pub verification: Option<String>,
    pub found_by: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Reported,
    Confirmed,
    Fixed,
    NotADefect,
    WontFix,
    Obsolete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Blocking,
    Major,
    Minor,
    Cosmetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    Urgent,
    High,
    Normal,
    Low,
    Someday,
}

impl Status {
    /// Still needing work. Drives the report and nothing else, so a status the
    /// register calls closed never appears as outstanding.
    pub fn is_open(self) -> bool {
        matches!(self, Status::Reported | Status::Confirmed)
    }

    /// A closure that has to carry evidence, and which kind.
    pub fn closure_evidence(self) -> Option<Evidence> {
        match self {
            Status::Fixed => Some(Evidence::FixAndVerification),
            Status::WontFix | Status::NotADefect | Status::Obsolete => Some(Evidence::Reason),
            Status::Reported | Status::Confirmed => None,
        }
    }
}

/// What a given closure has to be accompanied by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    /// What changed, and how it is proven.
    FixAndVerification,
    /// Why it is being dismissed, for the record.
    Reason,
}

impl Severity {
    /// Worst first, for the register's stored order.
    pub fn rank(self) -> u8 {
        match self {
            Severity::Blocking => 0,
            Severity::Major => 1,
            Severity::Minor => 2,
            Severity::Cosmetic => 3,
        }
    }
}

impl Priority {
    pub fn rank(self) -> u8 {
        match self {
            Priority::Urgent => 0,
            Priority::High => 1,
            Priority::Normal => 2,
            Priority::Low => 3,
            Priority::Someday => 4,
        }
    }
}

/// The numeric part of an id, for sorting. `B12a` sorts with 12: ids carry
/// letter suffixes for entries filed against an existing one, and a string
/// comparison would put B10 before B9.
/// The accepted spellings, for a usage message. Beside the enums they describe,
/// because that is the one place a new variant is added: a vocabulary in the
/// argument parser would be a second list to forget.
pub const STATUSES: &[&str] = &[
    "reported",
    "confirmed",
    "fixed",
    "not-a-defect",
    "wont-fix",
    "obsolete",
];
pub const SEVERITIES: &[&str] = &["blocking", "major", "minor", "cosmetic"];
pub const PRIORITIES: &[&str] = &["urgent", "high", "normal", "low", "someday"];

pub fn id_number(id: &str) -> u64 {
    let digits: String = id
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(u64::MAX)
}

impl Register {
    /// Worst first: priority, then severity, then the id's number. The stored
    /// order IS the reading order, so a report never imposes a second opinion
    /// on urgency.
    pub fn sort(&mut self) {
        self.bugs.sort_by_key(|b| {
            (
                b.priority.rank(),
                b.severity.rank(),
                id_number(&b.id),
                b.id.clone(),
            )
        });
    }

    /// The next free `B` number, from the register's own high-water mark. A
    /// suggestion: any unused id is acceptable, which is why suffixed ids exist.
    pub fn next_id(&self) -> u64 {
        self.bugs
            .iter()
            .map(|b| id_number(&b.id))
            .filter(|n| *n != u64::MAX)
            .max()
            .map_or(1, |n| n + 1)
    }

    pub fn find(&self, id: &str) -> Option<&Bug> {
        self.bugs.iter().find(|b| b.id == id)
    }

    /// Every rule the types cannot express: uniqueness, referential integrity
    /// between parent and id, and the evidence a closure owes. Empty means
    /// sound.
    ///
    /// The enums already made an unknown status or severity unrepresentable, so
    /// what is left here is exactly the set of rules that need more than one
    /// field to check.
    pub fn findings(&self) -> Vec<String> {
        let mut out = Vec::new();

        let mut seen: Vec<&str> = Vec::new();
        let mut duplicates: Vec<&str> = Vec::new();
        for bug in &self.bugs {
            if seen.contains(&bug.id.as_str()) {
                if !duplicates.contains(&bug.id.as_str()) {
                    duplicates.push(&bug.id);
                }
            } else {
                seen.push(&bug.id);
            }
        }
        if !duplicates.is_empty() {
            out.push(format!("duplicate ids: {}", duplicates.join(" ")));
        }

        for bug in &self.bugs {
            if let Some(parent) = bug.parent.as_deref() {
                if !parent.is_empty() && !seen.contains(&parent) {
                    out.push(format!("{}: parent {} does not exist", bug.id, parent));
                }
            }
            if bug.reproduce.trim().is_empty() {
                out.push(format!(
                    "{}: no reproduction — a report without one is a rumour",
                    bug.id
                ));
            }
            if bug.created_at.trim().is_empty() {
                out.push(format!("{}: missing created_at", bug.id));
            }
            if bug.updated_at.trim().is_empty() {
                out.push(format!("{}: missing updated_at", bug.id));
            }
            if bug.status == Status::Confirmed && blank(&bug.mechanism) {
                out.push(format!("{}: confirmed without a mechanism", bug.id));
            }
            if bug.status == Status::Fixed && blank(&bug.verification) {
                out.push(format!("{}: fixed without verification", bug.id));
            }
        }
        out
    }
}

fn blank(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|v| v.trim().is_empty())
}
