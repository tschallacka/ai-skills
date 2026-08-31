// MODE: DEV
// PACKAGE: PROD
use super::{esc, link, section};
use crate::plan::state::State;

pub fn render_coverage(state: &State) -> String {
    let rows = state
        .coverage
        .iter()
        .map(|row| {
            let ids: Vec<String> = row
                .units
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|id| !id.is_empty())
                .map(|id| link(id, &format!("#unit/{}", id)))
                .collect();
            let body = if ids.is_empty() {
                "<strong class=\"uncovered\">Uncovered</strong>".into()
            } else {
                ids.join(" ")
            };
            format!("<tr><th>{}</th><td>{}</td></tr>", esc(&row.outcome), body)
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<article><h1>Coverage</h1>{}</article>",
        section(
            "Definition of done",
            &format!(
                "<table>{}</table>",
                if rows.is_empty() {
                    "<tr><td>No coverage recorded.</td></tr>".into()
                } else {
                    rows
                }
            )
        )
    )
}
