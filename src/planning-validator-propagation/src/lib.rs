// MODE: DEV
// PACKAGE: PROD
//! Completion and propagation validation formerly provided by
//! `validate-plan-propagation-lib.sh`.

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
                None => unit.file.split('/').next().unwrap_or_default().to_owned(),
            })
        })
        .collect::<Vec<_>>();
    for unit in &inventory.units {
        let text = read_section(plan, unit, "## Instructions");
        if !text.to_ascii_lowercase().contains("edit")
            && !text.to_ascii_lowercase().contains("change")
            && !text.to_ascii_lowercase().contains("update")
            && !text.to_ascii_lowercase().contains("implement")
        {
            continue;
        }
        for token in symbol_tokens(&text) {
            if token.ends_with("::class") {
                continue;
            }
            let class = token.split("::").next().unwrap_or_default();
            let short = class.rsplit('\\').next().unwrap_or(class);
            if !prefixes
                .iter()
                .any(|prefix| class.starts_with(prefix) || short.starts_with(prefix))
            {
                continue;
            }
            let owned = inventory.units.iter().any(|candidate| {
                candidate
                    .file
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .trim_end_matches(".php")
                    == short
                    || candidate.scope.starts_with(class)
            });
            if !owned {
                findings.warn(format!("{} instructions mention '{}' which no inventory row owns; verify it is a seam description, or add a discovery/ownership row if it is an edit target", unit.id, token));
            }
        }
    }
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
fn table_cell(line: &str, index: usize) -> String {
    if !line.starts_with('|') {
        return String::new();
    }
    line.split('|')
        .nth(index)
        .unwrap_or_default()
        .trim()
        .to_owned()
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
}
