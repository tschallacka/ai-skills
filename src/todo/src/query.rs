// MODE: DEV
// PACKAGE: PROD
//! Reading the queue.
//!
//! The queue's reports answer a different question from the defect register's.
//! There is no severity to pair with priority, so the useful second column is
//! what is holding a task up: a blocked item with nothing in `blocked_on` is the
//! one that stalls a working session, and it should be visible without asking.

use crate::register::{token, Priority, Register, Status, Task};

/// An empty filter matches everything, rather than matching the empty string.
/// "Absent" has to mean unfiltered, or `list --status ""` returns nothing and
/// reads as an empty queue.
#[derive(Default)]
pub struct Filter {
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    pub parent: Option<String>,
    /// Substring against `refs`, which hold paths rather than ids, so a caller
    /// can ask what is queued against a file.
    pub touching: Option<String>,
    pub since: Option<String>,
}

impl Filter {
    pub fn matches(&self, task: &Task) -> bool {
        if let Some(status) = self.status {
            if task.status != status {
                return false;
            }
        }
        if let Some(priority) = self.priority {
            if task.priority != priority {
                return false;
            }
        }
        if let Some(parent) = self.parent.as_deref() {
            if task.parent.as_deref() != Some(parent) {
                return false;
            }
        }
        if let Some(needle) = self.touching.as_deref() {
            if !task.refs.iter().any(|r| r.contains(needle)) {
                return false;
            }
        }
        if let Some(since) = self.since.as_deref() {
            // Lexicographic works because the timestamps are fixed-width UTC —
            // a property clock.rs guarantees, not a coincidence.
            let stamp = if task.updated_at.is_empty() {
                task.created_at.as_str()
            } else {
                task.updated_at.as_str()
            };
            if stamp < since {
                return false;
            }
        }
        true
    }
}

pub fn select<'a>(register: &'a Register, filter: &Filter) -> Vec<&'a Task> {
    register
        .tasks
        .iter()
        .filter(|task| filter.matches(task))
        .collect()
}

/// One tab-separated row per task, so a caller can cut fields out without
/// parsing prose. The title comes last, being the field that runs long.
pub fn list(register: &Register, filter: &Filter) -> String {
    let mut out = String::new();
    for task in select(register, filter) {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            task.id,
            token(task.status),
            token(task.priority),
            task.title
        ));
    }
    out
}

/// What is still to do, in the register's own stored order. Blocked tasks name
/// what they are waiting on, because that is the line a reader has to act on
/// before the task can move.
pub fn report(register: &Register, filter: &Filter) -> String {
    let open: Vec<&Task> = register
        .tasks
        .iter()
        .filter(|task| task.status.is_open())
        .filter(|task| filter.matches(task))
        .collect();

    let mut out = format!("{} open of {} total\n\n", open.len(), register.tasks.len());
    for task in open {
        out.push_str(&format!(
            "{}  {}/{}  {}\n",
            task.id,
            token(task.status),
            token(task.priority),
            task.title
        ));
        if task.status == Status::Blocked {
            out.push_str(&format!(
                "      blocked on: {}\n",
                task.blocked_on
                    .as_deref()
                    .filter(|b| !b.trim().is_empty())
                    // A blocked task with nothing recorded is the one that
                    // stalls a session, so it is called out rather than shown
                    // as a blank.
                    .unwrap_or("NOTHING RECORDED — find out what, or reopen it")
            ));
        }
    }
    out
}

/// The glyph a status reads as. These are the skill's public face — the output a
/// user is shown — so they are part of its contract rather than decoration, and
/// they must not drift.
fn glyph(status: Status) -> &'static str {
    match status {
        Status::Open => "⬜",
        Status::Partly => "🔨",
        Status::Blocked => "⛔",
        Status::Decided => "📌",
        Status::Done => "✅",
        Status::Dropped | Status::Obsolete => "🚫",
    }
}

