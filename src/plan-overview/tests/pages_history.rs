// MODE: DEV
use plan_overview::pages::history::{render_discarded, render_history, render_superseded};
use plan_overview::plan::state::parse_state;

#[test]
fn history_pages_render_empty_and_current_history() {
    let state = parse_state(r#"{"identity":{"title":"Demo","uiAffected":"yes","reviewStatus":"approved","description":"d"},"goals":[],"steps":[],"edges":[],"testingMarks":[],"coverage":[],"findings":[],"cycles":1,"reviewTarget":2,"generatedAt":"now","generatedBy":"test"}"#).unwrap();
    assert!(render_history(&state).contains("Cycles recorded: 1"));
    assert!(render_superseded(&state).contains("History"));
    assert!(render_discarded(&state).contains("No discarded work"));
}
