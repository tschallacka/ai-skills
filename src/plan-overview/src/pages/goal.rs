// MODE: DEV
// PACKAGE: PROD
use super::{esc, link, section, status_badge};
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
    let goal_steps: Vec<_> = state.steps.iter().filter(|step| step.goal == id).collect();
    let done = goal_steps
        .iter()
        .filter(|step| matches!(step.status.as_str(), "completed" | "passed"))
        .count();
    let total = goal_steps.len();
    let unit_ids: Vec<&str> = goal_steps.iter().map(|step| step.unit.as_str()).collect();
    let units: Vec<String> = goal_steps
        .iter()
        .map(|step| {
            let deps: Vec<String> = state
                .edges
                .iter()
                .filter(|edge| edge.from == step.unit)
                .map(|edge| link(&edge.to, &format!("#unit/{}", edge.to)))
                .collect();
            let depends_on = if deps.is_empty() {
                String::new()
            } else {
                format!(
                    " <span class=\"depends-on\">depends on {}</span>",
                    deps.join(", ")
                )
            };
            format!(
                "<li><span class=\"unit-id\">{}</span> {} <span class=\"unit-kind\">{}</span> {}{depends_on}</li>",
                link(&step.unit, &format!("#unit/{}", step.unit)),
                esc(&step.step),
                esc(&step.kind),
                status_badge(&step.status)
            )
        })
        .collect();
    let units = if units.is_empty() {
        "<li>No owned units.</li>".into()
    } else {
        units.join("")
    };
    let goal_findings: Vec<String> = state
        .findings
        .iter()
        .filter(|finding| unit_ids.contains(&finding.work_unit.as_str()))
        .map(|finding| {
            format!(
                "<li>{} <span class=\"status status-pending\">{}</span> — {} (unit {})</li>",
                link(&finding.id, &format!("#finding/{}", finding.id)),
                esc(&finding.status),
                esc(&finding.item),
                link(&finding.work_unit, &format!("#unit/{}", finding.work_unit)),
            )
        })
        .collect();
    let findings_body = if goal_findings.is_empty() {
        "<p>No adversarial-review findings recorded against this goal's own units.</p>".into()
    } else {
        format!("<ul>{}</ul>", goal_findings.join(""))
    };
    format!(
        "<article><h1>Goal {}</h1><p class=\"progress-summary\">{done}/{total} steps done</p>{}{}{}{}</article>",
        esc(&goal.id),
        section("Outcome", &format!("<p>{}</p>", esc(&goal.outcome))),
        section(
            "Testing requirement",
            &format!("<p>{}</p>", esc(&goal.testing_requirement))
        ),
        section("Owned work units", &format!("<ul>{}</ul>", units)),
        section("Findings", &findings_body)
    )
}
