// MODE: DEV
// PACKAGE: PROD
//! Meeting a queue this version did not write, and shrinking one that grew.
//!
//! There is no compatibility layer here and there will not be one: no table of
//! known versions, no per-version branches, no code that keeps reading an older
//! shape. This repository does not carry backwards-compatible code, and a
//! converter full of version guards is that with a friendlier name.
//!
//! So there is one path, and it makes one attempt:
//!
//! 1. copy the original bytes to a VERSIONED backup, before parsing anything;
//! 2. try each task against the CURRENT types — the ones that fit and are still
//!    open are carried;
//! 3. report everything else, naming the commands to move it by hand.
//!
//! Step 3 is the design and not a fallback. The driver is an agent that can read
//! JSON and call a command, so the honest thing is to hand it the task and the
//! command rather than to grow logic guessing what an older field meant.
//!
//! PRUNING is the same machinery pointed at a current file. A queue is a working
//! list, and 121 entries of which most are closed is a queue nobody reads. So
//! closed tasks are moved OUT — into a timestamped archive beside the register,
//! never deleted — and that is why migration and pruning share this module: both
//! answer "which of these are still work", and both keep what they remove.

use crate::clock;
use crate::register::{Register, Task};

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

/// Where pruned tasks go. Stamped with the day rather than the version: two
/// prunes of the SAME version are the normal case, and each has to keep its own
/// contents.
pub fn archive_path(register_path: &str, day: &str) -> String {
    let stem = register_path.strip_suffix(".json").unwrap_or(register_path);
    format!("{stem}.{day}.archive.json")
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

/// One attempt, task by task, against the current types.
///
/// Carried: still open, PLUS the closed tasks a carried one still depends on.
/// Archived: converts cleanly, is closed, and nothing live needs it, so it stays
/// in the backup rather than padding the live queue. Unconvertible: reported with
/// the parse error, which names the field and is more use than any message this
/// function could invent.
///
/// Carrying open tasks ALONE produces an unsound queue, and did: the queue is a
/// tree, so a closed parent left in the backup orphans its open children, and
/// `check` then fails on the file the migration just wrote. What must come along
/// is decided by the same rule `plan` uses for a prune, so the two cannot
/// disagree about what "still needed" means.
pub fn attempt(value: &serde_json::Value) -> (Vec<Task>, Vec<String>, Vec<Unconvertible>) {
    let mut carried = Vec::new();
    let mut archived = Vec::new();
    let mut unconvertible = Vec::new();

    let entries = value
        .get("tasks")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();

    let mut converted: Vec<Task> = Vec::new();
    for entry in entries {
        let id = entry
            .get("id")
            .and_then(|i| i.as_str())
            .unwrap_or("<no id>")
            .to_string();

        match serde_json::from_value::<Task>(entry) {
            Ok(task) => converted.push(task),
            Err(error) => unconvertible.push(Unconvertible {
                id,
                why: error.to_string(),
            }),
        }
    }

    for task in converted.iter() {
        if needed(&converted, task) {
            carried.push(task.clone());
        } else {
            archived.push(task.id.clone());
        }
    }

    (carried, archived, unconvertible)
}

/// Does the live queue still need this task?
///
/// Open, or closed with something live hanging off it: an ancestor of an open
/// task (transitively — a closed grandparent is as load-bearing as a closed
/// parent), or named EXACTLY in an open task's `blocked_on`. `blocked_on` holds
/// prose as often as an id, so only an exact match is a reference; a sentence
/// that merely contains the id is prose about the task.
fn needed(all: &[Task], task: &Task) -> bool {
    if task.status.is_open() {
        return true;
    }
    if all
        .iter()
        .any(|t| t.status.is_open() && t.blocked_on.as_deref().map(str::trim) == Some(&task.id))
    {
        return true;
    }
    all.iter()
        .filter(|t| t.status.is_open())
        .any(|t| is_ancestor(all, &task.id, t))
}

/// Is `id` an ancestor of `task`? Walks upward with a depth bound rather than a
/// visited set, because a cycle in `parent` is possible in a hand-edited file and
/// an unbounded walk would hang rather than report it.
fn is_ancestor(all: &[Task], id: &str, task: &Task) -> bool {
    let mut current = task.parent.as_deref();
    let mut hops = 0;
    while let Some(parent) = current {
        if parent == id {
            return true;
        }
        hops += 1;
        if hops > all.len() {
            return false;
        }
        current = all
            .iter()
            .find(|t| t.id == parent)
            .and_then(|t| t.parent.as_deref());
    }
    false
}

/// Build the register this version writes, from what converted.
pub fn rebuilt(source: &serde_json::Value, carried: Vec<Task>) -> Register {
    let mut register = Register {
        skill: source
            .get("skill")
            .and_then(|v| v.as_str())
            .unwrap_or("todo")
            .to_string(),
        skill_version: SUPPORTED.to_string(),
        comment: source
            .get("comment")
            .and_then(|v| v.as_str())
            .unwrap_or("Work queued for this project.")
            .to_string(),
        tasks: carried,
    };
    register.sort();
    register
}

/// What to tell the agent about what did not convert. Names the command rather
/// than describing the old shape: the backup is the source of truth, and
/// `todo add` is the only writer that validates.
pub fn instructions(backup: &str, unconvertible: &[Unconvertible]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} task{} did not convert. They are intact in {}.\n",
        unconvertible.len(),
        if unconvertible.len() == 1 { "" } else { "s" },
        backup
    ));
    out.push_str("Read each one and re-file it, so the queue records what you decided:\n\n");
    for item in unconvertible {
        out.push_str(&format!("  {}: {}\n", item.id, item.why));
        // The backup is plain JSON, so an agent can read it directly. rjq is
        // offered as a convenience and not required: this skill declares no
        // tools, and an instruction that assumed one would put the dependency
        // straight back.
        out.push_str(&format!("    read {backup} and find \"{}\"\n", item.id));
        out.push_str(&format!(
            "      (or, with rjq: rjq -r '.tasks[] | select(.id == \"{}\")' {})\n",
            item.id, backup
        ));
    }
    out.push_str(
        "\nThen, with the fields that task actually had:\n\
         \n  todo add --title \"...\" --detail \"...\" \\\n\
         \x20     --priority <urgent|high|normal|low|someday> \\\n\
         \x20     --status <open|partly|blocked|decided> \\\n\
         \x20     [--id T<n>] [--parent T<n>] [--blocked-on \"...\"] [--refs a,b]\n",
    );
    out
}

