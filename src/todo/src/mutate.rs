// MODE: DEV
// PACKAGE: PROD
//! Adding and closing tasks.
//!
//! Both writers build a candidate, check it, and only then let the caller write.
//! The order matters: the shell version of the defect writer moved the file into
//! place and validated afterwards, so a refused update had already overwritten
//! the register (B109). Nothing here can leave a half-written file.

use crate::clock;
use crate::register::{Priority, Register, Status, Task};

pub struct NewTask {
    /// The caller supplies the id, because sub-tasks take suffixed ones
    /// (`T41a`) that no allocator would produce. `None` takes the next number.
    pub id: Option<String>,
    pub title: String,
    pub status: Status,
    pub priority: Priority,
    pub parent: Option<String>,
    pub detail: String,
    pub blocked_on: Option<String>,
    pub refs: Vec<String>,
    pub note: Option<String>,
}

/// A closure owes evidence in `note`, whether it arrives on `add` or on
/// `update`. Free-standing rather than a method, because both callers ask it of
/// a status and a note they hold separately.
pub fn missing_evidence(status: Status, note: Option<&str>) -> Option<String> {
    if !status.needs_evidence() {
        return None;
    }
    if note.is_some_and(|n| !n.trim().is_empty()) {
        return None;
    }
    Some(format!(
        "--status {} requires --note (what happened), so a closed task carries its evidence",
        crate::register::token(status)
    ))
}

pub fn add(register: &mut Register, new: NewTask) -> Result<String, Vec<String>> {
    let now = clock::now();
    let id = new.id.unwrap_or_else(|| format!("T{}", register.next_id()));

    register.tasks.push(Task {
        id: id.clone(),
        title: new.title,
        status: new.status,
        priority: new.priority,
        parent: new.parent,
        detail: new.detail,
        blocked_on: new.blocked_on,
        refs: new.refs,
        note: new.note,
        created_at: now.clone(),
        updated_at: now,
    });

    let findings = register.findings();
    if !findings.is_empty() {
        return Err(findings);
    }
    register.sort();
    Ok(id)
}

#[derive(Default)]
pub struct Change {
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    pub detail: Option<String>,
    pub blocked_on: Option<String>,
    pub note: Option<String>,
    pub append_note: Option<String>,
}

impl Change {
    pub fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.priority.is_none()
            && self.detail.is_none()
            && self.blocked_on.is_none()
            && self.note.is_none()
            && self.append_note.is_none()
    }

    /// Closing owes evidence, checked before anything is built so the message is
    /// about the request rather than about the result.
    ///
    /// Delegated rather than repeated, so `add` and `update` cannot come to
    /// disagree about what counts as evidence — and so a note of nothing but
    /// whitespace is refused in both, which a bare `is_some()` would accept.
    pub fn missing_evidence(&self) -> Option<String> {
        let status = self.status?;
        let note = self.note.as_deref().or(self.append_note.as_deref());
        missing_evidence(status, note)
    }
}

#[derive(Debug)]
pub enum UpdateError {
    NoSuchEntry,
    Unsound(Vec<String>),
}

pub fn update(register: &mut Register, id: &str, change: Change) -> Result<(), UpdateError> {
    let now = clock::now();
    let Some(task) = register.tasks.iter_mut().find(|t| t.id == id) else {
        return Err(UpdateError::NoSuchEntry);
    };

    task.updated_at = now;
    if let Some(status) = change.status {
        task.status = status;
    }
    if let Some(priority) = change.priority {
        task.priority = priority;
    }
    if let Some(detail) = change.detail {
        task.detail = detail;
    }
    if let Some(blocked_on) = change.blocked_on {
        task.blocked_on = Some(blocked_on);
    }
    if let Some(note) = change.note {
        task.note = Some(note);
    }
    // Appended rather than replacing: the note is a running record, and an
    // overwrite would erase why the task was opened.
    if let Some(addition) = change.append_note {
        task.note = Some(match task.note.as_deref() {
            Some(existing) if !existing.trim().is_empty() => format!("{existing} {addition}"),
            _ => addition,
        });
    }

    let findings = register.findings();
    if !findings.is_empty() {
        return Err(UpdateError::Unsound(findings));
    }
    register.sort();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::{Priority, Status};

    fn empty() -> Register {
        Register {
            skill: "todo".into(),
            skill_version: crate::migrate::SUPPORTED.into(),
            comment: "fixture".into(),
            tasks: vec![],
        }
    }

    fn new_task(status: Status, note: Option<&str>) -> NewTask {
        NewTask {
            id: None,
            title: "a task".into(),
            status,
            priority: Priority::Normal,
            parent: None,
            detail: "d".into(),
            blocked_on: None,
            refs: vec![],
            note: note.map(str::to_string),
        }
    }

    #[test]
    fn a_note_of_whitespace_is_not_evidence() {
        // A bare is_some() accepted this, which is how a queue fills up with
        // closures nobody can check.
        assert!(missing_evidence(Status::Done, Some("   ")).is_some());
        assert!(missing_evidence(Status::Done, None).is_some());
        assert!(missing_evidence(Status::Done, Some("abc123 — fixed")).is_none());
        assert!(missing_evidence(Status::Open, None).is_none());
    }

    #[test]
    fn add_and_update_agree_about_evidence() {
        let update = Change {
            status: Some(Status::Dropped),
            note: Some(" ".into()),
            ..Default::default()
        };
        assert!(update.missing_evidence().is_some());
        let added = new_task(Status::Dropped, Some(" "));
        assert!(missing_evidence(added.status, added.note.as_deref()).is_some());
    }

    #[test]
    fn an_id_is_taken_from_the_caller_when_given() {
        // Sub-task ids carry letter suffixes, which no allocator would produce.
        let mut register = empty();
        let mut task = new_task(Status::Open, None);
        task.id = Some("T41a".into());
        assert_eq!(add(&mut register, task).expect("added"), "T41a");
        let next = new_task(Status::Open, None);
        assert_eq!(add(&mut register, next).expect("added"), "T42");
    }

    #[test]
    fn a_note_is_appended_rather_than_replacing_what_was_there() {
        let mut register = empty();
        let mut first = new_task(Status::Open, None);
        first.note = Some("opened because X".into());
        let id = add(&mut register, first).expect("added");
        let change = Change {
            append_note: Some("then Y happened".into()),
            ..Default::default()
        };
        update(&mut register, &id, change).expect("updated");
        let note = register.find(&id).unwrap().note.clone().unwrap();
        assert!(note.contains("opened because X"), "{note}");
        assert!(note.contains("then Y happened"), "{note}");
    }

    #[test]
    fn an_update_to_a_missing_task_is_reported_not_invented() {
        let mut register = empty();
        let change = Change {
            priority: Some(Priority::High),
            ..Default::default()
        };
        assert!(matches!(
            update(&mut register, "T99", change),
            Err(UpdateError::NoSuchEntry)
        ));
    }
}
