// MODE: DEV
// PACKAGE: PROD
use crate::plan::state::State;

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
    let raw = hash.strip_prefix('#').unwrap_or(hash);
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
