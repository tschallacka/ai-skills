// MODE: DEV
// PACKAGE: PROD
//! Resolving an id collision in a conflicted queue.
//!
//! Two branches that both queue a task both take the same next id, so the merge
//! conflict is semantic: one id, two unrelated tasks, and neither side wrong.
//! Git cannot resolve that and no textual merge can.
//!
//! This is the QUEUE's resolver and not the defect register's, because the
//! queue's structure differs in ways a rename has to respect:
//!
//! * `parent` is load-bearing — most tasks hang off another, so a rename that
//!   missed it would orphan a subtree rather than misname one entry;
//! * `blocked_on` can hold an id, and a rename has to follow it there too —
//!   but ONLY on an exact match, since the same field holds prose as often as
//!   an id, and rewriting inside a sentence corrupts what someone wrote;
//! * `refs` hold paths, never ids, so a rename must not touch them at all.
//!
//! The three clean copies come from the git index stages — `:1` the merge base,
//! `:2` ours, `:3` theirs — rather than from branch names, so this works
//! mid-conflict with nothing checked out.
//!
//! Nothing is written until the caller hands back the token this prints. That
//! token is a change detector, not a security primitive: its job is to prove the
//! caller looked at the exact content being approved, so FNV-1a over the bytes
//! is the right size of tool.

use std::process::Command;

use crate::register::{Register, Task};

/// FNV-1a, 64-bit. A content fingerprint for the confirm token — see the module
/// note on why this is deliberately not a cryptographic hash.
pub fn fingerprint(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

pub struct Sides {
    pub base: Option<Register>,
    pub ours: Register,
    pub theirs: Register,
    pub ours_label: String,
    pub theirs_label: String,
}

pub enum Contest {
    /// One id, two tasks neither side inherited. A rename fixes it.
    Collision(String),
    /// One task both sides changed, differently. A rename would not fix it: the
    /// two versions are the same task, so one content has to win.
    Divergence(String),
}

/// Is this path unmerged in the index?
pub fn is_conflicted(path: &str) -> bool {
    Command::new("git")
        .args(["ls-files", "-u", "--", path])
        .output()
        .map(|out| !out.stdout.is_empty())
        .unwrap_or(false)
}

fn stage(path: &str, number: u8) -> Option<String> {
    let out = Command::new("git")
        .args(["show", &format!(":{number}:{path}")])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

/// `rev-parse --abbrev-ref MERGE_HEAD` answers "MERGE_HEAD" rather than the
/// branch, so name-rev does the naming and MERGE_HEAD is only the last resort.
fn label(reference: &str, fallback: &str) -> String {
    let named = Command::new("git")
        .args(["name-rev", "--name-only", reference])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "undefined" && !s.contains("MERGE_HEAD"));
    match named {
        // name-rev decorates a non-branch commit as remotes/origin/x~2; the
        // trailing ~N is not part of a name a caller would type.
        Some(name) => name
            .split(['~', '^'])
            .next()
            .unwrap_or(fallback)
            .to_string(),
        None => fallback.to_string(),
    }
}

pub enum SidesError {
    NotConflicted,
    MissingStage(&'static str),
    Unparsable { side: &'static str, why: String },
}

pub fn read_sides(path: &str) -> Result<Sides, SidesError> {
    if !is_conflicted(path) {
        return Err(SidesError::NotConflicted);
    }
    let ours_text = stage(path, 2).ok_or(SidesError::MissingStage("ours"))?;
    let theirs_text = stage(path, 3).ok_or(SidesError::MissingStage("theirs"))?;

    let parse = |text: &str, side: &'static str| -> Result<Register, SidesError> {
        serde_json::from_str(text).map_err(|error| SidesError::Unparsable {
            side,
            why: error.to_string(),
        })
    };

    // The base is absent when both sides added the file, which is why only this
    // one is allowed to be missing.
    let base = stage(path, 1).and_then(|text| serde_json::from_str(&text).ok());

    Ok(Sides {
        base,
        ours: parse(&ours_text, "ours")?,
        theirs: parse(&theirs_text, "theirs")?,
        ours_label: label("HEAD", "HEAD"),
        theirs_label: label("MERGE_HEAD", "MERGE_HEAD"),
    })
}

fn find<'a>(register: &'a Register, id: &str) -> Option<&'a Task> {
    register.tasks.iter().find(|t| t.id == id)
}

