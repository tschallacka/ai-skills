// MODE: DEV
// PACKAGE: PROD
pub mod autoplay;
pub mod coverage;
pub mod findings;
pub mod goal;
pub mod graph;
pub mod history;
pub mod overview;
pub mod tests;
pub mod unit;

pub(crate) fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Every call site names its target the historical hash-fragment way
/// (`#goal/foo`), translated here to a real request path (`/goal/foo`) --
/// a `#fragment` is client-side only and never reaches the server at all, so
/// a plain path is what makes the link an actual, working navigation against
/// serve.rs's per-request routing (see render::router::render_page).
pub(crate) fn link(label: &str, href: &str) -> String {
    let href = match href.strip_prefix('#') {
        Some(rest) => format!("/{rest}"),
        None => href.to_string(),
    };
    format!("<a href=\"{}\">{}</a>", esc(&href), esc(label))
}

pub(crate) fn section(title: &str, body: &str) -> String {
    format!("<section><h2>{}</h2>{}</section>", esc(title), body)
}

/// A small colored badge for a step's status, reused everywhere a step/unit
/// is listed so "which steps are done" is visible without opening each unit.
pub(crate) fn status_badge(status: &str) -> String {
    let class = match status {
        "completed" | "passed" | "verified" => "status-done",
        "in_progress" => "status-active",
        "blocked" => "status-blocked",
        _ => "status-pending",
    };
    format!(
        "<span class=\"status {class}\">{}</span>",
        esc(status.replace('_', " ").as_str())
    )
}
