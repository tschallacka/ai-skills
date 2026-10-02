// MODE: DEV
// PACKAGE: PROD
use super::state::{Coverage, Edge, Finding, Goal, Identity, State, Step, TestingMark};
use super::tree::{PlanDocument, PlanTree};
use std::path::Path;

fn section(text: &str, heading: &str) -> String {
    let marker = format!("## {heading}");
    let mut active = false;
    let mut result = Vec::new();
    for line in text.lines() {
        if line == marker {
            active = true;
            continue;
        }
        if active && line.starts_with("## ") {
            break;
        }
        if active {
            result.push(line);
        }
    }
    result.join("\n").trim_end().to_string()
}

fn field(text: &str, name: &str) -> String {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("- {name}: `")))
        .map(|value| {
            value
                .chars()
                .filter(|character| *character != '`')
                .collect()
        })
        .unwrap_or_default()
}

fn testing_requirement(text: &str) -> String {
    let mut inside = false;
    let mut result = String::new();
    for line in text.lines() {
        if line.contains("Testing requirement") || line.contains("Testing-requirement") {
            inside = true;
            continue;
        }
        if inside && line.starts_with("## ") {
            break;
        }
        if !inside || !line.starts_with('|') || line.contains("Goal") || line.contains("---") {
            continue;
        }
        let cells: Vec<_> = line.split('|').collect();
        let Some(raw_key) = cells.get(1) else {
            continue;
        };
        let Some(value) = cells.get(2) else {
            continue;
        };
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(raw_key);
        result.push_str(": ");
        result.push_str(value.trim());
    }
    result
}

fn cells(line: &str) -> Vec<String> {
    line.trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn document<'a>(tree: &'a PlanTree, path: &Path) -> Option<&'a PlanDocument> {
    tree.documents.iter().find(|doc| doc.path == path)
}

pub fn extract_state(tree: &PlanTree) -> Result<String, String> {
    extract_state_as(tree, "plan-overview")
}

