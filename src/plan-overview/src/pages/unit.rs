// MODE: DEV
// PACKAGE: PROD
use super::{esc, link, section};
use crate::plan::state::State;

pub fn render_unit(state: &State, id: &str) -> String {
    let step = match state.steps.iter().find(|step| step.unit == id) {
        Some(step) => step,
        None => {
            return format!(
                "<article><h1>Unit not found</h1><p>{}</p></article>",
                esc(id)
            )
        }
    };
    format!("<article><h1>Unit {}</h1>{}{}{}</article>", esc(id),
        section("Change target", &format!("<dl><dt>File</dt><dd>{}</dd><dt>Primary symbol</dt><dd>{}</dd><dt>Type</dt><dd>{}</dd></dl>", esc(&step.target), esc(&step.step), esc(&step.kind))),
        section("Instructions", &format!("<p>{}</p><p>Acceptance criteria: {}</p>", esc(&step.instructions), esc(&step.criteria))),
        render_unit_edges(state, id))
}

pub fn render_unit_edges(state: &State, id: &str) -> String {
    let deps: Vec<&str> = state
        .edges
        .iter()
        .filter(|edge| edge.from == id)
        .map(|edge| edge.to.as_str())
        .collect();
    let dependents: Vec<&str> = state
        .edges
        .iter()
        .filter(|edge| edge.to == id)
        .map(|edge| edge.from.as_str())
        .collect();
    let list = |title: &str, ids: Vec<&str>| {
        let body = if ids.is_empty() {
            format!("<li>No {} recorded.</li>", title.to_lowercase())
        } else {
            ids.into_iter()
                .map(|target| format!("<li>{}</li>", link(target, &format!("#unit/{}", target))))
                .collect::<Vec<_>>()
                .join("")
        };
        section(title, &format!("<ul>{}</ul>", body))
    };
    format!(
        "<div class=\"edges\">{}{}</div>",
        list("Dependencies", deps),
        list("Dependents", dependents)
    )
}
