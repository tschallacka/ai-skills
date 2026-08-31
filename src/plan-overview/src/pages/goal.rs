// MODE: DEV
// PACKAGE: PROD
use super::{esc, link, section};
use crate::plan::state::State;

pub fn render_goal(state: &State, id: &str) -> String {
    let goal = match state.goals.iter().find(|goal| goal.id == id) {
        Some(goal) => goal,
        None => {
            return format!(
                "<article><h1>Goal not found</h1><p>{}</p></article>",
                esc(id)
            )
        }
    };
    let units: Vec<String> = state
        .steps
        .iter()
        .filter(|step| step.goal == id)
        .map(|step| {
            format!(
                "<li>{}</li>",
                link(&step.unit, &format!("#unit/{}", step.unit))
            )
        })
        .collect();
    let units = if units.is_empty() {
        "<li>No owned units.</li>".into()
    } else {
        units.join("")
    };
    format!(
        "<article><h1>Goal {}</h1>{}{}{}</article>",
        esc(&goal.id),
        section("Outcome", &format!("<p>{}</p>", esc(&goal.outcome))),
        section(
            "Testing requirement",
            &format!("<p>{}</p>", esc(&goal.testing_requirement))
        ),
        section("Owned work units", &format!("<ul>{}</ul>", units))
    )
}