pub fn extract_state_as(tree: &PlanTree, generated_by: &str) -> Result<String, String> {
    let description = document(tree, &tree.root.join("plan-description.md"))
        .ok_or_else(|| "plan-description.md not found".to_string())?;
    let review = document(tree, &tree.root.join("adversarial-review.md"));
    let title = description
        .contents
        .lines()
        .find_map(|line| line.strip_prefix("# Plan: "))
        .unwrap_or_default()
        .to_string();
    let ui_affected = description
        .contents
        .lines()
        .find_map(|line| line.strip_prefix("- UI affected: "))
        .unwrap_or("no")
        .trim()
        .to_string();
    // B343: the real line is `- Status: \`✅ approved\`` (or `💤 pending`) --
    // a backtick-fenced code span, not a bare value. Only stripping the
    // leading "- Status: `" left the closing backtick in the extracted
    // value ("✅ approved`"), which then never matched derive_mode's
    // literal comparison either.
    let review_status = review
        .and_then(|doc| {
            doc.contents.lines().find_map(|line| {
                line.strip_prefix("- Status: `")
                    .and_then(|rest| rest.strip_suffix('`'))
            })
        })
        .unwrap_or_default()
        .to_string();

    let mut goals = Vec::new();
    let mut steps = Vec::new();
    for doc in &tree.documents {
        let relative = doc.path.strip_prefix(&tree.root).unwrap_or(&doc.path);
        let components: Vec<_> = relative.components().collect();
        if components.len() == 2 && components[1].as_os_str() == "goal.md" {
            let id = components[0].as_os_str().to_string_lossy().to_string();
            let testing_requirement = testing_requirement(&doc.contents);
            goals.push(Goal {
                id,
                outcome: section(&doc.contents, "Outcome and definition of done"),
                testing_requirement,
            });
        }
    }
    goals.sort_by(|a, b| a.id.cmp(&b.id));

    for doc in tree.documents.iter().filter(|doc| {
        let relative = doc.path.strip_prefix(&tree.root).unwrap_or(&doc.path);
        let components: Vec<_> = relative.components().collect();
        if components.len() < 3 || components[components.len() - 2].as_os_str() != "steps" {
            return false;
        }
        let stem = doc
            .path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();
        !planning_document::is_testing_companion(&stem, |base| {
            document(tree, &doc.path.with_file_name(format!("{base}.md"))).is_some()
        })
    }) {
        let relative = doc.path.strip_prefix(&tree.root).unwrap_or(&doc.path);
        let mut parts = relative.components();
        let goal = parts
            .next()
            .unwrap()
            .as_os_str()
            .to_string_lossy()
            .to_string();
        let step = doc.path.file_stem().unwrap().to_string_lossy().to_string();
        let companion_path = doc.path.with_file_name(format!("{step}-testing.md"));
        let status = tree
            .document(format!("{goal}/progress.md"))
            .and_then(|progress| {
                progress
                    .contents
                    .lines()
                    .find(|line| line.contains(&format!("| {step} |")))
            })
            .map(|line| {
                if line.contains("✅ completed") {
                    "completed"
                } else if line.contains("⏳ in progress") {
                    "in-progress"
                } else if line.contains("💤 incomplete") {
                    "incomplete"
                } else {
                    "unknown"
                }
            })
            .unwrap_or("unknown")
            .to_string();
        let companion_doc = document(tree, &companion_path);
        steps.push(Step {
            goal,
            step,
            unit: field(&doc.contents, "Work unit"),
            kind: field(&doc.contents, "Type"),
            target: field(&doc.contents, "File"),
            companion: companion_doc.map(|_| {
                companion_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            }),
            // The companion's own real content (every "## " verification
            // section: Automated tests, and Browser/Backend/Manual when
            // present), not merely its filename -- companion above only
            // records THAT one exists, which previously stood in for the
            // actual procedure text on the rendered test/unit pages.
            testing_procedure: companion_doc.map(|companion| {
                companion
                    .contents
                    .lines()
                    .skip_while(|line| !line.starts_with("## "))
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim()
                    .to_string()
            }),
            status,
            instructions: section(&doc.contents, "Instructions"),
            criteria: section(&doc.contents, "Acceptance criteria"),
        });
    }
    steps.sort_by(|a, b| a.goal.cmp(&b.goal).then_with(|| a.step.cmp(&b.step)));

    let inventory = document(tree, &tree.root.join("work-unit-inventory.md"));
    let mut edges = Vec::new();
    if let Some(inventory) = inventory {
        for line in inventory
            .contents
            .lines()
            .filter(|line| line.starts_with("| W"))
        {
            let row = cells(line);
            if line.chars().filter(|character| *character == '|').count() + 1 < 10
                || row.len() < 7
                || row[0].is_empty()
                || row[6].is_empty()
                || row[6] == "—"
            {
                continue;
            }
            for dependency in row[6]
                .split(',')
                .map(str::trim)
                .filter(|value| value.starts_with('W'))
            {
                edges.push(Edge {
                    from: row[0].clone(),
                    to: dependency.to_string(),
                });
            }
        }
    }
    let testing_marks = goals
        .iter()
        .filter(|goal| goal.testing_requirement == "yes")
        .flat_map(|goal| {
            steps
                .iter()
                .filter(move |step| step.goal == goal.id && step.companion.is_none())
                .map(move |step| TestingMark {
                    goal: goal.id.clone(),
                    step: step.step.clone(),
                })
        })
        .collect();

    let coverage = inventory
        .map(|doc| {
            doc.contents
                .lines()
                .skip_while(|line| *line != "## Definition-of-done coverage")
                .skip(1)
                .take_while(|line| !line.starts_with("## "))
                .filter_map(|line| {
                    if !line.starts_with('|')
                        || line.contains("---")
                        || line.contains("Required outcome")
                    {
                        return None;
                    }
                    let row = cells(line);
                    (row.len() >= 2).then(|| Coverage {
                        outcome: row[0].clone(),
                        units: row[1].clone(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut findings = Vec::new();
    if let Some(review) = review {
        let mut in_findings = false;
        for line in review.contents.lines() {
            if line == "## Findings" {
                in_findings = true;
                continue;
            }
            if in_findings && line.starts_with("## ") {
                break;
            }
            if !in_findings || !line.starts_with("| AR") {
                continue;
            }
            let row = cells(line);
            if row.len() >= 5 {
                findings.push(Finding {
                    id: row[0].clone(),
                    item: row[1].clone(),
                    change: row[2].clone(),
                    status: row[3].clone(),
                    work_unit: row[4].clone(),
                    cycle: if tree.document("adversarial-review-history.md").is_some_and(
                        |history| {
                            history.contents.lines().any(|history_line| {
                                history_line.contains(&format!("| {} |", row[0]))
                            })
                        },
                    ) {
                        "archived".into()
                    } else {
                        "current".into()
                    },
                });
            }
        }
    }
    // Every prior cycle's own findings, archived in adversarial-review-
    // history.md under its own "## Cycle N" heading, in the same five-column
    // shape as the live table above. An id recurs across cycles as it is
    // found, refined, and resolved -- the LAST (highest-numbered) cycle's own
    // text is kept per id, since that is the finding's final, resolved
    // account, not each intermediate restatement. Skips any id the live
    // table above already carries, so a currently-open finding is shown once,
    // from its authoritative live row, not also from wherever it was first
    // raised.
    if let Some(history) = tree.document("adversarial-review-history.md") {
        let mut by_id: std::collections::BTreeMap<String, Finding> =
            std::collections::BTreeMap::new();
        let mut cycle_label = String::new();
        for line in history.contents.lines() {
            if let Some(rest) = line.strip_prefix("## ") {
                cycle_label = rest.trim().to_string();
                continue;
            }
            if !line.starts_with("| AR") {
                continue;
            }
            let row = cells(line);
            if row.len() >= 5 {
                by_id.insert(
                    row[0].clone(),
                    Finding {
                        id: row[0].clone(),
                        item: row[1].clone(),
                        change: row[2].clone(),
                        status: row[3].clone(),
                        work_unit: row[4].clone(),
                        cycle: cycle_label.clone(),
                    },
                );
            }
        }
        let live_ids: std::collections::HashSet<String> =
            findings.iter().map(|finding| finding.id.clone()).collect();
        findings.extend(
            by_id
                .into_values()
                .filter(|finding| !live_ids.contains(&finding.id)),
        );
    }
    let cycles = tree
        .documents
        .iter()
        .find(|doc| doc.path.ends_with("adversarial-review-history.md"))
        .map(|doc| {
            doc.contents
                .lines()
                .filter(|line| line.starts_with("## Cycle "))
                .count() as u32
        })
        .unwrap_or(0);
    let state = State {
        identity: Identity {
            title,
            ui_affected,
            review_status,
            description: format!(
                "{}{}",
                section(&description.contents, "Current state"),
                section(&description.contents, "Desired outcome")
            ),
        },
        goals,
        steps,
        edges,
        testing_marks,
        coverage,
        findings,
        cycles,
        review_target: 2,
        generated_at: std::env::var("OVERVIEW_NOW").unwrap_or_else(|_| "serve-live".into()),
        generated_by: generated_by.into(),
    };
    serde_json::to_string(&state).map_err(|error| error.to_string())
}
