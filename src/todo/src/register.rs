// MODE: DEV
// PACKAGE: PROD
//! The work queue as types.
//!
//! These are the QUEUE's rules and they are not the defect register's. The two
//! were one shared library while both were shell, and that library had to
//! pretend they agreed. They do not:
//!
//! * a task has no `severity` at all — a queued item is not more or less broken;
//! * the status vocabulary is different, and includes `dropped` and `decided`,
//!   which have no meaning for a defect;
//! * `parent` is load-bearing here rather than occasional: the queue is a tree,
//!   and most of its entries hang off another;
//! * `blocked_on` is free text that SOMETIMES holds an id and sometimes prose,
//!   so only an exact id match may be treated as a reference;
//! * closing needs a `note` carrying the evidence, not a fix and a verification.
//!
//! `dropped` is in the vocabulary here because the shipped schema lists it. The
//! shell writers omitted it, so a status the schema documented could not be set
//! (B102) — a new tool inheriting that would have been inheriting a bug.
//!
//! Field order in `Task` is the ON-DISK order, and serde serialises a struct in
//! declaration order, so a read-modify-write reproduces the file byte for byte.
//! Do not reorder these to taste: the register is tracked, and a reordering is a
//! diff on every entry.

use serde::{Deserialize, Serialize};

/// Refuses an unknown key rather than dropping it, so a field a newer writer
/// added is not silently deleted by an older reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Register {
    pub skill: String,
    pub skill_version: String,
    pub comment: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub priority: Priority,
    pub parent: Option<String>,
    pub detail: String,
    pub blocked_on: Option<String>,
    pub refs: Vec<String>,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Open,
    Partly,
    Blocked,
    Decided,
    Done,
    Dropped,
    Obsolete,
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
    /// Still to do. `decided` counts as live: a decision recorded but not acted
    /// on is still work, which is why it is not lumped with `done`.
    pub fn is_open(self) -> bool {
        matches!(
            self,
            Status::Open | Status::Partly | Status::Blocked | Status::Decided
        )
    }

    /// A closure that owes evidence in `note`. Every closed item carries what
    /// happened, or the queue becomes a list of things somebody says are done.
    pub fn needs_evidence(self) -> bool {
        matches!(self, Status::Done | Status::Dropped | Status::Obsolete)
    }

    /// Queue order: what to look at first.
    pub fn rank(self) -> u8 {
        match self {
            Status::Open => 0,
            Status::Blocked => 1,
            Status::Partly => 2,
            Status::Decided => 3,
            Status::Done => 4,
            Status::Dropped => 5,
            Status::Obsolete => 6,
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

/// The accepted spellings, for a usage message. Beside the enums they describe,
/// because that is the one place a new variant is added: a vocabulary in the
/// argument parser would be a second list to forget.
///
/// There is no SEVERITIES here and there is not meant to be. A queued task is
/// not more or less broken, and the shared shell library's pretence that both
/// registers had the same fields is what this split exists to end.
pub const STATUSES: &[&str] = &[
    "open", "partly", "blocked", "decided", "done", "dropped", "obsolete",
];
pub const PRIORITIES: &[&str] = &["urgent", "high", "normal", "low", "someday"];

/// The numeric part of an id. `T41a` sorts with 41: sub-task ids carry letter
/// suffixes, because `todo add --id` takes the id from the caller and the
/// register only suggests the next number. 28 of the ids in the live queue are
/// of that shape, so an id-shape rule would be wrong, not merely strict.
pub fn id_number(id: &str) -> u64 {
    let digits: String = id
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(u64::MAX)
}

impl Register {
    /// Status rank, then priority, then the id's number. Status leads here where
    /// severity leads in the defect register: the queue's question is "what is
    /// still to do", so a done item never sits above an open one.
    pub fn sort(&mut self) {
        self.tasks.sort_by_key(|t| {
            (
                t.status.rank(),
                t.priority.rank(),
                id_number(&t.id),
                t.id.clone(),
            )
        });
    }

    pub fn next_id(&self) -> u64 {
        self.tasks
            .iter()
            .map(|t| id_number(&t.id))
            .filter(|n| *n != u64::MAX)
            .max()
            .map_or(1, |n| n + 1)
    }

    pub fn find(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// Every rule the types cannot express. The enums already made an unknown
    /// status or priority unrepresentable, so what is left needs more than one
    /// field to decide.
    pub fn findings(&self) -> Vec<String> {
        let mut out = Vec::new();

        let mut seen: Vec<&str> = Vec::new();
        let mut duplicates: Vec<&str> = Vec::new();
        for task in &self.tasks {
            if seen.contains(&task.id.as_str()) {
                if !duplicates.contains(&task.id.as_str()) {
                    duplicates.push(&task.id);
                }
            } else {
                seen.push(&task.id);
            }
        }
        if !duplicates.is_empty() {
            out.push(format!("duplicate ids: {}", duplicates.join(" ")));
        }

        for task in &self.tasks {
            if let Some(parent) = task.parent.as_deref() {
                if !parent.is_empty() && !seen.contains(&parent) {
                    out.push(format!("{}: parent {} does not exist", task.id, parent));
                }
            }
            if task.created_at.trim().is_empty() {
                out.push(format!("{}: missing created_at", task.id));
            }
            if task.updated_at.trim().is_empty() {
                out.push(format!("{}: missing updated_at", task.id));
            }
            if task.status.needs_evidence() && blank(&task.note) {
                out.push(format!(
                    "{}: {} with nothing in note — every closed item carries what happened",
                    task.id,
                    token(task.status)
                ));
            }
        }
        out
    }
}

/// The on-disk spelling, derived from serde so output and input share one
/// vocabulary rather than two lists that drift.
pub fn token<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "-".into())
}

fn blank(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|v| v.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropped_is_a_settable_status() {
        // B102: the shipped schema lists `dropped` and the shell writers refused
        // it, so the value the schema documented could not be set. Parsing it is
        // the assertion that this tool did not inherit that.
        let status: Status = serde_json::from_str("\"dropped\"").expect("dropped parses");
        assert_eq!(status, Status::Dropped);
        assert!(status.needs_evidence());
        assert!(!status.is_open());
    }

    #[test]
    fn decided_is_still_open_work() {
        assert!(Status::Decided.is_open());
        assert!(!Status::Decided.needs_evidence());
    }

    #[test]
    fn a_suffixed_subtask_id_sorts_with_its_number() {
        assert_eq!(id_number("T41a"), 41);
        assert_eq!(id_number("T1e"), 1);
        assert_eq!(id_number("T70"), 70);
    }

    #[test]
    fn tokens_match_the_on_disk_spelling() {
        assert_eq!(token(Status::Dropped), "dropped");
        assert_eq!(token(Status::Partly), "partly");
        assert_eq!(token(Priority::Someday), "someday");
    }
}
