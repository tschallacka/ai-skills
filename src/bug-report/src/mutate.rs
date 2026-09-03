// MODE: DEV
// PACKAGE: PROD
//! Filing and closing defects.
//!
//! Both writers build a candidate register, check it, and only then hand it back
//! to be written. The order is deliberate and it is the one thing the shell
//! version got wrong: `bug-update.sh` moved the new file into place and
//! validated afterwards, so a refused update had already overwritten the
//! register and the caller was told to run the repair tool (B109). Here the
//! caller gets the candidate or an error, never a half-written file.

use crate::clock;
use crate::register::{Bug, Evidence, Priority, Register, Severity, Status};

/// What `add` needs. Every field that has no sensible default is required, and
/// the requirement is stated where it is read rather than defaulted quietly:
/// a defect with no reproduction is a rumour, and one with no expectation is an
/// opinion.
pub struct NewBug {
    pub title: String,
    pub reproduce: String,
    pub observed: String,
    pub expected: String,
    pub severity: Severity,
    pub priority: Priority,
    pub status: Status,
    pub mechanism: Option<String>,
    pub parent: Option<String>,
    pub found_by: String,
    pub surfaces: Vec<String>,
}

/// Add one entry, returning the id it was given.
///
/// The id comes from the register's own high-water mark. It is a suggestion in
/// the sense that any unused id would do, but this is the only writer, so it
/// allocates rather than asking.
pub fn add(register: &mut Register, new: NewBug) -> Result<String, Vec<String>> {
    let now = clock::now();
    let id = format!("B{}", register.next_id());

    register.bugs.push(Bug {
        id: id.clone(),
        title: new.title,
        status: new.status,
        severity: new.severity,
        priority: new.priority,
        parent: new.parent,
        reproduce: new.reproduce,
        observed: new.observed,
        expected: new.expected,
        mechanism: new.mechanism,
        surfaces: new.surfaces,
        fix: None,
        verification: None,
        found_by: new.found_by,
        notes: None,
        created_at: now.clone(),
        updated_at: now,
    });

    let findings = register.findings();
    if !findings.is_empty() {
        // The caller discards the register rather than writing it, so the entry
        // never reaches the file. Popping it back off would also work and would
        // be a worse habit: it invites someone to "recover" and continue.
        return Err(findings);
    }
    register.sort();
    Ok(id)
}

/// What `update` may change. All optional: an update names only what moves.
#[derive(Default)]
pub struct Change {
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    pub fix: Option<String>,
    pub verification: Option<String>,
    pub mechanism: Option<String>,
    pub reason: Option<String>,
    pub append_note: Option<String>,
}

impl Change {
    pub fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.priority.is_none()
            && self.fix.is_none()
            && self.verification.is_none()
            && self.mechanism.is_none()
            && self.reason.is_none()
            && self.append_note.is_none()
    }

    /// The evidence a status change owes, checked before anything is built so
    /// the message is about the request rather than about the result.
    ///
    /// This is the register's whole point: a closure nobody can check is a
    /// guess, and a dismissal with no stated reason is one too.
    pub fn missing_evidence(&self) -> Option<String> {
        let status = self.status?;
        match status.closure_evidence()? {
            Evidence::FixAndVerification => {
                if self.fix.is_none() {
                    Some("--status fixed requires --fix (what changed)".into())
                } else if self.verification.is_none() {
                    Some(
                        "--status fixed requires --verification (how it is proven, \
                         including the mutation that made the test fail)"
                            .into(),
                    )
                } else {
                    None
                }
            }
            Evidence::Reason => {
                if self.reason.is_none() {
                    Some(format!(
                        "--status {} requires --reason, so the dismissal is on the record",
                        serde_json::to_value(status)
                            .ok()
                            .and_then(|v| v.as_str().map(str::to_string))
                            .unwrap_or_default()
                    ))
                } else {
                    None
                }
            }
        }
    }
}

pub enum UpdateError {
    NoSuchEntry,
    Unsound(Vec<String>),
}

/// Apply a change to one entry. `Err` leaves the register untouched.
pub fn update(register: &mut Register, id: &str, change: Change) -> Result<(), UpdateError> {
    let now = clock::now();
    let Some(bug) = register.bugs.iter_mut().find(|b| b.id == id) else {
        return Err(UpdateError::NoSuchEntry);
    };

    bug.updated_at = now;
    if let Some(status) = change.status {
        bug.status = status;
    }
    if let Some(priority) = change.priority {
        bug.priority = priority;
    }
    if let Some(fix) = change.fix {
        bug.fix = Some(fix);
    }
    if let Some(verification) = change.verification {
        bug.verification = Some(verification);
    }
    if let Some(mechanism) = change.mechanism {
        bug.mechanism = Some(mechanism);
    }
    // A reason and an appended note both land in `notes`, appended rather than
    // replacing: the note is a running record, and a dismissal that overwrote
    // an earlier finding would erase why the entry was opened.
    for addition in [change.reason, change.append_note].into_iter().flatten() {
        bug.notes = Some(match bug.notes.as_deref() {
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
