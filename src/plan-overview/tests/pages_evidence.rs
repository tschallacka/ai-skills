// MODE: DEV
use plan_overview::pages::{
    coverage::render_coverage, findings::render_finding, tests::render_test,
};
use plan_overview::plan::state::parse_state;

fn state() -> plan_overview::plan::state::State {
    parse_state(r#"{"identity":{"title":"Demo","uiAffected":"yes","reviewStatus":"approved","description":"d"},"goals":[],"steps":[{"goal":"","step":"test","unit":"W01","type":"test","target":"x","companion":"run command and inspect result","status":"passed","instructions":"i","criteria":"c"}],"edges":[],"testingMarks":[],"coverage":[{"outcome":"ship","units":"W01 W02 W03 W04 W05 W06 W07 W08"}],"findings":[{"id":"AR-1","item":"evidence","change":"correction","status":"open","workUnit":"W01","cycle":"current"}],"cycles":0,"reviewTarget":2,"generatedAt":"now","generatedBy":"test"}"#).unwrap()
}

#[test]
fn evidence_pages_render_procedure_and_all_coverage_ids() {
    let state = state();
    let test = render_test(&state, "W01");
    assert!(test.contains("run command and inspect result") && !test.contains("see companion"));
    let coverage = render_coverage(&state);
    for id in ["W01", "W02", "W03", "W04", "W05", "W06", "W07", "W08"] {
        assert!(coverage.contains(id));
    }
    assert!(render_finding(&state, "AR-1").contains("correction"));
}
