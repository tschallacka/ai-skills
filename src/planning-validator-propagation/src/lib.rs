// MODE: DEV
// PACKAGE: PROD
//! Completion and propagation validation formerly provided by
//! `validate-plan-propagation-lib.sh`.

use planning_table::table_cell;
use planning_validator_common::Findings;
use planning_validator_inventory::{Inventory, Unit};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn validate_completion(
    plan: &Path,
    inventory: &Inventory,
    complete: bool,
    findings: &mut Findings,
) {
    if !complete {
        return;
    }
    let plan_progress = plan.join("progress.md");
    let progress = fs::read_to_string(&plan_progress).unwrap_or_default();
    if !plan_progress.is_file() {
        findings.fail("Completion requires plan-level progress.md");
    }
    for (goal, ids) in &inventory.goals {
        if !progress
            .lines()
            .any(|line| table_cell(line, 2) == *goal && table_cell(line, 4) == "✅ completed")
        {
            findings.fail(format!("{goal} is not completed in plan progress"));
        }
        let goal_progress_path = plan.join(goal).join("progress.md");
        let goal_progress = fs::read_to_string(&goal_progress_path).unwrap_or_default();
        for id in ids {
            let Some(unit) = inventory.units.iter().find(|unit| &unit.id == id) else {
                continue;
            };
            if !goal_progress_path.is_file() {
                findings.fail(format!(
                    "{id} completion requires {}",
                    goal_progress_path.display()
                ));
            } else if !goal_progress.lines().any(|line| {
                table_cell(line, 3) == unit.step && table_cell(line, 5) == "✅ completed"
            }) {
                findings.fail(format!("{id} is not completed in {goal} progress"));
            }
        }
    }
}

pub fn validate_reach(plan: &Path, inventory: &Inventory, findings: &mut Findings) {
    for verifier in inventory
        .units
        .iter()
        .filter(|unit| unit.kind == "verification")
    {
        let text = read_step(plan, verifier);
        for named in ids_in(&text) {
            if named == verifier.id {
                continue;
            }
            let Some(target) = inventory.units.iter().find(|unit| unit.id == named) else {
                continue;
            };
            if target.goal != verifier.goal {
                continue;
            }
            if !reachable(inventory, &verifier.id, &target.id)
                && !reachable(inventory, &target.id, &verifier.id)
            {
                findings.fail(format!("{} is a verification unit that grades {} but has no dependency path to it; add a dependency edge", verifier.id, target.id));
            }
        }
    }
}

/// A step's Handoff prose must not promise a later unit something the
/// dependency graph does not order: a consumer that reads the Handoff as a
/// licence to run early needs an edge, not a sentence. Ported from
/// `plan_validate_propagation_handoff` / `plan_handoff_units` in
/// `validate-plan-propagation-lib.sh`, which this crate had not yet carried
/// over (found as a parity gap alongside B335, goal 4 of
/// planning-skill-rustify).
/// The WNN ids a step's Handoff paragraphs actually claim, paragraph by
/// paragraph (blank-line delimited, each paragraph's lines flattened to one
/// line) -- a paragraph whose flattened text carries a history marker is
/// dropped whole, so a corrective paragraph restating an old, disproven claim
/// never re-triggers the ordering check it was written to retract. Mirrors
/// `plan_handoff_units`'s own awk-based paragraph buffering exactly.
fn handoff_units(plan: &Path, unit: &Unit) -> Vec<String> {
    let section = read_section(plan, unit, "## Handoff");
    let mut paragraphs = Vec::new();
    let mut current = String::new();
    for line in section.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                paragraphs.push(std::mem::take(&mut current));
            }
        } else if current.is_empty() {
            current.push_str(line);
        } else {
            current.push(' ');
            current.push_str(line);
        }
    }
    if !current.is_empty() {
        paragraphs.push(current);
    }
    let mut result = Vec::new();
    for paragraph in paragraphs {
        let lower = paragraph.to_ascii_lowercase();
        if planning_validator_stale::STALE_MARKERS
            .iter()
            .any(|marker| lower.contains(marker))
        {
            continue;
        }
        for id in ids_in(&paragraph) {
            if !result.contains(&id) {
                result.push(id);
            }
        }
    }
    result
}

