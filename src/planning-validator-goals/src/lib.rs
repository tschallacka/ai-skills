// MODE: DEV
// PACKAGE: PROD
//! Goal and step validation formerly provided by `validate-plan-goals-lib.sh`.

use planning_validator_common::Findings;
use planning_validator_inventory::{Inventory, Unit};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const GOAL_HEADINGS: &[&str] = &[
    "## Current state and prior-goal handoffs",
    "## Outcome and definition of done",
    "## Why this goal is needed",
    "## Scope",
    "## Affected files, systems, data, and interfaces",
    "## Dependencies and handoffs",
    "## Implementation approach, risks, and edge cases",
    "## Owned work units",
    "## Testing requirement",
];
const STEP_HEADINGS: &[&str] = &[
    "## Ownership",
    "## Change target",
    "## Objective",
    "## Instructions",
    "## Acceptance criteria",
    "## Handoff",
    "## Atomicity check",
];

#[derive(Debug, Default)]
pub struct GoalTableRegistry {
    sections: BTreeSet<String>,
}

impl GoalTableRegistry {
    pub fn from_file(path: &Path, findings: &mut Findings) -> Self {
        let mut registry = Self::default();
        let Ok(text) = fs::read_to_string(path) else {
            findings.fail(format!(
                "goal-tables.json registry is missing at {}",
                path.display()
            ));
            return registry;
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            findings.fail(format!(
                "goal-tables.json registry is invalid at {}",
                path.display()
            ));
            return registry;
        };
        if let Some(tables) = value.get("tables").and_then(Value::as_array) {
            for table in tables {
                if table.get("document").and_then(Value::as_str) == Some("goal.md") {
                    if let Some(section) = table.get("section").and_then(Value::as_str) {
                        registry.sections.insert(section.to_owned());
                    }
                }
            }
        }
        if registry.sections.is_empty() {
            findings
                .fail("goal-tables.json registers no goal.md section with a yes/no first column");
        }
        registry
    }
}

pub fn validate_goals(
    plan: &Path,
    inventory: &Inventory,
    registry: &GoalTableRegistry,
    complete: bool,
    findings: &mut Findings,
) {
    for (goal, ids) in &inventory.goals {
        let goal_dir = plan.join(goal);
        let goal_file = goal_dir.join("goal.md");
        if !goal_file.is_file() {
            findings.fail(format!("Missing goal.md for {goal}"));
            continue;
        }
        let text = fs::read_to_string(&goal_file).unwrap_or_default();
        require_headings(&goal_file, &text, GOAL_HEADINGS, findings);
        let goal_file_display = format!("{}/{goal}//goal.md", plan.display());
        validate_testing_requirement(&goal_file_display, goal, &text, inventory, ids, findings);
        validate_yes_no_tables(&goal_file, &text, registry, findings);
        if text.contains("<required only when this goal has one permitted work unit>") {
            findings.fail(format!("{goal} has an unfilled goal-size placeholder; fill the reason or remove the section"));
        }
        if ids.is_empty() {
            findings.fail(format!("{goal} has no assigned work units"));
        }
        validate_size(&goal_file, goal, ids, inventory, findings);
        validate_testing_band(plan, goal, ids, inventory, findings);
        let _ = complete;
    }
}

pub fn validate_steps(plan: &Path, inventory: &Inventory, findings: &mut Findings) {
    for unit in &inventory.units {
        let goal_dir = plan.join(&unit.goal);
        let step_file = goal_dir.join("steps").join(format!("{}.md", unit.step));
        if !goal_dir.is_dir() {
            findings.fail(format!(
                "{} references missing goal directory {}",
                unit.id, unit.goal
            ));
            continue;
        }
        if !step_file.is_file() {
            findings.fail(format!(
                "{} references missing step file {}",
                unit.id,
                step_file.display()
            ));
            continue;
        }
        let text = fs::read_to_string(&step_file).unwrap_or_default();
        require_headings(&step_file, &text, STEP_HEADINGS, findings);
        exact_line(
            &step_file,
            &text,
            &format!("- Goal: `{}`", unit.goal),
            "owning goal",
            &unit.id,
            findings,
        );
        exact_line(
            &step_file,
            &text,
            &format!("- Work unit: `{}`", unit.id),
            "work-unit ID",
            &unit.id,
            findings,
        );
        exact_line(
            &step_file,
            &text,
            &format!("- Type: `{}`", unit.kind),
            "type",
            &unit.id,
            findings,
        );
        field_match(&step_file, &text, "File", &unit.file, &unit.id, findings);
        field_match(
            &step_file,
            &text,
            "Primary symbol or file scope",
            &unit.scope,
            &unit.id,
            findings,
        );
        field_match(
            &step_file,
            &text,
            "Subscope",
            &unit.subscope,
            &unit.id,
            findings,
        );
        validate_atomicity(plan, unit, &step_file, &text, findings);
    }
}

