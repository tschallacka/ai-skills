// MODE: DEV
use plan_overview::plan::mode::{derive_mode, Mode};
use plan_overview::plan::state::parse_state;
use plan_overview::render::shell::render_mode_surface;

fn state(review: &str, status: &str, companion: &str) -> plan_overview::plan::state::State {
    parse_state(&format!(r#"{{"identity":{{"title":"Demo","uiAffected":"yes","reviewStatus":"{review}","description":"d"}},"goals":[],"steps":[{{"goal":"","step":"s","unit":"W01","type":"source","target":"x","companion":{companion},"status":"{status}","instructions":"i","criteria":"c"}}],"edges":[],"testingMarks":[],"coverage":[],"findings":[],"cycles":0,"reviewTarget":2,"generatedAt":"now","generatedBy":"test"}}"#)).unwrap()
}

#[test]
fn mode_derivation_covers_lifecycle_boundaries() {
    assert_eq!(
        derive_mode(&state("pending", "open", "null")),
        Mode::Planning
    );
    assert_eq!(
        derive_mode(&state("approved", "in_progress", "null")),
        Mode::Implementing
    );
    assert_eq!(
        derive_mode(&state("approved", "completed", "\"verified\"")),
        Mode::Complete
    );
}

#[test]
fn approved_without_steps_is_explicitly_ambiguous_and_readable() {
    let state = parse_state(r#"{"identity":{"title":"Demo","uiAffected":"yes","reviewStatus":"approved","description":"d"},"goals":[],"steps":[],"edges":[],"testingMarks":[],"coverage":[],"findings":[],"cycles":0,"reviewTarget":2,"generatedAt":"now","generatedBy":"test"}"#).unwrap();
    assert_eq!(derive_mode(&state), Mode::Ambiguous);
    let surface = render_mode_surface(&state);
    assert!(surface.contains("ambiguous") && surface.contains("clarification"));
}