fn same(left: Option<&Task>, right: Option<&Task>) -> bool {
    match (left, right) {
        (Some(a), Some(b)) => serde_json::to_value(a).ok() == serde_json::to_value(b).ok(),
        (None, None) => true,
        _ => false,
    }
}

impl Sides {
    /// Every id the two sides disagree about, sorted into the two kinds.
    ///
    /// A task only ONE side touched is not a contest: the changed side wins, the
    /// way git merges a line one branch edited. Treating that as a conflict is
    /// what made the shell version demand decisions nobody needed to make.
    pub fn contests(&self) -> Vec<Contest> {
        let mut out = Vec::new();
        for ours in &self.ours.tasks {
            let Some(theirs) = find(&self.theirs, &ours.id) else {
                continue;
            };
            if same(Some(ours), Some(theirs)) {
                continue;
            }
            let base = self.base.as_ref().and_then(|b| find(b, &ours.id));
            match base {
                None => out.push(Contest::Collision(ours.id.clone())),
                Some(base) => {
                    let ours_changed = !same(Some(ours), Some(base));
                    let theirs_changed = !same(Some(theirs), Some(base));
                    if ours_changed && theirs_changed {
                        out.push(Contest::Divergence(ours.id.clone()));
                    }
                }
            }
        }
        out
    }

    /// The highest id number either side uses, so a suggested replacement cannot
    /// land on one already spoken for.
    pub fn next_free(&self) -> u64 {
        self.ours.next_id().max(self.theirs.next_id())
    }

    /// What each side changed since the base, for the caller to eyeball.
    /// A change BOTH sides made identically is not suspicious — it only means
    /// the base predates a commit both branches carry — so it is reported
    /// separately from a one-sided edit.
    pub fn since_base(&self) -> String {
        let Some(base) = self.base.as_ref() else {
            return String::new();
        };
        let mut out = String::new();
        let mut shared = Vec::new();
        let mut ours_only = Vec::new();
        let mut theirs_only = Vec::new();

        for entry in &base.tasks {
            let ours = find(&self.ours, &entry.id);
            let theirs = find(&self.theirs, &entry.id);
            let ours_changed = ours.is_some() && !same(ours, Some(entry));
            let theirs_changed = theirs.is_some() && !same(theirs, Some(entry));
            match (ours_changed, theirs_changed) {
                (true, true) if same(ours, theirs) => shared.push(entry.id.clone()),
                (true, false) => ours_only.push(entry.id.clone()),
                (false, true) => theirs_only.push(entry.id.clone()),
                _ => {}
            }
        }

        let added = |side: &Register| -> Vec<String> {
            side.tasks
                .iter()
                .filter(|t| find(base, &t.id).is_none())
                .map(|t| t.id.clone())
                .collect()
        };
        let ours_added = added(&self.ours);
        let theirs_added = added(&self.theirs);

        if !ours_added.is_empty() {
            out.push_str(&format!(
                "  {} added {}: {}\n",
                self.ours_label,
                ours_added.len(),
                ours_added.join(" ")
            ));
        }
        if !theirs_added.is_empty() {
            out.push_str(&format!(
                "  {} added {}: {}\n",
                self.theirs_label,
                theirs_added.len(),
                theirs_added.join(" ")
            ));
        }
        if !shared.is_empty() {
            out.push_str(&format!(
                "  modified identically on both sides, so the base is simply older: {}\n",
                shared.join(" ")
            ));
        }
        if !ours_only.is_empty() {
            out.push_str(&format!(
                "  modified on {} ONLY, so read these: {}\n",
                self.ours_label,
                ours_only.join(" ")
            ));
        }
        if !theirs_only.is_empty() {
            out.push_str(&format!(
                "  modified on {} ONLY, so read these: {}\n",
                self.theirs_label,
                theirs_only.join(" ")
            ));
        }
        out
    }
}

/// Which side a decision names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Ours,
    Theirs,
}

pub struct Rename {
    pub side: Side,
    pub from: String,
    pub to: String,
}

/// Parse `side:old:new`, accepting `ours`/`theirs` or either branch name.
pub fn parse_rename(spec: &str, ours_label: &str, theirs_label: &str) -> Result<Rename, String> {
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.is_empty()) {
        return Err(format!("\"{spec}\" is not <side>:<old-id>:<new-id>"));
    }
    let side = match parts[0] {
        "ours" | "OURS" | "HEAD" => Side::Ours,
        "theirs" | "THEIRS" | "MERGE_HEAD" => Side::Theirs,
        other if other == ours_label => Side::Ours,
        other if other == theirs_label => Side::Theirs,
        other => {
            return Err(format!(
                "unknown side \"{other}\" — use ours, theirs, {ours_label} or {theirs_label}"
            ))
        }
    };
    Ok(Rename {
        side,
        from: parts[1].to_string(),
        to: parts[2].to_string(),
    })
}

