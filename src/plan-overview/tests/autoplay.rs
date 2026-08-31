// MODE: DEV
use plan_overview::pages::autoplay::{active_states, autoplay_status, autoplay_subject};
use plan_overview::plan::mode::{derive_mode, Mode};
use plan_overview::plan::state::parse_state;

fn parse(status: &str, review: &str) -> plan_overview::plan::state::State {
    parse_state(&format!(r#"{{"identity":{{"title":"Demo","uiAffected":"yes","reviewStatus":"{review}","description":"d"}},"goals":[],"steps":[{{"goal":"","step":"s","unit":"W01","type":"source","target":"x","companion":null,"status":"{status}","instructions":"i","criteria":"c"}}],"edges":[],"testingMarks":[],"coverage":[],"findings":[],"cycles":0,"reviewTarget":2,"generatedAt":"now","generatedBy":"test"}}"#)).unwrap()
}

#[test]
fn active_state_and_subject_selection() {
    let active = parse("in_progress", "approved");
    assert_eq!(active_states(&active).len(), 1);
    assert_eq!(autoplay_subject(&active), Some("W01".into()));
    let complete = parse("completed", "approved");
    assert!(active_states(&complete).is_empty());
    assert_eq!(derive_mode(&complete), Mode::Complete);
    assert_eq!(autoplay_subject(&complete), None);
}

#[test]
fn planning_and_complete_modes_explain_availability() {
    let planning = parse("open", "pending");
    assert_eq!(
        autoplay_subject(&planning),
        Some("plan construction".into())
    );
    assert!(autoplay_status(&planning).contains("units"));
    let complete = parse("completed", "approved");
    assert!(autoplay_status(&complete).contains("unavailable"));
}