pub fn validate_handoff(plan: &Path, inventory: &Inventory, findings: &mut Findings) {
    for unit in &inventory.units {
        for named in handoff_units(plan, unit) {
            if named == unit.id {
                continue;
            }
            if !inventory.units.iter().any(|other| other.id == named) {
                // A WNN outside this plan's inventory is a cross-plan
                // reference, correct prose, not a claim this graph could
                // ever order.
                continue;
            }
            if !reachable(inventory, &unit.id, &named) && !reachable(inventory, &named, &unit.id) {
                findings.warn(format!(
                    "{} handoff names {named}, but neither has a dependency path to the other; add the ordering edge or correct the handoff",
                    unit.id
                ));
            }
        }
    }
}

pub fn validate_companions(plan: &Path, inventory: &Inventory, findings: &mut Findings) {
    for unit in &inventory.units {
        let path = plan
            .join(&unit.goal)
            .join("steps")
            .join(format!("{}-testing.md", unit.step));
        if !path.is_file() {
            continue;
        }
        let deps = dependencies(&unit.depends);
        for named in ids_in(&fs::read_to_string(&path).unwrap_or_default()) {
            if named == unit.id {
                continue;
            }
            let Some(other) = inventory
                .units
                .iter()
                .find(|candidate| candidate.id == named)
            else {
                continue;
            };
            if other.goal == unit.goal && matches!(other.kind.as_str(), "test" | "verification") {
                continue;
            }
            if !deps.contains(&named) {
                findings.warn(format!("{} companion references {}, which {} neither owns nor depends on; update the companion or add the dependency edge", unit.id, named, unit.id));
            }
        }
    }
}

pub fn validate_leaves(inventory: &Inventory, findings: &mut Findings) {
    let dependents: HashSet<&str> = inventory
        .units
        .iter()
        .flat_map(|unit| {
            dependencies(&unit.depends).into_iter().filter_map(|id| {
                inventory
                    .units
                    .iter()
                    .find(|candidate| candidate.id == id)
                    .map(|candidate| candidate.id.as_str())
            })
        })
        .collect();
    for ids in inventory.goals.values() {
        if !ids.iter().any(|id| {
            inventory
                .units
                .iter()
                .any(|unit| &unit.id == id && unit.kind == "verification")
        }) {
            continue;
        }
        for id in ids {
            let Some(unit) = inventory.units.iter().find(|unit| &unit.id == id) else {
                continue;
            };
            if unit.kind != "verification" && !dependents.contains(id.as_str()) {
                findings.warn(format!("{id} is a graph leaf in a goal that owns a verification unit; nothing depends on it, so nothing verifies its output"));
            }
        }
    }
}

/// Lines whose edit-intent verb makes a ::-symbol on them worth checking for
/// ownership. Matches `plan_validate_propagation_symbols_unit`'s own
/// `grep -iE '(create|add|implement|edit|change|update|modify|rewrite|replace|override)'`.
const EDIT_INTENT_VERBS: &[&str] = &[
    "create",
    "add",
    "implement",
    "edit",
    "change",
    "update",
    "modify",
    "rewrite",
    "replace",
    "override",
];

