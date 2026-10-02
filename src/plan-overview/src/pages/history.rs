// MODE: DEV
// PACKAGE: PROD
use super::{esc, link, section};
use crate::plan::state::State;

pub fn render_history(state: &State) -> String {
    let findings = state
        .findings
        .iter()
        .map(|f| {
            let owner = if f.work_unit.is_empty() {
                "no unit".into()
            } else {
                link(&f.work_unit, &format!("#unit/{}", f.work_unit))
            };
            format!(
                "<li>{} <span class=\"status status-pending\">{}</span> ({}) — {} · unit {}</li>",
                link(&f.id, &format!("#finding/{}", f.id)),
                esc(&f.status),
                esc(&f.cycle),
                esc(&f.item),
                owner
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<article><h1>History &amp; findings</h1>{}{}</article>",
        section(
            "Current phase",
            &format!("<p>{}</p>", esc(&state.identity.review_status))
        ),
        section(
            "Adversarial-review findings",
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
