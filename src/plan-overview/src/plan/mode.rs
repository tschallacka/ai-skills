// MODE: DEV
// PACKAGE: PROD
use super::state::State;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Planning,
    Implementing,
    Complete,
    Ambiguous,
}

pub fn derive_mode(state: &State) -> Mode {
    // B343: update-plan-content.sh's -rv writes exactly two literal values,
    // "✅ approved" and "💤 pending" (see update-plan-content's own
    // update_review_status) -- never the bare word "approved" this used to
    // compare against, which made every real plan register as not-approved
    // regardless of its actual review status.
    let approved = state.identity.review_status == "✅ approved";
    let started = state.steps.iter().any(|step| step.status != "open");
    let all_passed = !state.steps.is_empty()
        && state
            .steps
            .iter()
            .all(|step| step.status == "completed" || step.status == "passed");
    if all_passed {
        return Mode::Complete;
    }
    if approved && state.steps.is_empty() {
        return Mode::Ambiguous;
    }
    match (approved, started) {
        (false, _) | (true, false) => Mode::Planning,
        (true, true) => Mode::Implementing,
    }
}
