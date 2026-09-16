// MODE: DEV
// PACKAGE: PROD
use crate::plan::mode::{derive_mode, Mode};
use crate::plan::state::State;

const STYLE: &str = "<style>\
:root{color-scheme:light dark;--bg:#fff;--fg:#1a1a1a;--muted:#6b6b6b;--border:#e0e0e0;--card:#f7f7f8;--accent:#2b6cb0;--done:#2f855a;--done-bg:#e6f4ea;--active:#b7791f;--active-bg:#fff6e0;--blocked:#c53030;--blocked-bg:#fde8e8;--pending:#6b6b6b;--pending-bg:#eef0f2}\
@media(prefers-color-scheme:dark){:root{--bg:#15171a;--fg:#e6e6e6;--muted:#9a9fa6;--border:#2c3036;--card:#1d2024;--accent:#7bb1ec;--done:#7fd99a;--done-bg:#123822;--active:#f0c674;--active-bg:#3a2e0d;--blocked:#f28b82;--blocked-bg:#3a1414;--pending:#9a9fa6;--pending-bg:#24272b}}\
*{box-sizing:border-box}\
body{margin:0;padding:1.5rem;background:var(--bg);color:var(--fg);font:16px/1.55 -apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif}\
main{max-width:920px;margin:0 auto}\
h1{font-size:1.6rem;margin:0 0 .5rem}\
h2{font-size:1.05rem;margin:1.4rem 0 .5rem;color:var(--muted);text-transform:uppercase;letter-spacing:.04em}\
p{margin:.4rem 0}\
a{color:var(--accent);text-decoration:none}\
a:hover{text-decoration:underline}\
article>p.mode{font-size:1.05rem}\
section{background:var(--card);border:1px solid var(--border);border-radius:8px;padding:.9rem 1.1rem;margin:.8rem 0}\
ul{padding-left:0;list-style:none;margin:.3rem 0}\
ul li{padding:.35rem 0;border-bottom:1px solid var(--border)}\
ul li:last-child{border-bottom:none}\
.goal-list li.goal-row{display:flex;justify-content:space-between;align-items:center;gap:.5rem}\
.goal-list li.goal-complete{opacity:.7}\
.progress-summary{font-variant-numeric:tabular-nums;color:var(--muted);font-size:.9rem}\
.unit-id{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-weight:600}\
.unit-kind{color:var(--muted);font-size:.85rem;font-style:italic}\
.depends-on{display:block;color:var(--muted);font-size:.8rem;margin-top:.2rem}\
pre{white-space:pre-wrap;word-wrap:break-word;font-family:inherit;margin:.4rem 0}\
.status{display:inline-block;font-size:.75rem;font-weight:600;padding:.15rem .55rem;border-radius:999px;text-transform:uppercase;letter-spacing:.03em;white-space:nowrap}\
.status-done{color:var(--done);background:var(--done-bg)}\
.status-active{color:var(--active);background:var(--active-bg)}\
.status-blocked{color:var(--blocked);background:var(--blocked-bg)}\
.status-pending{color:var(--pending);background:var(--pending-bg)}\
.dashboard{display:flex;gap:1.5rem;flex-wrap:wrap}\
.dashboard ul{flex:1;min-width:200px}\
dl{display:grid;grid-template-columns:max-content 1fr;gap:.25rem 1rem;margin:.3rem 0}\
dt{color:var(--muted)}\
dd{margin:0}\
#mode-surface{display:inline-block;padding:.3rem .7rem;border-radius:6px;background:var(--card);border:1px solid var(--border);text-transform:capitalize}\
</style>";
const TEMPLATE_PREFIX_HEAD: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Plan overview</title>";
const TEMPLATE_PREFIX_BODY: &str = "</head><body><main id=\"app\">";
const TEMPLATE_SUFFIX: &str = "</main><script id=\"plan-state\" type=\"application/json\">";
const TEMPLATE_END: &str = "</script><script src=\"nav.js\"></script></body></html>";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderBufferStats {
    pub allocations: usize,
    pub growths: usize,
}

pub struct RenderBuffer {
    output: String,
    stats: RenderBufferStats,
}

impl RenderBuffer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            output: String::with_capacity(capacity),
            stats: RenderBufferStats {
                allocations: 1,
                growths: 0,
            },
        }
    }

    pub fn write_str(&mut self, value: &str) {
        #[cfg(any(test, feature = "test-per-field-buffer"))]
        let before = self.output.capacity();
        #[cfg(feature = "test-per-field-buffer")]
        {
            self.stats.allocations += 1;
            let owned = value.to_owned();
            self.output.push_str(&owned);
        }
        #[cfg(not(feature = "test-per-field-buffer"))]
        self.output.push_str(value);
        #[cfg(any(test, feature = "test-per-field-buffer"))]
        if self.output.capacity() != before {
            self.stats.growths += 1;
        }
    }

    pub fn finish(self) -> String {
        self.output
    }

    pub fn stats(&self) -> RenderBufferStats {
        self.stats
    }
}

pub fn render_shell(state: &State, page: &str) -> String {
    render_shell_with_stats(state, page).0
}

pub fn render_shell_with_stats(state: &State, page: &str) -> (String, RenderBufferStats) {
    let state_json = serde_json::to_string(state)
        .expect("State is serializable")
        .replace('<', "\\u003c");
    let capacity = TEMPLATE_PREFIX_HEAD.len()
        + STYLE.len()
        + TEMPLATE_PREFIX_BODY.len()
        + page.len()
        + TEMPLATE_SUFFIX.len()
        + state_json.len()
        + TEMPLATE_END.len();
    let mut buffer = RenderBuffer::with_capacity(capacity);
    buffer.write_str(TEMPLATE_PREFIX_HEAD);
    buffer.write_str(STYLE);
    buffer.write_str(TEMPLATE_PREFIX_BODY);
    buffer.write_str(page);
    buffer.write_str(TEMPLATE_SUFFIX);
    buffer.write_str(&state_json);
    buffer.write_str(TEMPLATE_END);
    let stats = buffer.stats();
    (buffer.finish(), stats)
}

pub fn render_mode_surface(state: &State) -> String {
    let (mode, lead) = match derive_mode(state) {
        Mode::Planning => ("planning", "Soundness and plan construction"),
        Mode::Implementing => ("implementing", "Execution and active work"),
        Mode::Complete => ("complete", "Outcome"),
        Mode::Ambiguous => ("ambiguous", "Lifecycle needs clarification"),
    };
    format!("<div id=\"mode-surface\" data-mode=\"{mode}\"><strong>{mode}</strong><p>Leading surface: {lead}</p><p>Other surfaces remain reachable.</p></div>")
}

pub fn render_transition(direction: &str, page: &str) -> String {
    let direction = match direction {
        "back" => "back",
        _ => "forward",
    };
    format!("<div class=\"route-transition transition-{direction}\" data-transition=\"{direction}\">{page}</div>")
}
