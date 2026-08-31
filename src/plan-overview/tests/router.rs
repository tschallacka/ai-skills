// MODE: DEV
// PACKAGE: PROD
use plan_overview::plan::state::parse_state;
use plan_overview::render::router::{route, Route};

fn state() -> plan_overview::plan::state::State {
    parse_state(r#"{"identity":{"title":"Demo","uiAffected":"yes","reviewStatus":"approved","description":"text"},"goals":[{"id":"g1","outcome":"ship","testingRequirement":"yes"}],"steps":[{"goal":"g1","step":"01-test","unit":"W01","type":"test","target":"x","companion":null,"status":"open","instructions":"x","criteria":"x"}],"edges":[],"testingMarks":[],"coverage":[],"findings":[{"id":"AR-01","item":"x","change":"y","status":"open","workUnit":"W01","cycle":"current"}],"cycles":0,"reviewTarget":2,"generatedAt":"now","generatedBy":"test"}"#).unwrap()
}

#[test]
fn route_table() {
    let state = state();
    assert_eq!(
        route("#overview", &state),
        Route::Overview { unresolved: None }
    );
    assert_eq!(route("#goal/g1", &state), Route::Goal { id: "g1".into() });
    assert_eq!(
        route("#unit/W01", &state),
        Route::Unit {
            id: "W01".into(),
            goal: "g1".into()
        }
    );
    assert_eq!(
        route("#finding/AR-01", &state),
        Route::Finding { id: "AR-01".into() }
    );
    assert_eq!(route("#test/W01", &state), Route::Test { id: "W01".into() });
    assert_eq!(route("#coverage", &state), Route::Coverage);
    assert_eq!(route("#history", &state), Route::History);
    assert_eq!(route("#graph", &state), Route::Graph);
    for hash in ["#missing", "#unit/W99", "#unit/W01/extra", "#unit|W01"] {
        assert_eq!(route(hash, &state).unresolved(), Some(hash));
    }
}