/// What a prune would do, decided but not yet applied.
pub struct Prune {
    /// Closed tasks that may leave.
    pub removable: Vec<Task>,
    /// Closed tasks that must stay, each with the reason.
    pub held: Vec<(String, String)>,
}

/// Decide which closed tasks may leave the live queue.
///
/// A closed task is removable unless `needed` says live work still depends on
/// it — the same rule migration carries a task forward by, so a prune and a
/// migration cannot come to different conclusions about the same file. A closed
/// ANCESTOR of an open task cannot go: the queue is a tree, and removing it
/// leaves the open task hanging off an id no longer in the file, which is exactly
/// the state `check` flags as a finding.
pub fn plan(register: &Register, older_than: Option<&str>) -> Prune {
    let mut removable = Vec::new();
    let mut held = Vec::new();
    let all = register.tasks.as_slice();

    for task in all {
        if task.status.is_open() {
            continue;
        }
        if let Some(cutoff) = older_than {
            let stamp = if task.updated_at.is_empty() {
                task.created_at.as_str()
            } else {
                task.updated_at.as_str()
            };
            // Lexicographic on fixed-width UTC stamps, as everywhere else.
            if stamp >= cutoff {
                held.push((
                    task.id.clone(),
                    format!("closed {stamp}, which is not older than {cutoff}"),
                ));
                continue;
            }
        }
        if needed(all, task) {
            held.push((task.id.clone(), why_needed(all, task)));
            continue;
        }
        removable.push(task.clone());
    }

    Prune { removable, held }
}

/// The reason `needed` said yes, for the report. Recomputed rather than returned
/// alongside the answer, so the DECISION has exactly one implementation and this
/// is only its explanation.
fn why_needed(all: &[Task], task: &Task) -> String {
    let descendants: Vec<&str> = all
        .iter()
        .filter(|t| t.status.is_open() && is_ancestor(all, &task.id, t))
        .map(|t| t.id.as_str())
        .collect();
    if !descendants.is_empty() {
        return format!("still an ancestor of open {}", descendants.join(" "));
    }
    let waiting: Vec<&str> = all
        .iter()
        .filter(|t| {
            t.status.is_open() && t.blocked_on.as_deref().map(str::trim) == Some(task.id.as_str())
        })
        .map(|t| t.id.as_str())
        .collect();
    if !waiting.is_empty() {
        return format!("open {} says it is blocked on this", waiting.join(" "));
    }
    "still needed by live work".to_string()
}

