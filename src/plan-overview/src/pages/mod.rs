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

pub(crate) fn link(label: &str, href: &str) -> String {
    format!("<a href=\"{}\">{}</a>", esc(href), esc(label))
}

pub(crate) fn section(title: &str, body: &str) -> String {
    format!("<section><h2>{}</h2>{}</section>", esc(title), body)
}
