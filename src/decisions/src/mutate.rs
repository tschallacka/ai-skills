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
/// `Decided`. Refuses an unknown id or a letter the question did not offer.
/// Refused once the question is no longer pending (`Implemented` and later
/// are resting states): re-answering must not revert an implementation back
/// to merely `Decided`.
pub fn answer(register: &mut Register, id: &str, letter: &str) -> Result<(), String> {
    let question = register
        .find_mut(id)
        .ok_or_else(|| format!("{id}: no such question"))?;
    if !question.status.is_pending() {
        return Err(format!(
            "{id}: cannot answer a question that is already {:?}",
            question.status
        ));
    }
    if !question.options.iter().any(|o| o.letter == letter) {
        return Err(format!(
            "{id}: option {letter} is not one of this question's options"
        ));
    }
    question.chosen = Some(letter.to_string());
    question.status = Status::Decided;
    question.updated_at = now();
    Ok(())
}

/// Record the human's own words as the pick, for the question-etiquette
/// "something else -- tell me" case: the offered letters did not fit, so
/// there is no letter to validate against. Sets `chosen` to exactly the
/// text given and moves the question to `Decided`, the same as a lettered
/// `answer` -- it is still a pick, just not one of the ones that were on
/// offer.
pub fn answer_with_text(register: &mut Register, id: &str, text: &str) -> Result<(), String> {
    let question = register
        .find_mut(id)
        .ok_or_else(|| format!("{id}: no such question"))?;
    if !question.status.is_pending() {
        return Err(format!(
            "{id}: cannot answer a question that is already {:?}",
            question.status
        ));
    }
    if text.trim().is_empty() {
        return Err(format!(
            "{id}: a free-text answer needs the words the human actually typed"
        ));
    }
    question.chosen = Some(text.to_string());
    question.status = Status::Decided;
    question.updated_at = now();
    Ok(())
}

/// Add a new option, or relabel an existing one if `letter` is already
/// offered -- one path for both, since fixing a typo in an option and
/// offering a fresh choice both just mean "`letter` now means `label`".
/// Refused once the question is no longer open: editing what was on offer
/// after someone has already picked from it would leave `chosen` pointing
/// at a choice that may no longer say what it said when it was picked.
pub fn set_option(
    register: &mut Register,
    id: &str,
    letter: &str,
    label: &str,
) -> Result<(), String> {
    let question = register
        .find_mut(id)
        .ok_or_else(|| format!("{id}: no such question"))?;
    if !question.status.is_open() {
        return Err(format!(
            "{id}: options can only be edited while the question is open (status is {:?})",
            question.status
        ));
    }
    if letter.trim().is_empty() {
        return Err(format!("{id}: an option letter cannot be blank"));
    }
    if label.trim().is_empty() {
        return Err(format!("{id}: option {letter} needs a label"));
    }
    match question.options.iter_mut().find(|o| o.letter == letter) {
        Some(existing) => existing.label = label.to_string(),
        None => question.options.push(Choice {
            letter: letter.to_string(),
            label: label.to_string(),
        }),
    }
    question.updated_at = now();
    Ok(())
}

/// Remove one or more options at once, atomically: every named letter must
/// already be one of the question's options, and at least one option must
/// remain, or nothing changes at all -- a partial removal would leave the
/// caller unsure which of several letters actually took effect. Refused
/// once the question is no longer open, same reasoning as `set_option`.
pub fn remove_options(register: &mut Register, id: &str, letters: &[String]) -> Result<(), String> {
    let question = register
        .find_mut(id)
        .ok_or_else(|| format!("{id}: no such question"))?;
    if !question.status.is_open() {
        return Err(format!(
            "{id}: options can only be edited while the question is open (status is {:?})",
            question.status
        ));
    }
    for letter in letters {
        if !question.options.iter().any(|o| &o.letter == letter) {
            return Err(format!(
                "{id}: option {letter} is not one of this question's options"
            ));
        }
    }
    let remaining: Vec<Choice> = question
        .options
        .iter()
        .filter(|o| !letters.contains(&o.letter))
        .cloned()
        .collect();
    if remaining.is_empty() {
        return Err(format!(
            "{id}: removing {} would leave no options at all",
            letters.join(", ")
        ));
    }
    question.options = remaining;
    question.updated_at = now();
    Ok(())
}

