// MODE: DEV
// PACKAGE: PROD
//! Work-unit inventory parsing and graph checks formerly provided by
//! `validate-plan-inventory-lib.sh`.

use planning_validator_common::{trim, Findings};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Unit {
    pub id: String,
    pub kind: String,
    pub file: String,
    pub scope: String,
    pub subscope: String,
    pub intended: String,
    pub depends: String,
    pub goal: String,
    pub step: String,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Inventory {
    pub units: Vec<Unit>,
    pub goals: BTreeMap<String, Vec<String>>,
    pub coverage_ids: BTreeSet<String>,
}

impl Inventory {
    pub fn parse(path: &Path, findings: &mut Findings) -> Self {
        let text = fs::read_to_string(path).unwrap_or_default();
        let mut inventory = Self::default();
        let mut seen = HashSet::new();
        let mut seen_steps = HashSet::new();
        for line in text.lines().filter(|line| is_unit_row(line)) {
            let cells = cells(line);
            if cells.len() < 9 {
                continue;
            }
            let unit = Unit {
                id: cells[0].clone(),
                kind: cells[1].clone(),
                file: cells[2].clone(),
                scope: cells[3].clone(),
                subscope: cells[4].clone(),
                intended: cells[5].clone(),
                depends: cells[6].clone(),
                goal: cells[7].clone(),
                step: cells[8].clone(),
            };
            if !valid_id(&unit.id) {
                findings.fail(format!("Invalid work-unit ID: {}", unit.id));
                continue;
            }
            if !seen.insert(unit.id.clone()) {
                findings.fail(format!("Duplicate work-unit ID: {}", unit.id));
                continue;
            }
            validate_unit(&unit, findings);
            let step_key = format!("{}/{}", unit.goal, unit.step);
            if !seen_steps.insert(step_key) {
                findings.fail(format!(
                    "Multiple work units are assigned to {}/steps/{}.md",
                    unit.goal, unit.step
                ));
            }
            inventory
                .goals
                .entry(unit.goal.clone())
                .or_default()
                .push(unit.id.clone());
            inventory.units.push(unit);
        }
        if inventory.units.is_empty() {
            findings.fail("No work-unit rows found; use IDs such as W01 in the Work units table");
        }
        for line in text.lines() {
            if line.starts_with("## Work units") {
                break;
            }
            if !line.starts_with('|') {
                continue;
            }
            let coverage = cells(line).get(1).cloned().unwrap_or_default();
            if coverage.starts_with("Required outcome") || coverage.contains("---") {
                continue;
            }
            for id in work_unit_ids(&coverage) {
                inventory.coverage_ids.insert(id);
            }
        }
        if inventory.coverage_ids.is_empty() {
            findings.fail("Definition-of-done coverage has no work-unit references");
        }
        for unit in &inventory.units {
            if !inventory.coverage_ids.contains(&unit.id) {
                findings.fail(format!(
                    "{} is not linked to a definition-of-done item",
                    unit.id
                ));
            }
        }
        for id in &inventory.coverage_ids {
            if !inventory.units.iter().any(|unit| unit.id == *id) {
                findings.fail(format!(
                    "Definition-of-done coverage names unknown work unit {id}"
                ));
            }
        }
        inventory
    }

    pub fn validate_dependency_graph(&self, findings: &mut Findings) {
        let known = self
            .units
            .iter()
            .map(|unit| unit.id.as_str())
            .collect::<HashSet<_>>();
        let mut states = BTreeMap::<String, Visit>::new();
        for unit in &self.units {
            visit(unit, &known, self, &mut states, findings);
        }
    }

    pub fn validate_target_paths(&self, repo_root: Option<&Path>, findings: &mut Findings) {
        let Some(repo_root) = repo_root else {
            return;
        };
        for unit in &self.units {
            if matches!(
                unit.kind.as_str(),
                "discovery" | "verification" | "generated"
            ) {
                continue;
            }
            if unit.file == "N/A" {
                continue;
            }
            let target = repo_root.join(&unit.file);
            if !target.is_file() {
                findings.fail(format!(
                    "{} target file does not exist under --repo-root: {}",
                    unit.id, unit.file
                ));
                continue;
            }
            if matches!(unit.scope.as_str(), "N/A")
                || unit.scope.starts_with('#')
                || unit.scope.starts_with('.')
            {
                continue;
            }
            let contents = fs::read_to_string(&target).unwrap_or_default();
            if !contents.contains(&unit.scope) {
                findings.fail(format!(
                    "{} primary symbol or file scope was not found in {}: {}",
                    unit.id, unit.file, unit.scope
                ));
            }
        }
    }

    pub fn depends_on(&self, candidate: &str, required: &str) -> bool {
        if candidate == required {
            return true;
        }
        let mut seen = HashSet::new();
        let mut stack = vec![candidate.to_owned()];
        while let Some(current) = stack.pop() {
            if !seen.insert(current.clone()) {
                continue;
            }
            let Some(unit) = self.units.iter().find(|unit| unit.id == current) else {
                continue;
            };
            for dependency in work_unit_ids(&unit.depends) {
                if dependency == required {
                    return true;
                }
                stack.push(dependency);
            }
        }
        false
    }
}

fn validate_unit(unit: &Unit, findings: &mut Findings) {
    const TYPES: &[&str] = &[
        "source",
        "markup",
        "style",
        "test",
        "config",
        "docs",
        "data",
        "generated",
        "discovery",
        "verification",
    ];
    if !TYPES.contains(&unit.kind.as_str()) {
        findings.fail(format!("{} has unsupported type '{}'", unit.id, unit.kind));
    }
    if unit.file.is_empty()
        || unit.scope.is_empty()
        || unit.subscope.is_empty()
        || unit.intended.is_empty()
        || unit.goal.is_empty()
        || unit.step.is_empty()
    {
        findings.fail(format!("{} has an empty required work-unit field", unit.id));
    }
    if unit.kind == "verification" && unit.file != "N/A" {
        findings.fail(format!(
            "{} is verification and must use File 'N/A'",
            unit.id
        ));
    }
    if unit.kind != "verification" && unit.kind != "discovery" && unit.file == "N/A" {
        findings.fail(format!(
            "{} is neither verification nor discovery and must name one file",
            unit.id
        ));
    }
    if unit.file.contains('*') || unit.file.ends_with('/') {
        findings.fail(format!(
            "{} must name one concrete file, not a glob or directory: {}",
            unit.id, unit.file
        ));
    }
    if unit.kind != "verification" {
        let symbol_count = unit.scope.matches("::").count();
        if symbol_count > 1 || unit.scope.contains(',') {
            findings.fail(format!(
                "{} lists multiple symbols or scopes: {}",
                unit.id, unit.scope
            ));
        }
    }
    if unit.kind == "style" && !css_selector(&unit.scope) {
        findings.fail(format!(
            "{} style scope must be one CSS selector, such as .completion-message",
            unit.id
        ));
    }
    if unit.kind == "markup" && !dom_selector(&unit.scope) {
        findings.fail(format!(
            "{} markup scope must be one named DOM selector, such as #checkout-summary",
            unit.id
        ));
    }
    if !goal_name(&unit.goal) {
        findings.fail(format!("{} has invalid goal name '{}'", unit.id, unit.goal));
    }
    if !step_name(&unit.step) {
        findings.fail(format!("{} has invalid step name '{}'", unit.id, unit.step));
    }
    if unit.subscope != "N/A" && (unit.subscope.contains(',') || unit.subscope.contains(" and ")) {
        findings.fail(format!(
            "{} lists multiple subscope targets: {}",
            unit.id, unit.subscope
        ));
    }
}

fn visit(
    unit: &Unit,
    known: &HashSet<&str>,
    inventory: &Inventory,
    states: &mut BTreeMap<String, Visit>,
    findings: &mut Findings,
) {
    match states.get(&unit.id) {
        Some(Visit::Visiting) => {
            findings.fail(format!("Dependency cycle includes {}", unit.id));
            return;
        }
        Some(Visit::Done) => return,
        None => {}
    }
    states.insert(unit.id.clone(), Visit::Visiting);
    for dependency in work_unit_ids(&unit.depends) {
        if !known.contains(dependency.as_str()) {
            findings.fail(format!(
                "{} depends on unknown work unit {}",
                unit.id, dependency
            ));
        } else if let Some(next) = inventory.units.iter().find(|next| next.id == dependency) {
            visit(next, known, inventory, states, findings);
        }
    }
    states.insert(unit.id.clone(), Visit::Done);
}

#[derive(Clone, Copy)]
enum Visit {
    Visiting,
    Done,
}

fn cells(line: &str) -> Vec<String> {
    line.split('|')
        .skip(1)
        .take_while(|_| true)
        .map(trim)
        .collect()
}

fn is_unit_row(line: &str) -> bool {
    let id = cells(line).into_iter().next().unwrap_or_default();
    valid_id(&id)
}

fn work_unit_ids(value: &str) -> Vec<String> {
    let bytes = value.as_bytes();
    let mut result = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'W' || index + 2 >= bytes.len() || !bytes[index + 1].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        index += 2;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        result.push(value[start..index].to_owned());
    }
    result
}