pub fn validate_step_naming(plan: &Path, inventory: &Inventory, findings: &mut Findings) {
    for entry in walk(plan) {
        let Some(name) = entry.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".md")
            || name.ends_with("-testing.md")
            || entry
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                != Some("steps")
        {
            continue;
        }
        let declared = field(&fs::read_to_string(&entry).unwrap_or_default(), "Work unit");
        let Some(unit) = inventory.units.iter().find(|unit| unit.id == declared) else {
            findings.fail(format!(
                "{} declares unlisted work unit '{}'",
                entry.display(),
                declared
            ));
            continue;
        };
        let actual = name.trim_end_matches(".md");
        if unit.goal
            != entry
                .ancestors()
                .nth(2)
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or_default()
            || unit.step != actual
        {
            findings.fail(format!(
                "{} does not match the inventory assignment for {}",
                entry.display(),
                declared
            ));
        }
        if !is_numbered_step(name) {
            findings.fail(format!("{} is not a numbered step file", entry.display()));
        }
    }
}

fn validate_testing_requirement(
    display_file: &str,
    goal: &str,
    text: &str,
    inventory: &Inventory,
    ids: &[String],
    findings: &mut Findings,
) {
    let rows = section_rows(text, "## Testing requirement");
    let matching = rows
        .iter()
        .filter(|row| row.len() >= 2 && matches!(row[0].trim(), "yes" | "no"))
        .collect::<Vec<_>>();
    let header = rows.iter().any(|row| {
        row.len() >= 2 && row[0].trim() == "Test required" && row[1].trim() == "Rationale"
    });
    let separator = text.lines().any(|line| line == "|---|---|");
    if !header
        || !separator
        || rows
            .iter()
            .filter(|row| row.len() >= 2 && row[0].trim() == "Test required")
            .count()
            != 1
        || matching.len() != 1
    {
        findings.fail(format!(
            "{} must contain exactly one Test required/Rationale table row",
            display_file
        ));
        return;
    }
    let required = matching[0][0].trim();
    let rationale = matching[0][1].trim();
    if rationale.is_empty() || rationale.contains('<') && rationale.contains('>') {
        findings.fail(format!(
            "{} must explain why testing is or is not required",
            display_file
        ));
    }
    let test_units = ids
        .iter()
        .filter_map(|id| inventory.units.iter().find(|unit| &unit.id == id))
        .filter(|unit| matches!(unit.kind.as_str(), "test" | "verification"))
        .count();
    if required == "yes" && test_units == 0 {
        findings.fail(format!(
            "{goal} declares testing is required but has no test or verification work unit"
        ));
    }
    if test_units > 0 && required != "yes" {
        findings.fail(format!(
            "{goal} has a test or verification work unit but its testing requirement is not yes"
        ));
    }
}

fn validate_testing_band(
    plan: &Path,
    goal: &str,
    ids: &[String],
    inventory: &Inventory,
    findings: &mut Findings,
) {
    let required = fs::read_to_string(plan.join(goal).join("goal.md"))
        .ok()
        .map(|text| section_rows(&text, "## Testing requirement"))
        .and_then(|rows| {
            rows.into_iter()
                .find(|row| row.len() >= 2 && matches!(row[0].trim(), "yes" | "no"))
        })
        .map(|row| row[0].trim().to_owned())
        .unwrap_or_default();
    if required != "yes" {
        return;
    }
    for id in ids {
        let Some(unit) = inventory.units.iter().find(|unit| &unit.id == id) else {
            continue;
        };
        if unit.kind != "docs" {
            let companion = plan
                .join(goal)
                .join("steps")
                .join(format!("{}-testing.md", unit.step));
            if !companion.is_file() {
                findings.fail(format!(
                    "{id} requires testing instructions at {}",
                    companion.display()
                ));
            }
        }
    }
}

fn validate_size(
    file: &Path,
    goal: &str,
    ids: &[String],
    inventory: &Inventory,
    findings: &mut Findings,
) {
    if ids.len() == 1 {
        let text = fs::read_to_string(file).unwrap_or_default();
        let exception = text.contains("## Goal-size exception");
        let kind = inventory
            .units
            .iter()
            .find(|unit| ids.contains(&unit.id))
            .map(|unit| unit.kind.as_str())
            .unwrap_or("");
        if !matches!(kind, "docs" | "config" | "discovery" | "verification") {
            findings.fail(format!(
                "{goal} has one {kind} work unit; add its test/proof or merge it into its demonstrable outcome"
            ));
        } else if !exception {
            findings.fail(format!(
                "Missing '## Goal-size exception' in {}",
                file.display()
            ));
        }
    } else if ids.len() > 10 {
        findings.fail(format!(
            "{goal} has {} work units; split it at a stable outcome boundary",
            ids.len()
        ));
    }
}