/// Apply the renames and union the two sides.
///
/// Per id, and NOT ours-first: a task the base carries that only theirs changed
/// has to come from theirs, or that edit is silently dropped. Getting this wrong
/// was a real data-loss bug in the shell version, caught by its own test only
/// because the test asserted BOTH directions.
pub fn merge(sides: &Sides, renames: &[Rename]) -> Register {
    let rename_side = |register: &Register, side: Side| -> Register {
        let mut copy = register.clone();
        for task in &mut copy.tasks {
            for rename in renames.iter().filter(|r| r.side == side) {
                if task.id == rename.from {
                    task.id = rename.to.clone();
                }
                // The tree edge. Missing this orphans a whole subtree rather
                // than misnaming one entry, which is why the queue's resolver
                // cannot be the defect register's.
                if task.parent.as_deref() == Some(rename.from.as_str()) {
                    task.parent = Some(rename.to.clone());
                }
                // Exact match only: this field holds prose as often as an id,
                // and a substring rewrite would edit somebody's sentence.
                if task.blocked_on.as_deref().map(str::trim) == Some(rename.from.as_str()) {
                    task.blocked_on = Some(rename.to.clone());
                }
                // `refs` are deliberately untouched — they hold paths, not ids.
            }
        }
        copy
    };

    let ours = rename_side(&sides.ours, Side::Ours);
    let theirs = rename_side(&sides.theirs, Side::Theirs);

    let mut merged = ours.clone();
    merged.tasks.clear();

    let mut order: Vec<String> = ours.tasks.iter().map(|t| t.id.clone()).collect();
    for task in &theirs.tasks {
        if !order.contains(&task.id) {
            order.push(task.id.clone());
        }
    }

    for id in order {
        let ours_entry = find(&ours, &id);
        let theirs_entry = find(&theirs, &id);
        let base_entry = sides.base.as_ref().and_then(|b| find(b, &id));
        let chosen = match (ours_entry, theirs_entry) {
            (Some(o), None) => Some(o),
            (None, Some(t)) => Some(t),
            (Some(o), Some(t)) if same(Some(o), Some(t)) => Some(o),
            (Some(o), Some(t)) => match base_entry {
                // Only one side moved it, so that side wins.
                Some(base) if same(Some(o), Some(base)) => Some(t),
                Some(base) if same(Some(t), Some(base)) => Some(o),
                // A divergence, refused before this point.
                _ => Some(o),
            },
            (None, None) => None,
        };
        if let Some(task) = chosen {
            merged.tasks.push(task.clone());
        }
    }

    merged.sort();
    merged
}

