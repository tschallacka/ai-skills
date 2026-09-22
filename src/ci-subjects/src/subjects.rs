// MODE: DEV
// PACKAGE: PROD

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Subjects {
    pub rjq: bool,
    pub chat: bool,
    pub plan_crypt: bool,
    pub planning_commands: bool,
    pub editor: bool,
    pub installer: bool,
}

impl Subjects {
    fn all_true() -> Self {
        Subjects {
            rjq: true,
            chat: true,
            plan_crypt: true,
            planning_commands: true,
            editor: true,
            installer: true,
        }
    }
}

/// `scope == "none"` -> every flag false; `scope == "selective"` -> the
/// crate-matching loop decides; anything else (including `"full"`, an empty
/// string, or an unrecognized value) -> every flag true, a fail-safe
/// default.
pub fn decide(scope: &str, crates: &str) -> Subjects {
    match scope {
        "none" => Subjects::default(),
        "selective" => {
            let mut subjects = Subjects::default();
            for crate_name in split_ifs(crates) {
                apply(&mut subjects, crate_name);
            }
            subjects
        }
        _ => Subjects::all_true(),
    }
}

/// AR-83: splits on space, tab, and newline only -- NOT every ASCII
/// whitespace character (vertical tab, form feed, and carriage return are
/// NOT separators).
fn split_ifs(s: &str) -> impl Iterator<Item = &str> {
    s.split([' ', '\t', '\n']).filter(|word| !word.is_empty())
}

/// First-match-wins.
fn apply(subjects: &mut Subjects, crate_name: &str) {
    if crate_name == "rjq" {
        subjects.rjq = true;
    } else if crate_name == "plan-crypt" {
        subjects.plan_crypt = true;
    } else if crate_name.starts_with("chat-") {
        subjects.chat = true;
    } else if crate_name.starts_with("ai-text-editor") {
        subjects.editor = true;
    } else if crate_name.starts_with("installer") {
        subjects.installer = true;
    } else {
        subjects.planning_commands = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(v: bool) -> Subjects {
        Subjects {
            rjq: v,
            chat: v,
            plan_crypt: v,
            planning_commands: v,
            editor: v,
            installer: v,
        }
    }

    #[test]
    fn full_builds_everything() {
        assert_eq!(decide("full", ""), all(true));
    }

    #[test]
    fn an_unknown_scope_builds_everything() {
        assert_eq!(decide("wat", ""), all(true));
    }

    #[test]
    fn an_empty_scope_builds_everything() {
        assert_eq!(decide("", ""), all(true));
    }

    #[test]
    fn none_builds_nothing() {
        assert_eq!(decide("none", "anything"), all(false));
    }

    #[test]
    fn selective_with_no_crates_builds_nothing() {
        assert_eq!(decide("selective", ""), all(false));
    }

    #[test]
    fn rjq_alone() {
        let s = decide("selective", "rjq");
        assert!(
            s.rjq && !s.chat && !s.plan_crypt && !s.planning_commands && !s.editor && !s.installer
        );
    }

    #[test]
    fn plan_crypt_alone() {
        let s = decide("selective", "plan-crypt");
        assert!(
            !s.rjq && !s.chat && s.plan_crypt && !s.planning_commands && !s.editor && !s.installer
        );
    }

    #[test]
    fn a_single_chat_crate() {
        let s = decide("selective", "chat-proto");
        assert!(
            s.chat && !s.rjq && !s.plan_crypt && !s.planning_commands && !s.editor && !s.installer
        );
    }

    #[test]
    fn every_chat_crate_together_is_the_same_as_one() {
        let s = decide("selective", "chat-proto chat-server-rs chat-client-rs");
        assert!(
            s.chat && !s.rjq && !s.plan_crypt && !s.planning_commands && !s.editor && !s.installer
        );
    }

    #[test]
    fn an_editor_crate() {
        let s = decide("selective", "ai-text-editor-mcp");
        assert!(
            s.editor && !s.rjq && !s.chat && !s.plan_crypt && !s.planning_commands && !s.installer
        );
    }

    #[test]
    fn an_installer_crate() {
        let s = decide("selective", "installer");
        assert!(
            s.installer && !s.rjq && !s.chat && !s.plan_crypt && !s.planning_commands && !s.editor
        );
    }

    #[test]
    fn every_installer_crate_together_is_the_same_as_one() {
        let s = decide(
            "selective",
            "installer installer-platform installer-release",
        );
        assert!(
            s.installer && !s.rjq && !s.chat && !s.plan_crypt && !s.planning_commands && !s.editor
        );
    }

    #[test]
    fn a_planning_crate_falls_to_the_catch_all() {
        let s = decide("selective", "planning-core");
        assert!(
            s.planning_commands && !s.rjq && !s.chat && !s.plan_crypt && !s.editor && !s.installer
        );
    }

    #[test]
    fn an_unrecognized_crate_falls_to_the_catch_all() {
        let s = decide("selective", "some-new-crate");
        assert!(
            s.planning_commands && !s.rjq && !s.chat && !s.plan_crypt && !s.editor && !s.installer
        );
    }

    #[test]
    fn several_subjects_at_once() {
        let s = decide("selective", "rjq chat-proto plan-overview installer");
        assert!(
            s.rjq && s.chat && s.planning_commands && s.installer && !s.plan_crypt && !s.editor
        );
    }

    #[test]
    fn vertical_tab_form_feed_and_carriage_return_are_not_separators() {
        // AR-83: only space, tab, and newline split; other whitespace bytes
        // stay embedded in whatever word they fall within.
        let weird = "some\u{000B}crate";
        let s = decide("selective", weird);
        assert!(
            s.planning_commands,
            "the whole odd string is one word, falling to the catch-all"
        );
        assert!(!s.rjq && !s.chat && !s.plan_crypt && !s.editor && !s.installer);
    }

    #[test]
    fn tabs_and_newlines_do_split() {
        let s = decide("selective", "rjq\tplan-crypt\ninstaller");
        assert!(s.rjq && s.plan_crypt && s.installer);
    }
}
