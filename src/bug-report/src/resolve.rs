// MODE: DEV
// PACKAGE: PROD
//! Resolving an id collision in a conflicted register.
//!
//! Two branches that both file a defect both take the same next id, so the
//! merge conflict is semantic: one id, two unrelated defects, and neither side
//! wrong. Git cannot resolve that and no textual merge can.
//!
//! The three clean copies come from the git index stages — `:1` the merge base,
//! `:2` ours, `:3` theirs — rather than from branch names, so this works
//! mid-conflict with nothing checked out.
//!
//! Nothing is written until the caller hands back the token this prints. That
//! token is a change detector, not a security primitive: its job is to prove the
//! caller looked at the exact content being approved, so FNV-1a over the bytes
//! is the right size of tool. A cryptographic digest here would imply a threat
//! model that does not exist, and would mean either a third dependency or a
//! second copy of plan-crypt's SHA-256.

use std::process::Command;

use crate::register::{Bug, Register};

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
    /// One id, two entries neither side inherited. A rename fixes it.
    Collision(String),
    /// One entry both sides changed, differently. A rename would not fix it:
    /// the two versions are the same defect, so one content has to win.
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

fn find<'a>(register: &'a Register, id: &str) -> Option<&'a Bug> {
    register.bugs.iter().find(|b| b.id == id)
}

fn same(left: Option<&Bug>, right: Option<&Bug>) -> bool {
    match (left, right) {
        (Some(a), Some(b)) => serde_json::to_value(a).ok() == serde_json::to_value(b).ok(),
        (None, None) => true,
        _ => false,
    }
}

impl Sides {
    /// Every id the two sides disagree about, sorted into the two kinds.
    ///
    /// An entry only ONE side touched is not a contest: the changed side wins,
    /// the way git merges a line one branch edited. Treating that as a conflict
    /// is what made the shell version demand decisions nobody needed to make.
    pub fn contests(&self) -> Vec<Contest> {
        let mut out = Vec::new();
        for ours in &self.ours.bugs {
            let theirs = find(&self.theirs, &ours.id);
            let Some(theirs) = theirs else { continue };
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

        for entry in &base.bugs {
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
            side.bugs
                .iter()
                .filter(|b| find(base, &b.id).is_none())
                .map(|b| b.id.clone())
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
/// Per id, and NOT ours-first: an entry the base carries that only theirs
/// changed has to come from theirs, or that edit is silently dropped. Getting
/// this wrong was a real data-loss bug in the shell version, caught by its own
/// test only because the test asserted BOTH directions.
pub fn merge(sides: &Sides, renames: &[Rename]) -> Register {
    let rename_side = |register: &Register, side: Side| -> Register {
        let mut copy = register.clone();
        for bug in &mut copy.bugs {
            for rename in renames.iter().filter(|r| r.side == side) {
                if bug.id == rename.from {
                    bug.id = rename.to.clone();
                }
                if bug.parent.as_deref() == Some(rename.from.as_str()) {
                    bug.parent = Some(rename.to.clone());
                }
            }
        }
        copy
    };

    let ours = rename_side(&sides.ours, Side::Ours);
    let theirs = rename_side(&sides.theirs, Side::Theirs);

    let mut merged = ours.clone();
    merged.bugs.clear();

    let mut order: Vec<String> = ours.bugs.iter().map(|b| b.id.clone()).collect();
    for bug in &theirs.bugs {
        if !order.contains(&bug.id) {
            order.push(bug.id.clone());
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
        if let Some(bug) = chosen {
            merged.bugs.push(bug.clone());
        }
    }

    merged.sort();
    merged
}

/// Ids the renames mention that still appear in prose. Reported, never
/// rewritten: an id inside a sentence may be a prefix of a longer one, or belong
/// to the other register, and a blind substitution corrupts text someone wrote.
pub fn prose_mentions(register: &Register, renames: &[Rename]) -> Vec<String> {
    let mut out = Vec::new();
    for bug in &register.bugs {
        let prose = [
            bug.mechanism.as_deref(),
            bug.fix.as_deref(),
            bug.verification.as_deref(),
            Some(bug.observed.as_str()),
            Some(bug.expected.as_str()),
            Some(bug.reproduce.as_str()),
            Some(bug.title.as_str()),
            bug.notes.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");

        for rename in renames {
            if mentions(&prose, &rename.from) {
                out.push(format!(
                    "  {} still mentions {} in its prose",
                    bug.id, rename.from
                ));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// A whole-word-ish match: the id must not be followed by another digit, so
/// `B9` does not match inside `B99`.
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

    #[test]
    fn a_fingerprint_changes_with_the_content() {
        assert_eq!(fingerprint(b"abc"), fingerprint(b"abc"));
        assert_ne!(fingerprint(b"abc"), fingerprint(b"abd"));
        assert_eq!(fingerprint(b"abc").len(), 16);
    }

    #[test]
    fn an_id_is_not_matched_inside_a_longer_id() {
        assert!(mentions("caused by B9 originally", "B9"));
        assert!(!mentions("caused by B99 originally", "B9"));
        assert!(mentions("see B9.", "B9"));
        assert!(!mentions("see SUBB9 here", "B9"));
    }

    #[test]
    fn a_side_is_named_by_keyword_or_branch() {
        let rename = parse_rename("theirs:B1:B2", "master", "feature").expect("parses");
        assert_eq!(rename.side, Side::Theirs);
        let rename = parse_rename("master:B1:B2", "master", "feature").expect("parses");
        assert_eq!(rename.side, Side::Ours);
        assert!(parse_rename("nosuch:B1:B2", "master", "feature").is_err());
        assert!(parse_rename("theirs:B1", "master", "feature").is_err());
    }
}
