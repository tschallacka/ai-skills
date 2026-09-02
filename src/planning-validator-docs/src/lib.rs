// MODE: DEV
// PACKAGE: PROD
//! Document-level validation passes formerly provided by
//! `validate-plan-docs-lib.sh`.

use planning_validator_common::{get_single_field, require_heading, Findings};
use std::path::{Path, PathBuf};

pub const REQUIRED_PLAN_HEADINGS: &[&str] = &[
    "## Current state",
    "## Desired outcome",
    "## Approach",
    "## Approach decisions",
    "## Scope",
    "## Affected areas",
    "## Constraints and decisions",
    "## Risks and open questions",
    "## Environment facts",
    "## UI classification",
    "## Adversarial review",
];

#[derive(Debug, Default, Eq, PartialEq)]
pub struct DocumentState {
    pub ui_affected: String,
    pub review_approved: bool,
    pub plan_docs: Vec<PathBuf>,
}

/// Return the shell pass's status code while recording ordinary missing-input
/// findings in the shared accumulator.
pub fn validate_existence(plan: &Path, findings: &mut Findings) -> i32 {
    if !plan.is_dir() {
        eprintln!("Plan directory not found: {}", plan.display());
        return 66;
    }
    if !plan.join("plan-description.md").is_file() {
        findings.fail("Missing plan-description.md");
    }
    if !plan.join("work-unit-inventory.md").is_file() {
        findings.fail("Missing work-unit-inventory.md");
    }
    if findings.errors > 0 {
        1
    } else {
        0
    }
}

/// Check the marker used to retire old plans. `None` means validation may
/// continue; `Some(65)` is the hard refusal used by the shell entry point.
pub fn validate_obsolete(plan: &Path, script_name: &str) -> Option<i32> {
    let marker = plan.join("OBSOLETE");
    if !marker.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(&marker).unwrap_or_default();
    let replacement = text.lines().find_map(|line| {
        line.trim_start()
            .strip_prefix("replaced-by:")
            .map(str::trim)
            .filter(|value| !value.is_empty())
    });
    eprintln!(
        "{script_name}: {} is marked obsolete by its OBSOLETE marker: it was built by an older planning-skill version and is never validated, resumed, or migrated.",
        plan.display()
    );
    if let Some(replacement) = replacement {
        eprintln!("{script_name}: the initiative was rebuilt as: {replacement}");
    } else {
        eprintln!("{script_name}: the marker names no replacement; add a \"replaced-by: <plan directory>\" line naming the plan that supersedes this one.");
    }
    eprintln!("{script_name}: nothing here was deleted — this directory is kept as history. Validate the replacement instead.");
    Some(65)
}

/// Detect duplicate numeric step prefixes within each goal and report the
/// same collision description consumed by the shell pass.
pub fn validate_step_numbers(plan: &Path, findings: &mut Findings) {
    let Ok(goals) = std::fs::read_dir(plan) else {
        return;
    };
    let mut by_goal = std::collections::BTreeMap::<String, Vec<(String, String)>>::new();
    for goal in goals.flatten().filter(|entry| entry.path().is_dir()) {
        let steps = goal.path().join("steps");
        let Ok(entries) = std::fs::read_dir(&steps) else {
            continue;
        };
        for step in entries.flatten().filter(|entry| entry.path().is_file()) {
            let Some(name) = step.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some((number, _)) = name.split_once('-') else {
                continue;
            };
            if number.len() != 2 || !number.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            by_goal
                .entry(goal.file_name().to_string_lossy().into_owned())
                .or_default()
                .push((number.to_owned(), name));
        }
    }
    for (goal, mut steps) in by_goal {
        steps.sort();
        for pair in steps.windows(2) {
            if pair[0].0 == pair[1].0 {
                findings.fail(format!(
                    "{goal} has two steps numbered {} ({} and {}); their order is undefined. Rename one to a free number and sweep the five surfaces that name a step: the step file, its testing companion, the inventory row, the goal blurb, and any progress tracker",
                    pair[0].0,
                    pair[0].1,
                    pair[1].1
                ));
            }
        }
    }
}

