// MODE: DEV
// PACKAGE: PROD
use super::{esc, link, section};
use crate::plan::state::State;

pub fn render_test(state: &State, id: &str) -> String {
    let step = match state
        .steps
        .iter()
        .find(|step| step.unit == id && step.kind == "test")
    {
        Some(value) => value,
        None => return "<article><h1>Test not found</h1></article>".into(),
    };
    let procedure = step
        .companion
        .as_deref()
        .unwrap_or("No testing companion procedure recorded.");
    format!(
        "<article><h1>Test {}</h1>{}{}{} </article>",
        esc(id),
        section("Procedure", &format!("<pre>{}</pre>", esc(procedure))),
        section("Status", &format!("<p>{}</p>", esc(&step.status))),
        section(
            "Proves",
            &format!(
                "<p>{}</p>",
                link(&step.unit, &format!("#unit/{}", step.unit))
            )
        )
    )
}