impl Prune {
    /// The archive to write beside the register: a register in its own right, so
    /// the same reader opens it and `todo --file` can query it unchanged.
    ///
    /// It carries the pruned tasks PLUS any still-live ancestor of one, copied
    /// in. Without that copy the archive is not a sound register at all: a
    /// pruned sub-task whose parent is still open points at an id the archive
    /// does not hold, `check` reports it, and `tree` cannot place it. The
    /// duplication is the right way round — an archive is a snapshot, and a
    /// snapshot missing the row above the one it kept is not readable — while
    /// the reverse never happens, because `needed` holds back any closed
    /// ancestor of live work.
    pub fn archive(&self, source: &Register) -> Register {
        let mut tasks = self.removable.clone();
        // Walked with an index rather than over a snapshot, so a parent added
        // during the pass is itself examined and — crucially — is already in
        // `tasks` when the next child asks for it. Four sub-tasks sharing one
        // missing parent copied it in four times when membership was tested
        // against a list taken before the pass.
        let mut index = 0;
        while index < tasks.len() {
            let parent = tasks[index].parent.clone();
            index += 1;
            let Some(parent) = parent else { continue };
            if parent.is_empty() || tasks.iter().any(|t| t.id == parent) {
                continue;
            }
            if let Some(ancestor) = source.find(&parent) {
                tasks.push(ancestor.clone());
            }
        }

        let mut archive = Register {
            skill: source.skill.clone(),
            skill_version: SUPPORTED.to_string(),
            comment: format!(
                "Closed tasks pruned from the live queue on {}. Kept, not deleted. \
                 A task still live in the queue appears here only as the parent of \
                 one that was pruned.",
                clock::now()
            ),
            tasks,
        };
        archive.sort();
        archive
    }

    /// The register with the removable tasks taken out.
    pub fn remaining(&self, source: &Register) -> Register {
        let leaving: Vec<&str> = self.removable.iter().map(|t| t.id.as_str()).collect();
        let mut kept = source.clone();
        kept.tasks.retain(|t| !leaving.contains(&t.id.as_str()));
        kept.sort();
        kept
    }
}