pub fn validate_plan_documents(
    plan: &Path,
    complete_mode: bool,
    findings: &mut Findings,
) -> DocumentState {
    let description = plan.join("plan-description.md");
    for heading in REQUIRED_PLAN_HEADINGS {
        require_heading(&description, heading, findings);
    }
    let ui_affected = get_single_field(&description, "UI affected", findings);
    if ui_affected != "yes" && ui_affected != "no" {
        findings.fail("UI classification must declare '- UI affected: yes' or 'no'");
    }
    let review = plan.join("adversarial-review.md");
    let mut review_approved = false;
    if !review.is_file() {
        findings.fail("Missing adversarial-review.md");
    } else {
        for heading in ["## Review scope", "## Findings", "## Verdict"] {
            require_heading(&review, heading, findings);
        }
        let review_text = std::fs::read_to_string(&review).unwrap_or_default();
        review_approved = review_text
            .lines()
            .any(|line| line == "- Status: `✅ approved`");
        if review_approved {
            let description_approved = std::fs::read_to_string(&description)
                .map(|text| text.lines().any(|line| line == "- Status: ✅ approved"))
                .unwrap_or(false);
            if !description_approved {
                findings
                    .fail("Plan description does not mirror approved adversarial-review status");
            }
            if review_text.lines().any(|line| {
                line.starts_with('|')
                    && line.contains("AR-")
                    && (line.contains("💤 open") || line.contains("⏳ in progress"))
            }) {
                findings.fail("Adversarial review has unresolved findings");
            }
        } else if std::fs::read_to_string(&description)
            .map(|text| text.lines().any(|line| line == "- Status: ✅ approved"))
            .unwrap_or(false)
        {
            findings
                .fail("Plan description claims approval but adversarial review is not approved");
        } else if complete_mode {
            findings.fail("Adversarial review is not approved");
        } else {
            findings.warn("Adversarial review is not approved (expected mid-cycle; use validate-plan.sh --complete for the strict gate)");
        }
    }
    let inventory = plan.join("work-unit-inventory.md");
    for heading in [
        "## Definition-of-done coverage",
        "## Work units",
        "## Decomposition review",
    ] {
        require_heading(&inventory, heading, findings);
    }
    let inventory_text = std::fs::read_to_string(&inventory).unwrap_or_default();
    if inventory_text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("- [") && !trimmed.starts_with("- [x]") && !trimmed.starts_with("- [X]")
    }) {
        findings.fail("Decomposition review contains unchecked items");
    }
    for review in [
        "- [x] Every definition-of-done item maps to one or more work units.",
        "- [x] Every known affected file and changing symbol has its own work unit.",
        "- [x] Every work unit has exactly one goal and one step.",
        "- [x] Each goal has 2–10 work units, or records an allowed exception.",
        "- [x] Each step has exactly one work unit and no unnamed incidental edits.",
        "- [x] Dependencies form an executable order with no cycle.",
    ] {
        if !inventory_text.lines().any(|line| line == review) {
            findings.fail(format!("Missing completed decomposition review: {review}"));
        }
    }
    if inventory_text.to_ascii_lowercase().contains("tbd") {
        findings.fail("Inventory contains TBD; add a bounded discovery work unit instead");
    }
    let mut plan_docs = vec![description, review];
    let mut goal_files = std::fs::read_dir(plan)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path().join("goal.md"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    goal_files.sort();
    for goal_file in goal_files {
        plan_docs.push(goal_file.clone());
        let Some(steps_dir) = goal_file.parent().map(|parent| parent.join("steps")) else {
            continue;
        };
        let mut steps = std::fs::read_dir(steps_dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| !name.ends_with("-testing.md"))
            })
            .collect::<Vec<_>>();
        steps.sort();
        plan_docs.extend(steps);
    }
    plan_docs.push(inventory);
    DocumentState {
        ui_affected,
        review_approved,
        plan_docs,
    }
}

#[cfg(test)]
mod tests {
    use super::validate_obsolete;
    use std::fs;

    #[test]
    fn obsolete_marker_returns_the_shell_refusal_code() {
        let root = std::env::temp_dir().join(format!("validator-docs-{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        fs::write(root.join("OBSOLETE"), "replaced-by: newer-plan\n").unwrap();
        assert_eq!(validate_obsolete(&root, "validate-plan.sh"), Some(65));
        let _ = fs::remove_dir_all(root);
    }
}
