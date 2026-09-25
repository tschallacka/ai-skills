// MODE: DEV
// PACKAGE: PROD
use crate::pages;
use crate::plan::state::State;
use crate::render::shell::render_shell;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Overview { unresolved: Option<String> },
    Goal { id: String },
    Unit { id: String, goal: String },
    Finding { id: String },
    Test { id: String },
    Coverage,
    History,
    Graph,
}

impl Route {
    pub fn unresolved(&self) -> Option<&str> {
        match self {
            Self::Overview { unresolved } => unresolved.as_deref(),
            _ => None,
        }
    }
}

pub fn route(hash: &str, state: &State) -> Route {
    // Accepts either spelling this crate actually uses: a client-side hash
    // fragment ("#goal/foo", the historical SPA-router shape) or a real
    // request path ("/goal/foo", what a browser actually sends and what
    // serve.rs's per-request routing parses from the HTTP request line).
    let raw = hash
        .strip_prefix('#')
        .or_else(|| hash.strip_prefix('/'))
        .unwrap_or(hash);
    let mut parts = raw.split('/');
    let page = parts.next().unwrap_or("");
    let id = parts.next();
    if parts.next().is_some() || (page.is_empty() && id.is_some()) {
        return Route::Overview {
            unresolved: Some(hash.to_string()),
        };
    }
    match (page, id) {
        ("", None) | ("overview", None) => Route::Overview { unresolved: None },
        ("goal", Some(id)) if state.goals.iter().any(|goal| goal.id == id) => {
            Route::Goal { id: id.to_string() }
        }
        ("unit", Some(id)) => state
            .steps
            .iter()
            .find(|step| step.unit == id)
            .map(|step| Route::Unit {
                id: id.to_string(),
                goal: step.goal.clone(),
            })
            .unwrap_or_else(|| Route::Overview {
                unresolved: Some(hash.to_string()),
            }),
        ("finding", Some(id)) if state.findings.iter().any(|finding| finding.id == id) => {
            Route::Finding { id: id.to_string() }
        }
        ("test", Some(id))
            if state
                .steps
                .iter()
                .any(|step| step.unit == id && step.kind == "test") =>
        {
            Route::Test { id: id.to_string() }
        }
        ("coverage", None) => Route::Coverage,
        ("history", None) => Route::History,
        ("graph", None) => Route::Graph,
        _ => Route::Overview {
            unresolved: Some(hash.to_string()),
        },
    }
}

/// Renders the full HTML shell for whatever `path` (a hash fragment like
/// `#goal/foo` or a plain request path like `/goal/foo` -- route() treats
/// both identically) resolves to against the current state. The one place
/// that maps every Route variant to its page renderer, shared by the
/// `--out`/CLI path (main.rs) and the live server's per-request routing
/// (serve.rs), so the two never drift into rendering different content for
/// the same route.
pub fn render_page(state: &State, path: &str) -> String {
    let page = match route(path, state) {
        Route::Overview { .. } => pages::overview::render_overview(state),
        Route::Goal { id } => pages::goal::render_goal(state, &id),
        Route::Unit { id, .. } => pages::unit::render_unit(state, &id),
        Route::Finding { id } => pages::findings::render_finding(state, &id),
        Route::Test { id } => pages::tests::render_test(state, &id),
        Route::Coverage => pages::coverage::render_coverage(state),
        Route::History => pages::history::render_history(state),
        Route::Graph => pages::graph::render_graph(state),
    };
    render_shell(state, &page)
}