fn valid_id(value: &str) -> bool {
    value
        .strip_prefix('W')
        .is_some_and(|rest| rest.len() >= 2 && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

fn goal_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 4
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b'-'
        && bytes[3..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

fn step_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 9
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && &bytes[2..8] == b"-step-"
        && bytes[8..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

fn css_selector(value: &str) -> bool {
    selector(value, true)
}

fn dom_selector(value: &str) -> bool {
    selector(value, false)
}

fn selector(value: &str, allow_dot: bool) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2
        && (bytes[0] == b'#' || (allow_dot && bytes[0] == b'.'))
        && (bytes[1].is_ascii_alphabetic() || bytes[1] == b'_')
        && bytes[2..]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::Inventory;
    use planning_validator_common::Findings;
    use std::fs;

    #[test]
    fn parses_units_coverage_and_dependency_edges() {
        let root = std::env::temp_dir().join(format!("validator-inventory-{}", std::process::id()));
        let file = root.join("work-unit-inventory.md");
        let _ = fs::create_dir_all(&root);
        fs::write(
            &file,
            "## Definition-of-done coverage\n\n| Required outcome | Work units | Notes |\n|---|---|---|\n| outcome | W01,W02 | note |\n\n## Work units\n\n| ID | Type | File | Scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n| W01 | source | a.rs | A::run | N/A | A | — | 01-goal | 01-step-a |\n| W02 | test | t.sh | run | N/A | T | W01 | 01-goal | 02-step-b |\n",
        )
        .unwrap();
        let mut findings = Findings::default();
        let inventory = Inventory::parse(&file, &mut findings);
        assert_eq!(inventory.units.len(), 2);
        assert_eq!(inventory.coverage_ids.len(), 2);
        inventory.validate_dependency_graph(&mut findings);
        assert_eq!(findings.errors, 0);
        assert!(inventory.depends_on("W02", "W01"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn detects_unknown_dependency_and_cycle() {
        let root =
            std::env::temp_dir().join(format!("validator-inventory-errors-{}", std::process::id()));
        let file = root.join("inventory.md");
        let _ = fs::create_dir_all(&root);
        fs::write(
            &file,
            "| Required outcome | Work units | Notes |\n|---|---|---|\n| x | W01,W02 | n |\n## Work units\n| ID | Type | File | Scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n| W01 | source | a | A::x | N/A | a | W02 W99 | 01-goal | 01-step-a |\n| W02 | source | b | B::x | N/A | b | W01 | 01-goal | 02-step-b |\n",
        )
        .unwrap();
        let mut findings = Findings::default();
        let inventory = Inventory::parse(&file, &mut findings);
        inventory.validate_dependency_graph(&mut findings);
        assert!(findings
            .messages
            .iter()
            .any(|message| message.text.contains("unknown work unit W99")));
        assert!(findings
            .messages
            .iter()
            .any(|message| message.text.contains("Dependency cycle")));
        let _ = fs::remove_dir_all(root);
    }
}
