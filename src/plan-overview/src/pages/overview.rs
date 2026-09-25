// MODE: DEV
// PACKAGE: PROD
use super::{esc, link, section};
use crate::plan::derive::{derive_counts, Counts};
use crate::plan::mode::{derive_mode, Mode};
use crate::plan::state::State;

fn goal_progress(state: &State, goal_id: &str) -> (usize, usize) {
    let steps: Vec<_> = state
        .steps
        .iter()
        .filter(|step| step.goal == goal_id)
        .collect();
    let done = steps
        .iter()
        .filter(|step| matches!(step.status.as_str(), "completed" | "passed"))
        .count();
    (done, steps.len())
}

fn count_link(label: &str, value: u32, href: &str) -> String {
    format!(
        "<li>{}: {}</li>",
        link(label, href),
        link(&value.to_string(), href)
    )
}

pub fn render_overview(state: &State) -> String {
    let counts = derive_counts(state);
    let mode = derive_mode(state);
    let mode_name = match mode {
        Mode::Planning => "planning",
        Mode::Implementing => "implementing",
        Mode::Complete => "complete",
        Mode::Ambiguous => "ambiguous",
    };
    let mut goals = String::from("<ul class=\"goal-list\">");
    for goal in &state.goals {
        let (done, total) = goal_progress(state, &goal.id);
        let complete_class = if total > 0 && done == total {
            " goal-complete"
        } else {
            ""
        };
        goals.push_str(&format!(
            "<li class=\"goal-row{complete_class}\">{} <span class=\"progress-summary\">{done}/{total}</span></li>",
            link(&goal.id, &format!("#goal/{}", goal.id))
        ));
    }
    if state.goals.is_empty() {
        goals.push_str("<li>No goals recorded.</li>");
    }
    goals.push_str("</ul>");
    let blockers = if state.steps.iter().any(|step| step.status == "blocked") {
        state
            .steps
            .iter()
            .filter(|step| step.status == "blocked")
            .map(|step| {
                format!(
                    "<li>{} waits on a dependency</li>",
                    link(&step.unit, &format!("#unit/{}", step.unit))
                )
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        "<li>No blockers recorded.</li>".into()
    };
    format!("<article id=\"overview\"><h1>{}</h1><p class=\"mode\">Current phase: <strong>{}</strong></p><p>UI affected: {}. Generated at {} by {}.</p>{}<div class=\"dashboard\"><ul>{}{}{}</ul></div>{}{}</article>",
        esc(&state.identity.title), esc(mode_name), esc(&state.identity.ui_affected),
        esc(&state.generated_at), esc(&state.generated_by),
        section("Description", &format!("<p>{}</p>", esc(&state.identity.description))),
        count_link("Goals", counts.goals.total, "#goal/overview"),
        count_link("Steps", counts.steps.total, "#history"),
        count_link("Findings", counts.findings.total, "#history"),
        section("Blockers", &format!("<ul>{}</ul>", blockers)),
        section("Goals", &goals))
}

#[allow(dead_code)]
fn _counts_are_derived(_: &Counts) {}
