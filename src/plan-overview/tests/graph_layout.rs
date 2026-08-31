// MODE: DEV
use plan_overview::pages::graph::layout_nodes;
use plan_overview::plan::state::parse_state;

fn state(extra: &str) -> plan_overview::plan::state::State {
    parse_state(&format!(r#"{{"identity":{{"title":"Demo","uiAffected":"yes","reviewStatus":"approved","description":"d"}},"goals":[],"steps":[{{"goal":"","step":"a","unit":"W01","type":"source","target":"x","companion":null,"status":"open","instructions":"i","criteria":"c"}},{{"goal":"","step":"b","unit":"W02","type":"source","target":"y","companion":null,"status":"open","instructions":"i","criteria":"c"}}{}],"edges":[{{"from":"W01","to":"W02"}}],"testingMarks":[],"coverage":[],"findings":[],"cycles":0,"reviewTarget":2,"generatedAt":"now","generatedBy":"test"}}"#, extra)).unwrap()
}

#[test]
fn layout_is_stable_and_sorted() {
    let first = layout_nodes(&state(""));
    let second = layout_nodes(&state(""));
    assert_eq!(first, second);
    assert_eq!(first["W01"].0, 220);
    assert_eq!(first["W02"].0, 40);
}

#[test]
fn a_late_unit_does_not_displace_existing_rows() {
    let before = layout_nodes(&state(""));
    let after = layout_nodes(&state(
        r#",{"goal":"","step":"c","unit":"W99","type":"source","target":"z","companion":null,"status":"open","instructions":"i","criteria":"c"}"#,
    ));
    assert_eq!(before["W01"], after["W01"]);
    assert_eq!(before["W02"], after["W02"]);
}