/// The day part of a timestamp, for naming the archive.
pub fn today() -> String {
    clock::now().chars().take(10).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::{Priority, Status};

    fn task(id: &str, status: Status) -> Task {
        Task {
            id: id.into(),
            title: format!("title of {id}"),
            status,
            priority: Priority::Normal,
            parent: None,
            detail: "d".into(),
            blocked_on: None,
            refs: vec![],
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
            skill_version: SUPPORTED.into(),
            comment: "fixture".into(),
            tasks,
        }
    }

    #[test]
    fn a_backup_is_named_for_the_version_it_holds() {
        assert_eq!(backup_path("TODO.json", "1.2.0"), "TODO.1.2.0.back.json");
        assert_eq!(backup_path("TODO.json", ""), "TODO.unversioned.back.json");
    }

    #[test]
    fn a_prune_takes_closed_leaves_and_leaves_open_work() {
        let reg = register(vec![
            task("T1", Status::Open),
            task("T2", Status::Done),
            task("T3", Status::Dropped),
            task("T4", Status::Decided),
        ]);
        let plan = plan(&reg, None);
        let leaving: Vec<&str> = plan.removable.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(leaving, vec!["T2", "T3"]);
        assert!(plan.held.is_empty());
        assert_eq!(plan.remaining(&reg).tasks.len(), 2);
        assert_eq!(plan.archive(&reg).tasks.len(), 2);
    }

    #[test]
    fn a_closed_parent_of_open_children_is_held() {
        let mut child = task("T2", Status::Open);
        child.parent = Some("T1".into());
        let reg = register(vec![task("T1", Status::Done), child]);
        let plan = plan(&reg, None);
        assert!(plan.removable.is_empty());
        assert_eq!(plan.held.len(), 1);
        assert!(plan.held[0].1.contains("T2"));
    }

    #[test]
    fn a_closed_task_an_open_one_is_blocked_on_is_held() {
        let mut waiting = task("T2", Status::Blocked);
        waiting.blocked_on = Some("T1".into());
        let reg = register(vec![task("T1", Status::Done), waiting]);
        let plan = plan(&reg, None);
        assert!(plan.removable.is_empty());
        assert!(plan.held[0].1.contains("T2"));
    }

    #[test]
    fn prose_mentioning_an_id_is_not_a_dependency() {
        // blocked_on holds a sentence as often as an id, so only an exact match
        // counts — otherwise a task nobody waits on could never be pruned.
        let mut waiting = task("T2", Status::Blocked);
        waiting.blocked_on = Some("the docs reorganisation, see T1 for context".into());
        let reg = register(vec![task("T1", Status::Done), waiting]);
        let plan = plan(&reg, None);
        assert_eq!(plan.removable.len(), 1);
        assert_eq!(plan.removable[0].id, "T1");
    }

    #[test]
    fn the_archive_is_sound_on_its_own() {
        // The defect: T1a was pruned while its parent T1 stayed live, so the
        // archive held a task whose parent was not in the file and `check`
        // reported it. An archive nothing can read is not a kept task.
        let mut child = task("T1a", Status::Done);
        child.parent = Some("T1".into());
        let mut grandchild = task("T1a1", Status::Done);
        grandchild.parent = Some("T1a".into());
        let mut live_child = task("T1e", Status::Open);
        live_child.parent = Some("T1".into());
        let reg = register(vec![
            task("T1", Status::Done),
            child,
            grandchild,
            live_child,
        ]);

        let plan = plan(&reg, None);
        // T1 is held: T1e still hangs off it.
        let held: Vec<&str> = plan.held.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(held, vec!["T1"]);

        let archive = plan.archive(&reg);
        assert!(archive.findings().is_empty(), "{:?}", archive.findings());
        let ids: Vec<&str> = archive.tasks.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"T1"), "the live parent is copied in: {ids:?}");
        assert!(ids.contains(&"T1a"));
        assert!(ids.contains(&"T1a1"));
        // And the live queue keeps everything the archive borrowed.
        let remaining = plan.remaining(&reg);
        assert!(remaining.find("T1").is_some());
        assert!(remaining.findings().is_empty());
    }

    #[test]
    fn one_live_parent_of_several_pruned_children_is_copied_once() {
        let mut children = vec![task("T1", Status::Done)];
        for suffix in ["a", "b", "c", "d"] {
            let mut child = task(&format!("T1{suffix}"), Status::Done);
            child.parent = Some("T1".into());
            children.push(child);
        }
        let mut live = task("T1e", Status::Open);
        live.parent = Some("T1".into());
        children.push(live);
        let reg = register(children);

        let archive = plan(&reg, None).archive(&reg);
        let copies = archive.tasks.iter().filter(|t| t.id == "T1").count();
        assert_eq!(copies, 1, "T1 appears {copies} times");
        assert!(archive.findings().is_empty(), "{:?}", archive.findings());
    }

    #[test]
    fn a_closed_ancestor_of_open_work_is_carried_forward() {
        // The defect this rule exists for: carrying only open tasks left T1e's
        // closed parent in the backup, so `check` failed on the file the
        // migration had just written.
        let mut child = task("T2", Status::Open);
        child.parent = Some("T1".into());
        let source = serde_json::json!({
            "skill": "todo",
            "skill_version": "1.4.2",
            "comment": "older",
            "tasks": [
                serde_json::to_value(task("T1", Status::Done)).unwrap(),
                serde_json::to_value(child).unwrap(),
            ]
        });
        let (carried, archived, unconvertible) = attempt(&source);
        assert!(unconvertible.is_empty());
        assert!(archived.is_empty(), "{archived:?}");
        let ids: Vec<&str> = carried.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["T1", "T2"]);
        // And the result is sound, which is the assertion that actually matters.
        assert!(rebuilt(&source, carried).findings().is_empty());
    }

    #[test]
    fn a_closed_grandparent_is_as_load_bearing_as_a_closed_parent() {
        let mut middle = task("T2", Status::Done);
        middle.parent = Some("T1".into());
        let mut leaf = task("T3", Status::Open);
        leaf.parent = Some("T2".into());
        let reg = register(vec![task("T1", Status::Done), middle, leaf]);
        let plan = plan(&reg, None);
        assert!(plan.removable.is_empty(), "{:?}", plan.removable.len());
        let held: Vec<&str> = plan.held.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(held, vec!["T1", "T2"]);
    }

    #[test]
    fn a_parent_cycle_does_not_hang_the_walk() {
        // Possible in a hand-edited file. A visited set would be the other fix;
        // the depth bound is the one that cannot itself be got wrong.
        let mut a = task("T1", Status::Done);
        a.parent = Some("T2".into());
        let mut b = task("T2", Status::Done);
        b.parent = Some("T1".into());
        let reg = register(vec![a, b, task("T3", Status::Open)]);
        let plan = plan(&reg, None);
        assert_eq!(plan.removable.len(), 2);
    }

    #[test]
    fn older_than_holds_back_a_closure_newer_than_the_cutoff() {
        // The fixture closed at 2026-01-02.
        let reg = register(vec![task("T1", Status::Done)]);

        let past_the_cutoff = plan(&reg, Some("2026-06-01T00:00:00Z"));
        assert_eq!(past_the_cutoff.removable.len(), 1);

        let inside_the_cutoff = plan(&reg, Some("2020-01-01T00:00:00Z"));
        assert!(inside_the_cutoff.removable.is_empty());
        assert!(inside_the_cutoff.held[0].1.contains("not older than"));
    }
}
