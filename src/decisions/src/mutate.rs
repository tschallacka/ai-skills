// MODE: DEV
// PACKAGE: PROD
//! Filing, answering, stubbing and closing questions.

use crate::register::{Choice, Priority, Question, Register, Status};
use std::time::{SystemTime, UNIX_EPOCH};

/// `YYYY-MM-DDTHH:MM:SSZ` for now, dependency-free the same way bug-report's
/// own clock module is -- shelling out to `date` would reintroduce exactly
/// the environment dependency a compiled binary exists to remove.
fn now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = seconds.div_euclid(86_400);
    let rem = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant's days-to-civil algorithm, the same one bug-report's clock
/// module uses.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// What `add` needs to file a new question.
pub struct NewQuestion {
    pub title: String,
    pub options: Vec<Choice>,
    pub priority: Priority,
    pub branch: String,
    pub context: String,
}

/// Add one entry, returning the id it was given. The id comes from the
/// register's own high-water mark, the same way bug-report's and todo's own
/// `add` allocate theirs.
pub fn add(register: &mut Register, new: NewQuestion) -> Result<String, String> {
    if new.options.is_empty() {
        return Err("a question needs at least one option".to_string());
    }
    let when = now();
    let id = format!("Q{}", register.next_id());
    register.questions.push(Question {
        id: id.clone(),
        title: new.title,
        status: Status::Open,
        priority: new.priority,
        branch: new.branch,
        options: new.options,
        context: new.context,
        chosen: None,
        resolution: None,
        created_at: when.clone(),
        updated_at: when,
    });
    Ok(id)
}

/// Record the user's pick: sets `chosen` and moves the question to
/// `Answered`. Refuses an unknown id or a letter the question did not offer.
pub fn answer(register: &mut Register, id: &str, letter: &str) -> Result<(), String> {
    let question = register
        .find_mut(id)
        .ok_or_else(|| format!("{id}: no such question"))?;
    if !question.options.iter().any(|o| o.letter == letter) {
        return Err(format!(
            "{id}: option {letter} is not one of this question's options"
        ));
    }
    question.chosen = Some(letter.to_string());
    question.status = Status::Answered;
    question.updated_at = now();
    Ok(())
}

/// Records what the agent assumed or stubbed while the question stayed open,
/// without changing its status -- the explicit "keep working, mark it open"
/// workflow this register exists for.
pub fn stub(register: &mut Register, id: &str, assumption: &str) -> Result<(), String> {
    let question = register
        .find_mut(id)
        .ok_or_else(|| format!("{id}: no such question"))?;
    if !question.context.is_empty() {
        question.context.push_str("\n\n");
    }
    question.context.push_str(assumption);
    question.updated_at = now();
    Ok(())
}

/// Close a question with a resolution, from any status -- answering first is
/// not required, since a question can also be withdrawn or resolved without
/// ever being formally answered.
pub fn close(register: &mut Register, id: &str, resolution: &str) -> Result<(), String> {
    let question = register
        .find_mut(id)
        .ok_or_else(|| format!("{id}: no such question"))?;
    question.resolution = Some(resolution.to_string());
    question.status = Status::Closed;
    question.updated_at = now();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_register() -> Register {
        Register {
            skill: "decisions".to_string(),
            skill_version: "2.0.0-alpha.4".to_string(),
            comment: String::new(),
            questions: vec![],
        }
    }

    fn two_options() -> Vec<Choice> {
        vec![
            Choice {
                letter: "a".to_string(),
                label: "Yes".to_string(),
            },
            Choice {
                letter: "b".to_string(),
                label: "No".to_string(),
            },
        ]
    }

    #[test]
    fn add_on_an_empty_register_returns_q1_open() {
        let mut register = empty_register();
        let id = add(
            &mut register,
            NewQuestion {
                title: "Pick one".to_string(),
                options: two_options(),
                priority: Priority::Urgent,
                branch: "main".to_string(),
                context: String::new(),
            },
        )
        .unwrap();
        assert_eq!(id, "Q1");
        assert_eq!(register.questions[0].status, Status::Open);
    }

    #[test]
    fn answer_on_an_unknown_id_is_refused() {
        let mut register = empty_register();
        let err = answer(&mut register, "Q9", "a").unwrap_err();
        assert!(err.contains("Q9"));
    }

    #[test]
    fn answer_with_an_option_not_offered_is_refused() {
        let mut register = empty_register();
        let id = add(
            &mut register,
            NewQuestion {
                title: "Pick one".to_string(),
                options: two_options(),
                priority: Priority::Normal,
                branch: "main".to_string(),
                context: String::new(),
            },
        )
        .unwrap();
        let err = answer(&mut register, &id, "z").unwrap_err();
        assert!(err.contains('z'));
    }

    #[test]
    fn stub_appends_to_context_and_leaves_status_unchanged() {
        let mut register = empty_register();
        let id = add(
            &mut register,
            NewQuestion {
                title: "Pick one".to_string(),
                options: two_options(),
                priority: Priority::Normal,
                branch: "main".to_string(),
                context: "initial".to_string(),
            },
        )
        .unwrap();
        stub(&mut register, &id, "assumed option a").unwrap();
        let question = register.find(&id).unwrap();
        assert_eq!(question.status, Status::Open);
        assert!(question.context.contains("assumed option a"));
    }

    #[test]
    fn close_sets_resolution_and_status_closed() {
        let mut register = empty_register();
        let id = add(
            &mut register,
            NewQuestion {
                title: "Pick one".to_string(),
                options: two_options(),
                priority: Priority::Normal,
                branch: "main".to_string(),
                context: String::new(),
            },
        )
        .unwrap();
        close(&mut register, &id, "Went with a").unwrap();
        let question = register.find(&id).unwrap();
        assert_eq!(question.status, Status::Closed);
        assert_eq!(question.resolution.as_deref(), Some("Went with a"));
    }
}
