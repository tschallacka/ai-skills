// MODE: DEV
// PACKAGE: PROD
use plan_overview::plan::derive::{derive_counts, derive_geometry};
use plan_overview::plan::state::parse_state;
use plan_overview::plan::tree::read_plan_tree;

fn state_json() -> String {
    r#"{"identity":{"title":"Demo","uiAffected":"yes","reviewStatus":"approved","description":"text"},"goals":[{"id":"g1","outcome":"ship","testingRequirement":"yes"}],"steps":[{"goal":"g1","step":"01-first","unit":"W01","type":"source","target":"src/lib.rs","companion":null,"status":"completed","instructions":"read","criteria":"works"}],"edges":[],"testingMarks":[],"coverage":[],"findings":[{"id":"AR-01","item":"x","change":"y","status":"resolved","workUnit":"W01","cycle":"current"}],"cycles":2,"reviewTarget":2,"generatedAt":"now","generatedBy":"extract"}"#.into()
}

#[test]
fn parse_state_fixture_preserves_every_emitted_field() {
    let state = parse_state(&state_json()).unwrap();
    assert_eq!(state.identity.title, "Demo");
    assert_eq!(state.goals[0].testing_requirement, "yes");
    assert_eq!(state.steps[0].unit, "W01");
    assert_eq!(state.findings[0].work_unit, "W01");
    assert_eq!(state.review_target, 2);
}

#[test]
fn unknown_fields_are_reported() {
    let input = state_json().replace("\"cycles\":2", "\"unexpected\":true,\"cycles\":2");
    let error = parse_state(&input).unwrap_err();
    assert_eq!(error.unknown_fields, vec!["unexpected"]);
}

#[test]
fn truncation_reports_where_parsing_stopped() {
    let error = parse_state(&state_json()[..state_json().len() - 4]).unwrap_err();
    assert!(
        error.message.contains("line 1, column"),
        "{}",
        error.message
    );
}

#[test]
fn counts_and_geometry_are_zero_safe_and_consistent() {
    let mut state = parse_state(&state_json()).unwrap();
    state.steps.clear();
    state.findings.clear();
    let counts = derive_counts(&state);
    assert_eq!(
        counts.steps,
        plan_overview::plan::derive::Count {
            completed: 0,
            total: 0
        }
    );
    let geometry = derive_geometry(&counts);
    assert_eq!(geometry.donut_offset, geometry.donut_circumference);
    assert_eq!(geometry.work_offset, geometry.ring_circumference);
    assert!(geometry.donut_offset.is_finite());
}

#[test]
fn tree_reader_owns_document_contents_and_reports_optional_absence() {
    let root = std::env::temp_dir().join(format!("plan-overview-test-{}", std::process::id()));
    std::fs::create_dir_all(root.join("goal/steps")).unwrap();
    std::fs::write(root.join("plan-description.md"), "# Plan: demo").unwrap();
    std::fs::write(root.join("goal/goal.md"), "goal").unwrap();
    std::fs::write(root.join("goal/steps/step.md"), "step").unwrap();
    let tree = read_plan_tree(&root).unwrap();
    assert_eq!(tree.documents.len(), 3);
    assert!(tree.document("goal/steps/step.md").unwrap().contents == "step");
    assert!(tree
        .missing_optional
        .iter()
        .any(|path| path.ends_with("adversarial-review.md")));
    std::fs::remove_dir_all(root).unwrap();
}