/// The whole queue as a tree, children indented under their parent.
///
/// The queue is a tree in a way the defect register is not — most entries hang
/// off another — so this is its primary view rather than an extra one.
pub fn tree(register: &Register) -> String {
    let mut out = String::new();
    render(register, None, 0, &mut out);
    // An entry whose parent does not exist would otherwise be rendered nowhere.
    // `check` reports that as a finding; the tree must still show the task, or a
    // reader believes the queue is smaller than it is.
    let ids: Vec<&str> = register.tasks.iter().map(|t| t.id.as_str()).collect();
    let orphans: Vec<&Task> = register
        .tasks
        .iter()
        .filter(|t| {
            t.parent
                .as_deref()
                .is_some_and(|p| !p.is_empty() && !ids.contains(&p))
        })
        .collect();
    for task in orphans {
        out.push_str(&format!(
            "{} {}  [{}]  {}  (parent {} is missing)\n",
            glyph(task.status),
            task.id,
            token(task.priority),
            task.title,
            task.parent.as_deref().unwrap_or("?")
        ));
    }
    out
}

fn render(register: &Register, parent: Option<&str>, depth: usize, out: &mut String) {
    let mut children: Vec<&Task> = register
        .tasks
        .iter()
        .filter(|task| match (task.parent.as_deref(), parent) {
            // An empty string and absent both mean "top level" on disk.
            (None, None) | (Some(""), None) => true,
            (Some(p), Some(q)) => p == q,
            _ => false,
        })
        .collect();
    children.sort_by_key(|task| {
        (
            task.status.rank(),
            task.priority.rank(),
            crate::register::id_number(&task.id),
            task.id.clone(),
        )
    });
    for task in children {
        out.push_str(&format!(
            "{}{} {}  [{}]  {}\n",
            "  ".repeat(depth),
            glyph(task.status),
            task.id,
            token(task.priority),
            task.title
        ));
        render(register, Some(&task.id), depth + 1, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, status: Status, priority: Priority) -> Task {
        Task {
            id: id.into(),
            title: format!("title of {id}"),
            status,
            priority,
            parent: None,
            detail: "d".into(),
            blocked_on: None,
            refs: vec!["a/b.sh".into()],
            note: if status.needs_evidence() {
                Some("closed in test".into())
            } else {
                None
            },
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-02T00:00:00Z".into(),
        }
    }

    fn register(tasks: Vec<Task>) -> Register {
        Register {
            skill: "todo".into(),
            skill_version: crate::migrate::SUPPORTED.into(),
            comment: "fixture".into(),
            tasks,
        }
    }

    #[test]
    fn an_absent_filter_matches_everything() {
        let reg = register(vec![
            task("T1", Status::Open, Priority::Normal),
            task("T2", Status::Done, Priority::Low),
        ]);
        assert_eq!(select(&reg, &Filter::default()).len(), 2);
    }

    #[test]
    fn decided_counts_as_open_in_the_report() {
        let reg = register(vec![
            task("T1", Status::Open, Priority::Normal),
            task("T2", Status::Decided, Priority::Normal),
            task("T3", Status::Done, Priority::Low),
            task("T4", Status::Dropped, Priority::Low),
        ]);
        assert!(report(&reg, &Filter::default()).starts_with("2 open of 4 total"));
    }

    #[test]
    fn a_blocked_task_with_no_reason_says_so() {
        let reg = register(vec![task("T1", Status::Blocked, Priority::High)]);
        assert!(report(&reg, &Filter::default()).contains("NOTHING RECORDED"));
    }

    #[test]
    fn touching_filters_on_refs() {
        let mut other = task("T2", Status::Open, Priority::Normal);
        other.refs = vec!["installer/src/50-manifest.sh".into()];
        let reg = register(vec![task("T1", Status::Open, Priority::Normal), other]);
        let filter = Filter {
            touching: Some("installer/".into()),
            ..Default::default()
        };
        let hits = select(&reg, &filter);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "T2");
    }

    #[test]
    fn an_orphan_is_still_shown_by_the_tree() {
        let mut orphan = task("T2", Status::Open, Priority::Normal);
        orphan.parent = Some("T99".into());
        let reg = register(vec![task("T1", Status::Open, Priority::Normal), orphan]);
        let rendered = tree(&reg);
        assert!(rendered.contains("T1"));
        assert!(rendered.contains("T2"));
        assert!(rendered.contains("parent T99 is missing"));
    }

    #[test]
    fn since_excludes_older_tasks_lexicographically() {
        let mut older = task("T1", Status::Open, Priority::Normal);
        older.updated_at = "2025-06-01T00:00:00Z".into();
        let reg = register(vec![older, task("T2", Status::Open, Priority::Normal)]);
        let filter = Filter {
            since: Some("2026-01-01T00:00:00Z".into()),
            ..Default::default()
        };
        let hits = select(&reg, &filter);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "T2");
    }
}
