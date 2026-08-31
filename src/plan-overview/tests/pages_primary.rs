// MODE: DEV
use plan_overview::pages::{goal::render_goal, overview::render_overview, unit::render_unit};
use plan_overview::plan::state::parse_state;
use plan_overview::render::router::{route, Route};

fn state() -> plan_overview::plan::state::State {
    parse_state(r#"{"identity":{"title":"<Demo>","uiAffected":"yes","reviewStatus":"approved","description":"desc"},"goals":[{"id":"g1","outcome":"ship","testingRequirement":"yes"}],"steps":[{"goal":"g1","step":"instructions","unit":"W01","type":"source","target":"src/lib.rs","companion":null,"status":"open","instructions":"do it","criteria":"works"}],"edges":[],"testingMarks":[],"coverage":[],"findings":[],"cycles":0,"reviewTarget":2,"generatedAt":"now","generatedBy":"test"}"#).unwrap()
}

#[test]
fn primary_pages_render_required_fields_and_safe_links() {
    let state = state();
    let overview = render_overview(&state);
    assert!(overview.contains("Goals") && overview.contains("Current phase"));
    assert!(overview.contains("&lt;Demo&gt;") && !overview.contains("<Demo>"));
    assert!(render_goal(&state, "g1").contains("ship"));
    let unit = render_unit(&state, "W01");
    assert!(
        unit.contains("src/lib.rs")
            && unit.contains("Instructions")
            && unit.contains("No dependencies")
    );
    assert_eq!(
        route("#unit/W01", &state),
        Route::Unit {
            id: "W01".into(),
            goal: "g1".into()
        }
    );
}

#[test]
fn empty_relationships_are_explicit() {
    assert!(render_unit(&state(), "W01").contains("No dependencies recorded."));
}