pub fn validate_symbols(plan: &Path, inventory: &Inventory, findings: &mut Findings) {
    let prefixes = inventory
        .units
        .iter()
        .filter_map(|unit| {
            if unit.file == "N/A" || unit.file.is_empty() {
                return None;
            }
            Some(match unit.file.split_once('\\') {
                Some((root, _)) => root.to_owned(),
                None if unit.file.starts_with("app/") || unit.file.starts_with("vendor/") => {
                    unit.file.split('/').next().unwrap_or_default().to_owned()
                }
                None => std::path::Path::new(&unit.file)
                    .parent()
                    .map(|parent| parent.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            })
        })
        .filter(|prefix| !prefix.is_empty())
        .collect::<Vec<_>>();
    for unit in &inventory.units {
        let text = read_section(plan, unit, "## Instructions");
        let edit_lines = text
            .lines()
            .filter(|line| {
                let lower = line.to_ascii_lowercase();
                EDIT_INTENT_VERBS.iter().any(|verb| lower.contains(verb))
            })
            .collect::<Vec<_>>()
            .join("\n");
        if edit_lines.is_empty() {
            continue;
        }
        for token in symbol_tokens(&edit_lines) {
            if token.ends_with("::class") {
                continue;
            }
            let class = token.split("::").next().unwrap_or_default();
            let short = class.rsplit('\\').next().unwrap_or(class);
            // A Vendor_Module::path/to/template.phtml token is a template id,
            // not a Class::method call -- confirmed by a slash after `::` on
            // the same edit line (plan_validate_propagation_symbols_token).
            if is_vendor_module_class(class)
                && edit_lines
                    .lines()
                    .any(|line| line_has_template_path(line, class))
            {
                continue;
            }
            // A namespaced class (one with a `\` root) must sit under a
            // prefix the plan itself edits, or it is a vendor seam and drops
            // out. A bare, unnamespaced class carries no namespace to check
            // against, so it is always treated as a candidate for ownership
            // -- matching the shell case arm `"$klass_short")`, which always
            // matches when klass has no namespace (klass == klass_short).
            if class != short
                && !prefixes
                    .iter()
                    .any(|prefix| class.starts_with(prefix) || short.starts_with(prefix))
            {
                continue;
            }
            let owned = inventory.units.iter().any(|candidate| {
                candidate.file.rsplit('/').next().unwrap_or_default() == short
                    || candidate.file == class
                    || candidate.scope.split("::").next().unwrap_or_default() == class
                    || candidate
                        .scope
                        .split("::")
                        .next()
                        .unwrap_or_default()
                        .rsplit('\\')
                        .next()
                        .unwrap_or_default()
                        == short
            });
            if !owned && token != unit.id {
                findings.warn(format!("{} instructions mention '{}' which no inventory row owns; verify it is a seam description, or add a discovery/ownership row if it is an edit target", unit.id, token));
            }
        }
    }
}

/// `Vendor_Module::...` shape: two capitalized, underscore-joined words
/// ahead of `::` (`^[A-Z][a-zA-Z0-9]*_[A-Z][a-zA-Z0-9]*` in the shell pass).
fn is_vendor_module_class(class: &str) -> bool {
    let Some((first, second)) = class.split_once('_') else {
        return false;
    };
    let starts_upper = |part: &str| part.chars().next().is_some_and(|c| c.is_ascii_uppercase());
    starts_upper(first)
        && first.chars().all(|c| c.is_ascii_alphanumeric())
        && starts_upper(second)
        && second.chars().all(|c| c.is_ascii_alphanumeric())
}

/// True when `line` carries `class<word-chars>::<path-with-a-slash>` ahead of
/// a space or `(`, i.e. a template identifier rather than a method call.
fn line_has_template_path(line: &str, class: &str) -> bool {
    let Some(after_class) = line.split_once(class) else {
        return false;
    };
    let rest = after_class.1;
    let Some(sep) = rest.find("::") else {
        return false;
    };
    let (extra, path) = (&rest[..sep], &rest[sep + 2..]);
    if !extra.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    let path_segment = path
        .find([' ', '('])
        .map(|end| &path[..end])
        .unwrap_or(path);
    path_segment.contains('/')
}

pub fn validate_roster(plan: &Path, inventory: &Inventory, findings: &mut Findings) {
    for (goal, assigned_ids) in &inventory.goals {
        let goal_file = plan.join(goal).join("goal.md");
        if !goal_file.is_file() {
            continue;
        }
        let text = fs::read_to_string(&goal_file).unwrap_or_default();
        let mut roster = HashSet::new();
        let mut in_91 = false;
        let mut paragraph = String::new();
        for line in text.lines() {
            if line == "§ 9.1" {
                in_91 = true;
                continue;
            }
            if in_91 && (line.starts_with("§ ") || line.starts_with("## ")) {
                in_91 = false;
            }
            if in_91 && !line.trim().is_empty() {
                paragraph.push(' ');
                paragraph.push_str(line);
            }
        }
        let mut leading = paragraph;
        for marker in [" —", " - ", ".", ", in that order", " in that order"] {
            if let Some((head, _)) = leading.split_once(marker) {
                leading = head.to_owned();
            }
        }
        for id in ids_in(&leading) {
            roster.insert(id);
        }

        let mut in_owned = false;
        for line in text.lines() {
            if line == "## Owned work units" {
                in_owned = true;
                continue;
            }
            if line == "## Goal-size exception" {
                in_owned = false;
            }
            if in_owned {
                let trimmed = line.trim_start();
                if let Some(rest) = trimmed.strip_prefix('`') {
                    if let Some((id, _)) = rest.split_once('`') {
                        if valid_id(id) {
                            roster.insert(id.to_owned());
                        }
                    }
                }
            }
        }
        for id in &roster {
            if !inventory.units.iter().any(|unit| unit.id == *id) {
                continue;
            }
            if !assigned_ids.iter().any(|assigned| assigned == id) {
                findings.fail(format!(
                    "{goal} §9.x roster lists {id} which the inventory does not assign to this goal; reconcile the roster and the inventory"
                ));
            }
        }
        for id in assigned_ids {
            if !roster.contains(id) {
                findings.fail(format!(
                    "{goal} §9.x roster omits {id} which the inventory assigns to this goal; add it to the roster"
                ));
            }
        }
    }
}

pub fn validate_freshness(
    repo_root: &Path,
    plan: &Path,
    inventory: &Inventory,
    findings: &mut Findings,
) {
    let Ok(abs_repo) = fs::canonicalize(repo_root) else {
        return;
    };
    let Ok(abs_plan) = fs::canonicalize(plan) else {
        return;
    };
    let Ok(rel_plan) = abs_plan.strip_prefix(&abs_repo) else {
        return;
    };
    if rel_plan.as_os_str().is_empty() {
        return;
    }
    let Some(plan_newest) = git_timestamp(&abs_repo, rel_plan) else {
        return;
    };
    let mut drift = 0usize;
    for unit in &inventory.units {
        if unit.file.is_empty() || unit.file == "N/A" {
            continue;
        }
        let Some(code_newest) = git_timestamp(&abs_repo, Path::new(&unit.file)) else {
            continue;
        };
        if code_newest > plan_newest {
            drift += 1;
            if drift <= 3 {
                findings.warn(format!(
                    "unit {} target '{}' changed at {}, after the last plan record ({}); record the mutation with update-step or update-progress",
                    unit.id,
                    unit.file,
                    code_newest.split('T').next().unwrap_or(&code_newest),
                    plan_newest.split('T').next().unwrap_or(&plan_newest)
                ));
            }
        }
    }
    if drift > 3 {
        findings.warn(format!(
            "{drift} unit targets changed after the last plan record; bring the plan back to the world"
        ));
    }
}

fn git_timestamp(repo_root: &Path, path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(repo_root)
        .args(["log", "-1", "--format=%cI", "--"])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn read_step(plan: &Path, unit: &Unit) -> String {
    fs::read_to_string(
        plan.join(&unit.goal)
            .join("steps")
            .join(format!("{}.md", unit.step)),
    )
    .unwrap_or_default()
}
fn read_section(plan: &Path, unit: &Unit, heading: &str) -> String {
    let text = read_step(plan, unit);
    let mut inside = false;
    let mut output = String::new();
    for line in text.lines() {
        if line == heading {
            inside = true;
            continue;
        }
        if inside && line.starts_with("## ") {
            break;
        }
        if inside {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}
fn ids_in(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    for word in text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        if word.len() >= 3
            && word.starts_with('W')
            && word[1..].bytes().all(|byte| byte.is_ascii_digit())
            && !result.contains(&word.to_owned())
        {
            result.push(word.to_owned());
        }
    }
    result
}
fn valid_id(value: &str) -> bool {
    value.len() >= 3
        && value.starts_with('W')
        && value[1..].bytes().all(|byte| byte.is_ascii_digit())
}
fn dependencies(text: &str) -> HashSet<String> {
    ids_in(text).into_iter().collect()
}
fn reachable(inventory: &Inventory, from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    let mut seen = HashSet::new();
    let mut stack = vec![from.to_owned()];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let Some(unit) = inventory.units.iter().find(|unit| unit.id == id) else {
            continue;
        };
        for dep in dependencies(&unit.depends) {
            if dep == to {
                return true;
            }
            stack.push(dep);
        }
    }
    false
}
fn symbol_tokens(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    for word in text.split_whitespace().map(|word| {
        word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '\\' && c != ':')
    }) {
        if word.contains("::")
            && word.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && word.split("::").all(|part| !part.is_empty())
            && !result.contains(&word.to_owned())
        {
            result.push(word.to_owned());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_completed_plan_and_goal_progress_table_satisfies_the_completion_gate() {
        // Regression: this crate once carried its own private, differently
        // indexed `table_cell` (missing the canonical function's `- 1`
        // adjustment), silently reading the wrong columns -- Goalname read
        // back as the Description cell, Completion status as an empty
        // trailing cell -- so a fully-completed plan always FAILed under
        // --complete. Fixed by importing planning_table::table_cell instead
        // of shadowing it locally.
        let root = std::env::temp_dir().join(format!(
            "validator-propagation-completion-{}",
            std::process::id()
        ));
        let goal_dir = root.join("01-goal");
        fs::create_dir_all(&goal_dir).unwrap();
        fs::write(
            root.join("progress.md"),
            "| Goalname | Description | Completion status |\n|---|---|---|\n| 01-goal | do the thing | ✅ completed |\n",
        )
        .unwrap();
        fs::write(
            goal_dir.join("progress.md"),
            "| Goalname | Stepname | Description | Completion status |\n|---|---|---|---|\n| 01-goal | 01-step | do the thing | ✅ completed |\n",
        )
        .unwrap();
        let inventory = Inventory {
            units: vec![Unit {
                id: "W01".into(),
                goal: "01-goal".into(),
                step: "01-step".into(),
                ..Unit::default()
            }],
            goals: [("01-goal".into(), vec!["W01".into()])]
                .into_iter()
                .collect(),
            ..Inventory::default()
        };
        let mut findings = Findings::default();
        validate_completion(&root, &inventory, true, &mut findings);
        assert_eq!(findings.errors, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn finds_transitive_dependency() {
        let inventory = Inventory {
            units: vec![
                Unit {
                    id: "W01".into(),
                    depends: "".into(),
                    ..Unit::default()
                },
                Unit {
                    id: "W02".into(),
                    depends: "W01".into(),
                    ..Unit::default()
                },
            ],
            ..Inventory::default()
        };
        assert!(reachable(&inventory, "W02", "W01"));
    }
    #[test]
    fn extracts_work_unit_ids() {
        assert_eq!(ids_in("W01 and W02; W01"), vec!["W01", "W02"]);
    }

    #[test]
    fn roster_requires_the_inventory_assignment_set() {
        let root = std::env::temp_dir().join(format!("validator-roster-{}", std::process::id()));
        let goal = root.join("01-goal");
        fs::create_dir_all(&goal).unwrap();
        fs::write(
            goal.join("goal.md"),
            "§ 9.1\nW01 — first.\n\n## Owned work units\n`W01` — first\n",
        )
        .unwrap();
        let inventory = Inventory {
            units: vec![Unit {
                id: "W01".into(),
                goal: "01-goal".into(),
                ..Unit::default()
            }],
            goals: [("01-goal".into(), vec!["W01".into()])]
                .into_iter()
                .collect(),
            ..Inventory::default()
        };
        let mut findings = Findings::default();
        validate_roster(&root, &inventory, &mut findings);
        assert_eq!(findings.errors, 0);
        let _ = fs::remove_dir_all(root);
    }

    fn scratch_symbols_plan(name: &str, instructions: &str) -> (std::path::PathBuf, Unit) {
        let root =
            std::env::temp_dir().join(format!("validator-symbols-{name}-{}", std::process::id()));
        let steps = root.join("01-goal").join("steps");
        fs::create_dir_all(&steps).unwrap();
        fs::write(
            steps.join("01-step.md"),
            format!("## Instructions\n\n{instructions}\n\n## Acceptance criteria\n"),
        )
        .unwrap();
        let unit = Unit {
            id: "W01".into(),
            file: "src/plan-overview/src/render/shell.rs".into(),
            goal: "01-goal".into(),
            step: "01-step".into(),
            ..Unit::default()
        };
        (root, unit)
    }

    #[test]
    fn a_bare_class_with_no_namespace_is_always_checked_for_ownership() {
        // RenderBuffer carries no namespace root, so it cannot be matched
        // against any project prefix -- bash's own case pattern always
        // treats this shape as a candidate (B335-adjacent gap, goal 4).
        let (root, unit) = scratch_symbols_plan(
            "bare-class",
            "Create memory.rs against RenderBuffer::new in render/shell.rs.",
        );
        let inventory = Inventory {
            units: vec![unit],
            ..Inventory::default()
        };
        let mut findings = Findings::default();
        validate_symbols(&root, &inventory, &mut findings);
        assert_eq!(findings.warnings, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn create_and_add_are_edit_intent_verbs_not_just_edit_change_update_implement() {
        let (root, unit) =
            scratch_symbols_plan("create-verb", "Add a call to Widget::render here.");
        let inventory = Inventory {
            units: vec![unit],
            ..Inventory::default()
        };
        let mut findings = Findings::default();
        validate_symbols(&root, &inventory, &mut findings);
        assert_eq!(findings.warnings, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_symbol_an_inventory_row_already_owns_is_not_flagged() {
        let (root, mut unit) =
            scratch_symbols_plan("owned", "Update RenderBuffer::write_str for the new field.");
        unit.scope = "RenderBuffer::write_str".into();
        let inventory = Inventory {
            units: vec![unit],
            ..Inventory::default()
        };
        let mut findings = Findings::default();
        validate_symbols(&root, &inventory, &mut findings);
        assert_eq!(findings.warnings, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_vendor_module_template_path_is_not_a_class_method_token() {
        let (root, unit) = scratch_symbols_plan(
            "template-path",
            "Update the Magento_Weee::email/items/price/row.phtml template.",
        );
        let inventory = Inventory {
            units: vec![unit],
            ..Inventory::default()
        };
        let mut findings = Findings::default();
        validate_symbols(&root, &inventory, &mut findings);
        assert_eq!(findings.warnings, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_line_with_no_edit_intent_verb_is_never_flagged() {
        let (root, unit) = scratch_symbols_plan(
            "no-verb",
            "See RenderBuffer::new for context on the seam boundary.",
        );
        let inventory = Inventory {
            units: vec![unit],
            ..Inventory::default()
        };
        let mut findings = Findings::default();
        validate_symbols(&root, &inventory, &mut findings);
        assert_eq!(findings.warnings, 0);
        let _ = fs::remove_dir_all(root);
    }
}