/// Resort the options into exactly the given letter order -- a permutation
/// of the current letters, not a way to add, drop, or rename one at the
/// same time: keeping those three separate operations keeps each one
/// impossible to get silently wrong in the other's way. Refused once the
/// question is no longer open, same reasoning as `set_option`.
pub fn reorder_options(register: &mut Register, id: &str, order: &[String]) -> Result<(), String> {
    let question = register
        .find_mut(id)
        .ok_or_else(|| format!("{id}: no such question"))?;
    if !question.status.is_open() {
        return Err(format!(
            "{id}: options can only be edited while the question is open (status is {:?})",
            question.status
        ));
    }
    let mut current: Vec<String> = question.options.iter().map(|o| o.letter.clone()).collect();
    let mut wanted = order.to_vec();
    current.sort();
    wanted.sort();
    if current != wanted {
        return Err(format!(
            "{id}: reorder must name exactly the current options ({}), got {}",
            question
                .options
                .iter()
                .map(|o| o.letter.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            order.join(", ")
        ));
    }
    let reordered: Vec<Choice> = order
        .iter()
        .map(|letter| {
            question
                .options
                .iter()
                .find(|o| &o.letter == letter)
                .cloned()
                .expect("checked as a member of current above")
        })
        .collect();
    question.options = reordered;
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

/// Mark a decided question as carried out in the code, optionally recording
/// what was done. Refuses anything not currently `Decided`: there is nothing
/// to implement before a pick exists, and a question that is open, already
/// implemented, closed, dropped or obsolete has no further work for this to
/// record -- `close` is the way to withdraw a question instead.
pub fn implement(register: &mut Register, id: &str, note: &str) -> Result<(), String> {
    let question = register
        .find_mut(id)
        .ok_or_else(|| format!("{id}: no such question"))?;
    if question.status != Status::Decided {
        return Err(format!(
            "{id}: only a decided question can be marked implemented (status is {:?})",
            question.status
        ));
    }
    if !note.is_empty() {
        question.resolution = Some(note.to_string());
    }
    question.status = Status::Implemented;
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

    #[test]
    fn answer_moves_a_question_to_decided_not_answered() {
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
        answer(&mut register, &id, "a").unwrap();
        assert_eq!(register.find(&id).unwrap().status, Status::Decided);
    }

    #[test]
    fn implement_on_a_decided_question_records_the_note_and_moves_to_implemented() {
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
        answer(&mut register, &id, "a").unwrap();
        implement(&mut register, &id, "landed in src/thing.rs").unwrap();
        let question = register.find(&id).unwrap();
        assert_eq!(question.status, Status::Implemented);
        assert_eq!(
            question.resolution.as_deref(),
            Some("landed in src/thing.rs")
        );
    }

    #[test]
    fn answer_on_an_already_implemented_question_is_refused_not_reverted() {
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
        answer(&mut register, &id, "a").unwrap();
        implement(&mut register, &id, "landed in src/thing.rs").unwrap();
        let err = answer(&mut register, &id, "b").unwrap_err();
        assert!(err.contains(&id));
        assert_eq!(register.find(&id).unwrap().status, Status::Implemented);
    }

    #[test]
    fn answer_with_text_on_an_already_implemented_question_is_refused_not_reverted() {
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
        answer(&mut register, &id, "a").unwrap();
        implement(&mut register, &id, "landed in src/thing.rs").unwrap();
        let err = answer_with_text(&mut register, &id, "redo it").unwrap_err();
        assert!(err.contains(&id));
        assert_eq!(register.find(&id).unwrap().status, Status::Implemented);
    }

    #[test]
    fn implement_before_a_pick_exists_is_refused() {
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
        let err = implement(&mut register, &id, "").unwrap_err();
        assert!(err.contains(&id));
        assert_eq!(register.find(&id).unwrap().status, Status::Open);
    }

    #[test]
    fn implement_on_an_already_implemented_question_is_refused() {
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
        answer(&mut register, &id, "a").unwrap();
        implement(&mut register, &id, "").unwrap();
        assert!(implement(&mut register, &id, "again").is_err());
    }

    #[test]
    fn answer_with_text_records_the_human_s_own_words_and_decides() {
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
        answer_with_text(&mut register, &id, "Actually, do neither").unwrap();
        let question = register.find(&id).unwrap();
        assert_eq!(question.status, Status::Decided);
        assert_eq!(question.chosen.as_deref(), Some("Actually, do neither"));
    }

    #[test]
    fn answer_with_text_refuses_blank_text() {
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
        assert!(answer_with_text(&mut register, &id, "   ").is_err());
        assert_eq!(register.find(&id).unwrap().status, Status::Open);
    }

    #[test]
    fn set_option_relabels_an_existing_letter_and_adds_a_new_one() {
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
        set_option(&mut register, &id, "a", "Definitely yes").unwrap();
        set_option(&mut register, &id, "c", "Maybe").unwrap();
        let options = &register.find(&id).unwrap().options;
        assert_eq!(options.len(), 3);
        assert_eq!(options[0].label, "Definitely yes");
        assert_eq!(options[2].letter, "c");
        assert_eq!(options[2].label, "Maybe");
    }

    #[test]
    fn set_option_is_refused_once_the_question_is_decided() {
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
        answer(&mut register, &id, "a").unwrap();
        assert!(set_option(&mut register, &id, "c", "Maybe").is_err());
    }

    #[test]
    fn set_option_is_refused_with_a_blank_letter() {
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
        assert!(set_option(&mut register, &id, "  ", "Maybe").is_err());
    }

    #[test]
    fn remove_options_drops_only_the_named_letters() {
        let mut register = empty_register();
        let id = add(
            &mut register,
            NewQuestion {
                title: "Pick one".to_string(),
                options: vec![
                    Choice {
                        letter: "a".to_string(),
                        label: "Yes".to_string(),
                    },
                    Choice {
                        letter: "b".to_string(),
                        label: "No".to_string(),
                    },
                    Choice {
                        letter: "c".to_string(),
                        label: "Maybe".to_string(),
                    },
                ],
                priority: Priority::Normal,
                branch: "main".to_string(),
                context: String::new(),
            },
        )
        .unwrap();
        remove_options(&mut register, &id, &["b".to_string()]).unwrap();
        let options = &register.find(&id).unwrap().options;
        assert_eq!(options.len(), 2);
        assert!(options.iter().all(|o| o.letter != "b"));
    }

    #[test]
    fn remove_options_is_atomic_on_an_unknown_letter() {
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
        let err =
            remove_options(&mut register, &id, &["a".to_string(), "z".to_string()]).unwrap_err();
        assert!(err.contains('z'));
        assert_eq!(register.find(&id).unwrap().options.len(), 2);
    }

    #[test]
    fn remove_options_refuses_to_leave_nothing_offered() {
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
        assert!(remove_options(&mut register, &id, &["a".to_string(), "b".to_string()]).is_err());
        assert_eq!(register.find(&id).unwrap().options.len(), 2);
    }

    #[test]
    fn reorder_options_resorts_to_exactly_the_given_order() {
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
        reorder_options(&mut register, &id, &["b".to_string(), "a".to_string()]).unwrap();
        let options = &register.find(&id).unwrap().options;
        assert_eq!(options[0].letter, "b");
        assert_eq!(options[1].letter, "a");
    }

    #[test]
    fn reorder_options_refuses_a_set_that_does_not_match() {
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
        let err =
            reorder_options(&mut register, &id, &["a".to_string(), "z".to_string()]).unwrap_err();
        assert!(err.contains('z'));
        assert_eq!(register.find(&id).unwrap().options[0].letter, "a");
    }
}