/// Ids the renames mention that still appear in prose. Reported, never
/// rewritten: an id inside a sentence may be a prefix of a longer one, or belong
/// to the defect register, and a blind substitution corrupts text someone wrote.
///
/// `blocked_on` is scanned here as well as rewritten in `merge`, and that is not
/// a contradiction: `merge` rewrites it only on an exact match, so a SENTENCE
/// mentioning the old id survives, and this is what tells the caller about it.
pub fn prose_mentions(register: &Register, renames: &[Rename]) -> Vec<String> {
    let mut out = Vec::new();
    for task in &register.tasks {
        let prose = [
            Some(task.title.as_str()),
            Some(task.detail.as_str()),
            task.blocked_on.as_deref(),
            task.note.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");

        for rename in renames {
            if mentions(&prose, &rename.from) {
                out.push(format!(
                    "  {} still mentions {} in its prose",
                    task.id, rename.from
                ));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// A whole-word-ish match: the id must not be followed by another digit, so
/// `T8` does not match inside `T81`.
fn mentions(text: &str, id: &str) -> bool {
    let mut from = 0;
    while let Some(found) = text[from..].find(id) {
        let at = from + found;
        let after = text[at + id.len()..].chars().next();
        let before = text[..at].chars().last();
        let boundary_before = before.is_none_or(|c| !c.is_ascii_alphanumeric());
        let boundary_after = after.is_none_or(|c| !c.is_ascii_digit());
        if boundary_before && boundary_after {
            return true;
        }
        from = at + id.len();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::{Priority, Status};

    fn task(id: &str) -> Task {
        Task {
            id: id.into(),
            title: format!("title of {id}"),
            status: Status::Open,
            priority: Priority::Normal,
            parent: None,
            detail: "d".into(),
            blocked_on: None,
            refs: vec![],
            note: None,
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

    fn sides(base: Option<Register>, ours: Register, theirs: Register) -> Sides {
        Sides {
            base,
            ours,
            theirs,
            ours_label: "master".into(),
            theirs_label: "feature".into(),
        }
    }

    #[test]
    fn a_fingerprint_changes_with_the_content() {
        assert_eq!(fingerprint(b"abc"), fingerprint(b"abc"));
        assert_ne!(fingerprint(b"abc"), fingerprint(b"abd"));
        assert_eq!(fingerprint(b"abc").len(), 16);
    }

    #[test]
    fn an_id_is_not_matched_inside_a_longer_id() {
        assert!(mentions("waiting on T8 first", "T8"));
        assert!(!mentions("waiting on T81 first", "T8"));
        assert!(mentions("see T8.", "T8"));
        assert!(!mentions("see SUBT8 here", "T8"));
    }

    #[test]
    fn a_side_is_named_by_keyword_or_branch() {
        let rename = parse_rename("theirs:T1:T2", "master", "feature").expect("parses");
        assert_eq!(rename.side, Side::Theirs);
        let rename = parse_rename("master:T1:T2", "master", "feature").expect("parses");
        assert_eq!(rename.side, Side::Ours);
        assert!(parse_rename("nosuch:T1:T2", "master", "feature").is_err());
        assert!(parse_rename("theirs:T1", "master", "feature").is_err());
    }

    #[test]
    fn a_rename_follows_the_tree_edge_and_an_exact_blocker() {
        let mut child = task("T2");
        child.parent = Some("T1".into());
        let mut waiting = task("T3");
        waiting.blocked_on = Some("T1".into());
        let mut prose = task("T4");
        prose.blocked_on = Some("the docs pass, then T1".into());

        let ours = register(vec![task("T1"), child, waiting, prose]);
        let sides = sides(None, ours, register(vec![]));
        let renames = vec![Rename {
            side: Side::Ours,
            from: "T1".into(),
            to: "T9".into(),
        }];
        let merged = merge(&sides, &renames);

        assert!(merged.find("T9").is_some());
        assert!(merged.find("T1").is_none());
        assert_eq!(merged.find("T2").unwrap().parent.as_deref(), Some("T9"));
        assert_eq!(merged.find("T3").unwrap().blocked_on.as_deref(), Some("T9"));
        // The sentence is left exactly as written, and reported instead.
        assert_eq!(
            merged.find("T4").unwrap().blocked_on.as_deref(),
            Some("the docs pass, then T1")
        );
        let reported = prose_mentions(&merged, &renames);
        assert!(reported.iter().any(|line| line.contains("T4")));
    }

    #[test]
    fn a_rename_never_touches_refs() {
        let mut with_refs = task("T1");
        // A path that happens to contain the id, which a substring rewrite over
        // every field would have mangled.
        with_refs.refs = vec!["planning/T1-notes.md".into()];
        let sides = sides(None, register(vec![with_refs]), register(vec![]));
        let merged = merge(
            &sides,
            &[Rename {
                side: Side::Ours,
                from: "T1".into(),
                to: "T9".into(),
            }],
        );
        assert_eq!(merged.tasks[0].refs, vec!["planning/T1-notes.md"]);
    }

    #[test]
    fn an_edit_only_theirs_made_is_not_dropped() {
        // The data-loss shape: a union that took ours first would silently lose
        // this. Asserted in both directions, which is the only reason the shell
        // version's bug was ever caught.
        let base = register(vec![task("T1")]);
        let ours = base.clone();
        let mut edited = task("T1");
        edited.title = "changed only on theirs".into();
        let theirs = register(vec![edited]);
        let merged = merge(&sides(Some(base.clone()), ours, theirs), &[]);
        assert_eq!(merged.find("T1").unwrap().title, "changed only on theirs");

        let mut edited = task("T1");
        edited.title = "changed only on ours".into();
        let merged = merge(
            &sides(Some(base.clone()), register(vec![edited]), base),
            &[],
        );
        assert_eq!(merged.find("T1").unwrap().title, "changed only on ours");
    }
}
