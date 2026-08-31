// MODE: DEV
// PACKAGE: PROD
use super::{esc, link, section};
use crate::plan::state::State;

pub fn render_finding(state: &State, id: &str) -> String {
    let finding = match state.findings.iter().find(|finding| finding.id == id) {
        Some(value) => value,
        None => return "<article><h1>Finding not found</h1></article>".into(),
    };
    let owner = if finding.work_unit.is_empty() {
        "No owning unit.".into()
    } else {
        link(&finding.work_unit, &format!("#unit/{}", finding.work_unit))
    };
    format!(
        "<article><h1>Finding {}</h1>{}{}{}</article>",
        esc(&finding.id),
        section("Evidence", &format!("<p>{}</p>", esc(&finding.item))),
        section(
            "Required correction",
            &format!("<p>{}</p>", esc(&finding.change))
        ),
        section(
            "Status",
            &format!(
                "<p>{} | {}</p><p>Owner: {}</p>",
                esc(&finding.status),
                esc(&finding.cycle),
                owner
            )
        )
    )
}
