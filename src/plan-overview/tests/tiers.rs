// MODE: DEV
// PACKAGE: PROD
use plan_overview::render::tiers::{emit_tier_table, Tier, TierController, TierPolicy};

#[test]
fn tier_selection_has_hysteresis() {
    let mut controller = TierController::new();
    assert_eq!(controller.tier(), Tier::Full);
    for _ in 0..3 {
        controller.observe(25.0);
    }
    assert_eq!(controller.tier(), Tier::Reduced);
    controller.observe(18.0);
    assert_eq!(controller.tier(), Tier::Reduced);
    for _ in 0..3 {
        controller.observe(15.0);
    }
    assert_eq!(controller.tier(), Tier::Full);
    for sample in [25.0, 15.0, 25.0, 15.0, 25.0] {
        controller.observe(sample);
    }
    assert_eq!(controller.tier(), Tier::Full);
    assert!(emit_tier_table().contains("hysteresis"));
}

#[test]
fn sustained_miss_can_reach_minimal_without_flapping() {
    let mut controller = TierController::with_policy(TierPolicy {
        down_ms: 20.0,
        up_ms: 10.0,
        hysteresis_samples: 2,
    });
    for _ in 0..4 {
        controller.observe(30.0);
    }
    assert_eq!(controller.tier(), Tier::Minimal);
}
