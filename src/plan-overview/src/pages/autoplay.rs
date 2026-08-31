// MODE: DEV
// PACKAGE: PROD
use crate::plan::mode::{derive_mode, Mode};
use crate::plan::state::State;

pub fn active_states(state: &State) -> Vec<(String, String)> {
    state
        .steps
        .iter()
        .filter(|step| step.status == "in_progress" || step.status == "active")
        .map(|step| (step.unit.clone(), step.goal.clone()))
        .collect()
}

pub fn autoplay_subject(state: &State) -> Option<String> {
    match derive_mode(state) {
        Mode::Implementing => active_states(state)
            .into_iter()
            .next()
            .map(|(unit, _)| unit),
        Mode::Planning => Some("plan construction".into()),
        Mode::Complete => None,
        Mode::Ambiguous => None,
    }
}

pub fn autoplay_status(state: &State) -> &'static str {
    match derive_mode(state) {
        Mode::Planning => "Following plan construction: units, edges and findings",
        Mode::Implementing => "Following active work",
        Mode::Complete => "Autoplay unavailable: nothing is being followed",
        Mode::Ambiguous => "Autoplay unavailable: plan state is ambiguous",
    }
}
