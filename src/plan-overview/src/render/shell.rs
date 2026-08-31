// MODE: DEV
// PACKAGE: PROD
use crate::plan::mode::{derive_mode, Mode};
use crate::plan::state::State;

const TEMPLATE_PREFIX: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Plan overview</title></head><body><main id=\"app\">";
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
    let capacity = TEMPLATE_PREFIX.len()
        + page.len()
        + TEMPLATE_SUFFIX.len()
        + state_json.len()
        + TEMPLATE_END.len();
    let mut buffer = RenderBuffer::with_capacity(capacity);
    buffer.write_str(TEMPLATE_PREFIX);
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
