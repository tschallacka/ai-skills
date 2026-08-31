// MODE: DEV
use plan_overview::pages::{coverage, findings, goal, graph, history, overview, tests, unit};
use plan_overview::plan::state::parse_state;

#[test]
fn every_state_field_presented() {
    let state = parse_state(r#"{"identity":{"title":"Demo","uiAffected":"yes","reviewStatus":"approved","description":"d"},"goals":[],"steps":[],"edges":[],"testingMarks":[],"coverage":[],"findings":[],"cycles":0,"reviewTarget":2,"generatedAt":"now","generatedBy":"test"}"#).unwrap();
    let rendered = [
        overview::render_overview(&state),
        coverage::render_coverage(&state),
        history::render_history(&state),
        graph::render_graph(&state),
        unit::render_unit_edges(&state, "missing"),
        goal::render_goal(&state, "missing"),
        findings::render_finding(&state, "missing"),
        tests::render_test(&state, "missing"),
    ]
    .join(" ");
    for value in [
        "Demo",
        "approved",
        "now",
        "test",
        "2",
        "No goals recorded.",
        "No coverage recorded.",
    ] {
        assert!(rendered.contains(value), "unconsumed field/value: {value}");
    }
}
