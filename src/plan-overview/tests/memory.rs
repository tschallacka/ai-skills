// MODE: DEV
// PACKAGE: PROD
use plan_overview::plan::state::{Identity, State};
use plan_overview::render::shell::render_shell_with_stats;

fn fixture() -> State {
    State {
        identity: Identity {
            title: "memory fixture".into(),
            ui_affected: "no".into(),
            review_status: "approved".into(),
            description: "fixture".into(),
        },
        goals: Vec::new(),
        steps: Vec::new(),
        edges: Vec::new(),
        testing_marks: Vec::new(),
        coverage: Vec::new(),
        findings: Vec::new(),
        cycles: 0,
        review_target: 0,
        generated_at: "now".into(),
        generated_by: "test".into(),
    }
}

#[test]
#[cfg(not(feature = "test-per-field-buffer"))]
fn production_render_buffer_allocates_once_without_growth() {
    let (_, stats) = render_shell_with_stats(&fixture(), "<p>fixture</p>");
    assert_eq!(stats.allocations, 1, "RenderBuffer allocations: {stats:?}");
    assert_eq!(stats.growths, 0, "RenderBuffer growths: {stats:?}");
}

#[cfg(feature = "test-per-field-buffer")]
#[test]
fn per_field_buffer_mutation_is_rejected() {
    // The feature is compiled by the normal all-features suite, but this
    // assertion is the explicit proof seam: PLAN_OVERVIEW_EXPECT_MEMORY_FAILURE=1
    // cargo test --features test-per-field-buffer --test memory
    if std::env::var_os("PLAN_OVERVIEW_EXPECT_MEMORY_FAILURE").is_none() {
        return;
    }

    let (_, stats) = render_shell_with_stats(&fixture(), "<p>fixture</p>");
    assert_eq!(
        stats.allocations, 1,
        "per-field allocation mutation (run with PLAN_OVERVIEW_EXPECT_MEMORY_FAILURE=1): {stats:?}"
    );
}
