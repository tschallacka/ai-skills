// MODE: DEV
// PACKAGE: PROD
//! The question register as types.
//!
//! Field order in `Question` is the ON-DISK order, and serde serialises a
//! struct in declaration order, so a read-modify-write reproduces the file
//! byte for byte rather than reordering keys under the author. Do not
//! reorder these fields to taste; the register is a tracked file and a
//! reordering is a diff on every entry.

use serde::{Deserialize, Serialize};

/// Refuses an unknown key rather than dropping it, the same reason
/// bug-report's and todo's own registers do: a typed reader that ignores
/// what it does not understand silently deletes a field a newer writer
/// added.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Register {
    pub skill: String,
    pub skill_version: String,
    pub comment: String,
    pub questions: Vec<Question>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub priority: Priority,
    pub branch: String,
    pub options: Vec<Choice>,
    pub context: String,
    pub chosen: Option<String>,
    pub resolution: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// One lettered answer option: `a`, `b`, `c`, ... with its label.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub letter: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Open,
    Decided,
    Implemented,
    Closed,
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
    pub fn is_open(self) -> bool {
        matches!(self, Status::Open)
    }

    /// The user has picked an option, but the pick has not yet been carried
    /// out in the code -- the one state this register keeps pinned and
    /// visible until an agent acts on it.
    pub fn is_decided(self) -> bool {
        matches!(self, Status::Decided)
    }

    /// Still needs attention from someone: an answer (`Open`) or an
    /// implementation (`Decided`). `Implemented`, `Closed`, `Dropped` and
    /// `Obsolete` are all resting states -- nothing further is expected of
    /// any of them.
    pub fn is_pending(self) -> bool {
        matches!(self, Status::Open | Status::Decided)
    }
}

impl Priority {
    /// Worst (most urgent) first.
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

/// The numeric part of a `Q<n>` id, or `u64::MAX` when it does not parse --
/// a non-conforming id loses only its ranking, never the entry itself,
/// parallel to bug-report's and todo's own `id_number`.
pub fn id_number(id: &str) -> u64 {
    let digits: String = id
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(u64::MAX)
}

impl Register {
    /// The next free `Q` number, from the register's own high-water mark.
    /// No branch is consulted here: ids are minted the same way bug-report's
    /// and todo's own `next_id` work, with the collision-avoidance discipline
    /// left to whichever repository hosts this tool (see `.agents/MAINTAINER.md`
    /// 1.14 in this one).
    pub fn next_id(&self) -> u64 {
        self.questions
            .iter()
            .map(|q| id_number(&q.id))
            .filter(|n| *n != u64::MAX)
            .max()
            .map_or(1, |n| n + 1)
    }

    pub fn find(&self, id: &str) -> Option<&Question> {
        self.questions.iter().find(|q| q.id == id)
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut Question> {
        self.questions.iter_mut().find(|q| q.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_question(id: &str) -> Question {
        Question {
            id: id.to_string(),
            title: "Pick a strategy".to_string(),
            status: Status::Open,
            priority: Priority::Urgent,
            branch: "feature/x".to_string(),
            options: vec![
                Choice {
                    letter: "a".to_string(),
                    label: "Yes".to_string(),
                },
                Choice {
                    letter: "b".to_string(),
                    label: "No".to_string(),
                },
            ],
            context: "Stubbed with option a while waiting.".to_string(),
            chosen: None,
            resolution: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn a_register_round_trips_through_json_unchanged() {
        let register = Register {
            skill: "decisions".to_string(),
            skill_version: "2.0.0-alpha.4".to_string(),
            comment: "The question register.".to_string(),
            questions: vec![sample_question("Q1")],
        };
        let text = serde_json::to_string(&register).unwrap();
        let parsed: Register = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed.questions[0].id, "Q1");
        assert_eq!(parsed.questions[0].options.len(), 2);
    }

    #[test]
    fn next_id_on_an_empty_register_is_one() {
        let register = Register {
            skill: "decisions".to_string(),
            skill_version: "2.0.0-alpha.4".to_string(),
            comment: String::new(),
            questions: vec![],
        };
        assert_eq!(register.next_id(), 1);
    }

    #[test]
    fn next_id_skips_to_one_past_the_highest_existing_number() {
        let register = Register {
            skill: "decisions".to_string(),
            skill_version: "2.0.0-alpha.4".to_string(),
            comment: String::new(),
            questions: vec![sample_question("Q1"), sample_question("Q3")],
        };
        assert_eq!(register.next_id(), 4);
    }

    #[test]
    fn status_and_priority_serialize_to_kebab_case() {
        assert_eq!(serde_json::to_string(&Status::Open).unwrap(), "\"open\"");
        assert_eq!(
            serde_json::to_string(&Status::Decided).unwrap(),
            "\"decided\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Implemented).unwrap(),
            "\"implemented\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Closed).unwrap(),
            "\"closed\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Dropped).unwrap(),
            "\"dropped\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Obsolete).unwrap(),
            "\"obsolete\""
        );
        assert_eq!(
            serde_json::to_string(&Priority::Urgent).unwrap(),
            "\"urgent\""
        );
        assert_eq!(
            serde_json::to_string(&Priority::Someday).unwrap(),
            "\"someday\""
        );
    }
}
