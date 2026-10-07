// MODE: DEV
// PACKAGE: PROD
//! Listing and filtering questions.

use crate::register::{Priority, Question, Register, Status};

/// What `list` narrows by. `None` in any field means that field does not
/// filter; combining fields narrows further, never widens.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    pub branch: Option<String>,
}

/// The matching questions, in the register's own existing order.
pub fn list<'a>(register: &'a Register, filter: &Filter) -> Vec<&'a Question> {
    register
        .questions
        .iter()
        .filter(|q| filter.status.is_none_or(|s| q.status == s))
        .filter(|q| filter.priority.is_none_or(|p| q.priority == p))
        .filter(|q| filter.branch.as_deref().is_none_or(|b| q.branch == b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::Choice;

    fn question(id: &str, status: Status, priority: Priority, branch: &str) -> Question {
        Question {
            id: id.to_string(),
            title: "t".to_string(),
            status,
            priority,
            branch: branch.to_string(),
            options: vec![Choice {
                letter: "a".to_string(),
                label: "Yes".to_string(),
            }],
            context: String::new(),
            chosen: None,
            resolution: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn register() -> Register {
        Register {
            skill: "decisions".to_string(),
            skill_version: "2.0.0-alpha.4".to_string(),
            comment: String::new(),
            questions: vec![
                question("Q1", Status::Open, Priority::Normal, "main"),
                question("Q2", Status::Closed, Priority::Normal, "feature/x"),
            ],
        }
    }

    #[test]
    fn filter_by_status_open_returns_only_open() {
        let register = register();
        let filter = Filter {
            status: Some(Status::Open),
            ..Default::default()
        };
        let matches = list(&register, &filter);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, "Q1");
    }

    #[test]
    fn filter_by_priority_urgent_ignores_status() {
        let mut register = register();
        register.questions[1].priority = Priority::Urgent;
        let filter = Filter {
            priority: Some(Priority::Urgent),
            ..Default::default()
        };
        let matches = list(&register, &filter);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, "Q2");
    }

    #[test]
    fn filter_by_branch_returns_only_that_branch() {
        let register = register();
        let filter = Filter {
            branch: Some("feature/x".to_string()),
            ..Default::default()
        };
        let matches = list(&register, &filter);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, "Q2");
    }

    #[test]
    fn combining_filters_narrows_further() {
        let register = register();
        let filter = Filter {
            status: Some(Status::Open),
            branch: Some("feature/x".to_string()),
            ..Default::default()
        };
        assert_eq!(list(&register, &filter).len(), 0);
    }
}
