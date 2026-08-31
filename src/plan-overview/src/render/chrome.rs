// MODE: DEV
// PACKAGE: PROD
use super::router::Route;

pub fn breadcrumbs(route: &Route) -> String {
    let mut items = vec![("Plan", "#overview".to_string())];
    match route {
        Route::Overview { .. } => {}
        Route::Goal { id } => items.push((id.as_str(), format!("#goal/{id}"))),
        Route::Unit { id, goal } => {
            items.push((goal.as_str(), format!("#goal/{goal}")));
            items.push((id.as_str(), format!("#unit/{id}")));
        }
        Route::Test { id } => items.push((id.as_str(), format!("#test/{id}"))),
        Route::Finding { id } => items.push((id.as_str(), format!("#finding/{id}"))),
        Route::Coverage => items.push(("Coverage", "#coverage".into())),
        Route::History => items.push(("History", "#history".into())),
        Route::Graph => items.push(("Graph", "#graph".into())),
    }
    items
        .into_iter()
        .map(|(label, href)| format!("<a href=\"{href}\">{label}</a>"))
        .collect::<Vec<_>>()
        .join(" <span aria-hidden=\"true\">/</span> ")
}
