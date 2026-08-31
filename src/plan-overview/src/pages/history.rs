// MODE: DEV
// PACKAGE: PROD
use super::{esc, section};
use crate::plan::state::State;

pub fn render_history(state: &State) -> String {
    let findings = state
        .findings
        .iter()
        .map(|f| {
            format!(
                "<li>{}: {} ({})</li>",
                esc(&f.id),
                esc(&f.status),
                esc(&f.cycle)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<article><h1>History</h1>{}{}</article>",
        section(
            "Current phase",
            &format!("<p>{}</p>", esc(&state.identity.review_status))
        ),
        section(
            "Review cycles",
            &format!(
                "<p>Cycles recorded: {}</p><ul>{}</ul>",
                state.cycles,
                if findings.is_empty() {
                    "<li>No findings recorded.</li>".into()
                } else {
                    findings
                }
            )
        )
    )
}

pub fn render_superseded(state: &State) -> String {
    render_history(state)
}
pub fn render_discarded(_state: &State) -> String {
    section("Discarded work", "<p>No discarded work recorded.</p>")
}
