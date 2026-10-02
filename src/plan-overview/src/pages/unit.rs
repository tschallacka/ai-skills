// MODE: DEV
// PACKAGE: PROD
use super::{esc, link, section, status_badge};
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
    let testing_section = match step.testing_procedure.as_deref() {
        Some(procedure) if !procedure.is_empty() => section(
            "Testing companion",
            &format!("<pre>{}</pre>", esc(procedure)),
        ),
        _ => section(
            "Testing companion",
            "<p>No testing companion recorded for this unit.</p>",
        ),
    };
    format!(
        "<article><h1>Unit {} {}</h1><p>{} · step {}</p>{}{}{}{}</article>",
        esc(id),
        status_badge(&step.status),
        link(&step.goal, &format!("#goal/{}", step.goal)),
        esc(&step.step),
        section(
            "Change target",
            &format!(
                "<dl><dt>File</dt><dd>{}</dd><dt>Type</dt><dd>{}</dd></dl>",
                esc(&step.target),
                esc(&step.kind)
            )
        ),
        section(
            "Instructions",
            &format!(
                "<p>{}</p><p>Acceptance criteria: {}</p>",
                esc(&step.instructions),
                esc(&step.criteria)
            )
        ),
        testing_section,
        render_unit_edges(state, id)
    )
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