fn validate_yes_no_tables(
    file: &Path,
    text: &str,
    registry: &GoalTableRegistry,
    findings: &mut Findings,
) {
    for section in yes_no_sections(text) {
        if !registry.sections.contains(&section) {
            findings.fail(format!("{} has a hand-written yes/no table under '{}'; only registered goal sections may carry one", file.display(), section));
        }
    }
}

fn validate_atomicity(plan: &Path, unit: &Unit, file: &Path, text: &str, findings: &mut Findings) {
    let completed = fs::read_to_string(plan.join(&unit.goal).join("progress.md"))
        .ok()
        .is_some_and(|progress| {
            progress
                .lines()
                .any(|line| line.contains(&unit.step) && line.contains("completed"))
        });
    for sentence in [
        "This step owns exactly one inventory work unit.",
        "No other file, symbol, test target, or verification flow changes here.",
        "Any follow-on target has a separately named work unit and step.",
    ] {
        let ticked = text.lines().any(|line| {
            (line.starts_with("- [x]") || line.starts_with("- [X]")) && line.contains(sentence)
        });
        if !ticked && completed {
            findings.fail(format!(
                "{} has unticked atomicity box but its step is completed",
                file.display()
            ));
        }
    }
}

fn require_headings(file: &Path, text: &str, headings: &[&str], findings: &mut Findings) {
    for heading in headings {
        if !text.lines().any(|line| line == *heading) {
            findings.fail(format!(
                "{} is missing required heading {}",
                file.display(),
                heading
            ));
        }
    }
}
fn exact_line(
    file: &Path,
    text: &str,
    expected: &str,
    label: &str,
    id: &str,
    findings: &mut Findings,
) {
    if !text.lines().any(|line| line == expected) {
        findings.fail(format!("{} has wrong {} for {}", file.display(), label, id));
    }
}
fn field_match(
    file: &Path,
    text: &str,
    label: &str,
    expected: &str,
    id: &str,
    findings: &mut Findings,
) {
    if field(text, label) != expected {
        findings.fail(format!(
            "{} {} does not match {} inventory row",
            file.display(),
            label.to_lowercase(),
            id
        ));
    }
}
fn field(text: &str, label: &str) -> String {
    text.lines()
        .find_map(|line| {
            line.strip_prefix(&format!("- {label}: "))
                .map(|value| value.trim().trim_matches('`').to_owned())
        })
        .unwrap_or_default()
}
fn section_rows(text: &str, heading: &str) -> Vec<Vec<String>> {
    let mut inside = false;
    text.lines()
        .filter_map(|line| {
            if line == heading {
                inside = true;
                return None;
            }
            if inside && line.starts_with("## ") {
                inside = false;
            }
            if !inside || !line.starts_with('|') || line.contains("---") {
                return None;
            }
            Some(
                line.split('|')
                    .skip(1)
                    .take_while(|_| true)
                    .map(|v| v.trim().to_owned())
                    .collect(),
            )
        })
        .collect()
}
fn yes_no_sections(text: &str) -> Vec<String> {
    let mut section = String::new();
    let mut result = Vec::new();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("## ") {
            section = format!("## {value}");
        }
        if (line.trim_start().starts_with("| yes |") || line.trim_start().starts_with("| no |"))
            && !result.contains(&section)
        {
            result.push(section.clone());
        }
    }
    result
}
fn is_numbered_step(name: &str) -> bool {
    let Some(rest) = name.strip_suffix(".md") else {
        return false;
    };
    let bytes = rest.as_bytes();
    if bytes.len() < 9
        || !bytes[0].is_ascii_digit()
        || !bytes[1].is_ascii_digit()
        || !rest[2..].starts_with("-step-")
    {
        return false;
    }
    let suffix = &rest[7..];
    !suffix.is_empty()
        && suffix
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            result.extend(walk(&path));
        } else if path.is_file() {
            result.push(path);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_testing_rows() {
        assert_eq!(section_rows("## Testing requirement\n| Test required | Rationale |\n|---|---|\n| yes | prove it |", "## Testing requirement")[1][0], "yes");
    }
    #[test]
    fn recognizes_numbered_steps() {
        assert!(is_numbered_step("02-step-build.md"));
        assert!(!is_numbered_step("step-build.md"));
    }
}
