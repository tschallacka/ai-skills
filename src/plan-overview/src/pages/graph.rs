// MODE: DEV
// PACKAGE: PROD
use super::{esc, link, section};
use crate::plan::state::State;
use std::collections::BTreeMap;

pub fn layout_nodes(state: &State) -> BTreeMap<String, (u32, u32)> {
    let mut depth = BTreeMap::new();
    for step in &state.steps {
        depth.insert(step.unit.clone(), 0u32);
    }
    for _ in 0..state.steps.len() {
        for edge in &state.edges {
            let next = depth.get(&edge.to).copied().unwrap_or(0).saturating_add(1);
            let current = depth.entry(edge.from.clone()).or_insert(0);
            *current = (*current).max(next);
        }
    }
    let mut columns: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for (id, level) in depth {
        columns.entry(level).or_default().push(id);
    }
    let mut positions = BTreeMap::new();
    for (level, mut ids) in columns {
        ids.sort();
        for (row, id) in ids.into_iter().enumerate() {
            positions.insert(id, (level * 180 + 40, row as u32 * 70 + 40));
        }
    }
    positions
}

pub fn render_graph(state: &State) -> String {
    let positions = layout_nodes(state);
    let edges = state
        .edges
        .iter()
        .map(|edge| {
            format!(
                "<a href=\"#unit/{}\" class=\"edge\">{} -> {}</a>",
                esc(&edge.to),
                esc(&edge.from),
                esc(&edge.to)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let nodes = state
        .steps
        .iter()
        .map(|step| {
            let (x, y) = positions[&step.unit];
            format!(
                "<a href=\"#unit/{}\" class=\"node status-{}\" style=\"left:{}px;top:{}px\">{}</a>",
                esc(&step.unit),
                esc(&step.status),
                x,
                y,
                esc(&step.unit)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<article><h1>Dependency graph</h1>{}<div class=\"graph\">{}{}</div>{}</article>",
        section(
            "Legend",
            "<p>Each node label carries its status; edges are navigable.</p>"
        ),
        nodes,
        edges,
        render_graph_anomalies(state)
    )
}

pub fn render_graph_anomalies(state: &State) -> String {
    let orphans = state
        .steps
        .iter()
        .filter(|step| {
            !state
                .edges
                .iter()
                .any(|e| e.from == step.unit || e.to == step.unit)
        })
        .map(|step| link(&step.unit, &format!("#unit/{}", step.unit)))
        .collect::<Vec<_>>();
    let body = if orphans.is_empty() {
        "<li>None found.</li>".into()
    } else {
        orphans
            .into_iter()
            .map(|id| format!("<li>Orphan: {id}</li>"))
            .collect::<Vec<_>>()
            .join("")
    };
    section("Anomalies", &format!("<ul>{}</ul>", body))
}
