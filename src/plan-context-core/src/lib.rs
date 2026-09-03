// MODE: DEV
// PACKAGE: PROD
use plan_crypt::sha256::Sha256;
use planning_table::{table_cell, table_cells};
use std::fs;
use std::path::{Path, PathBuf};

pub const CONTEXT_SCHEMA_VERSION: u8 = 1;
pub const CONTEXT_GENERATOR_VERSION: u8 = 1;
pub const CONTEXT_RESULT_SCHEMA_VERSION: u8 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoryRow {
    pub id: String,
    pub kind: String,
    pub file: String,
    pub scope: String,
    pub subscope: String,
    pub intended_change: String,
    pub depends_on: String,
    pub goal: String,
    pub step: String,
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    plan_crypt::sha256::hex(&digest.finish())
}

pub fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    Ok(hash_bytes(&bytes))
}

pub fn inventory_rows(path: &Path) -> Result<Vec<InventoryRow>, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    Ok(content
        .lines()
        .filter(|line| is_inventory_row(line))
        .filter_map(|line| {
            let cells = table_cells(line);
            (cells.len() >= 9).then(|| InventoryRow {
                id: table_cell(line, 2),
                kind: table_cell(line, 3),
                file: table_cell(line, 4),
                scope: table_cell(line, 5),
                subscope: table_cell(line, 6),
                intended_change: table_cell(line, 7),
                depends_on: table_cell(line, 8),
                goal: table_cell(line, 9),
                step: table_cell(line, 10),
            })
        })
        .collect())
}

pub fn is_inventory_row(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('|') {
        return false;
    }
    let id = table_cell(trimmed, 2);
    let Some(number) = id.strip_prefix('W') else {
        return false;
    };
    !number.is_empty() && number.chars().all(|character| character.is_ascii_digit())
}

pub fn resolve_document(plan: &Path, id: &str) -> Result<PathBuf, String> {
    let path = match id {
        "plan" => plan.join("plan-description.md"),
        "inventory" | "coverage" => plan.join("work-unit-inventory.md"),
        "progress" => plan.join("progress.md"),
        "adversarial-review" => plan.join("adversarial-review.md"),
        "stories" => plan.join("ui-user-stories.md"),
        "bugs" => plan.join("bugs.md"),
        "planning-bugs" => plan.join("planning-bugs.json"),
        "fixes" => plan.join("fixes.md"),
        "fix-keys" | "fixkeys" => plan.join("fix-keys.json"),
        "approval" => plan.join("approval.json"),
        value if value.starts_with("goal-progress:") => {
            let goal = value.strip_prefix("goal-progress:").unwrap();
            if goal.is_empty() {
                return Err(format!("usage: invalid goal-progress entry: {id}"));
            }
            plan.join(goal).join("progress.md")
        }
        value if value.starts_with("goal:") => plan
            .join(value.strip_prefix("goal:").unwrap())
            .join("goal.md"),
        value if value.starts_with("step:") => {
            let value = value.strip_prefix("step:").unwrap();
            let Some((goal, step)) = value.split_once('/') else {
                return Err(format!("usage: invalid step entry: {id}"));
            };
            if goal.is_empty() || step.is_empty() {
                return Err(format!("usage: invalid step entry: {id}"));
            }
            plan.join(goal).join("steps").join(format!("{step}.md"))
        }
        value if value.starts_with("unit:W") => {
            let unit = value.strip_prefix("unit:").unwrap();
            let rows = inventory_rows(&plan.join("work-unit-inventory.md"))?;
            let row = rows
                .into_iter()
                .find(|row| row.id == unit)
                .ok_or_else(|| format!("not-found: work unit {unit}"))?;
            plan.join(row.goal)
                .join("steps")
                .join(format!("{}.md", row.step))
        }
        value => return Err(format!("usage: unsupported entry id: {value}")),
    };
    Ok(path)
}

pub fn entry_inputs(plan: &Path, id: &str) -> Result<Vec<PathBuf>, String> {
    let mut inputs = vec![resolve_document(plan, id)?];
    if id.starts_with("unit:") {
        inputs.push(plan.join("work-unit-inventory.md"));
    }
    Ok(inputs)
}

pub fn entry_hash(plan: &Path, id: &str) -> Result<String, String> {
    let inputs = entry_inputs(plan, id)?;
    if inputs.len() == 1 {
        return hash_file(&inputs[0]);
    }
    let mut combined = Vec::new();
    for input in inputs {
        if input.is_file() {
            combined.extend_from_slice(hash_file(&input)?.as_bytes());
        } else {
            combined.extend_from_slice(b"absent\n");
        }
        combined.push(b'\n');
    }
    Ok(hash_bytes(&combined))
}

pub fn inventory_row_text(plan: &Path, unit: &str) -> Result<String, String> {
    let row = inventory_rows(&plan.join("work-unit-inventory.md"))?
        .into_iter()
        .find(|row| row.id == unit)
        .ok_or_else(|| format!("not-found: work unit {unit}"))?;
    Ok(format!(
        "## Inventory row\n- ID: {}\n- Type: {}\n- File: {}\n- Scope: {}\n- Subscope: {}\n- Intended change: {}\n- Depends on: {}\n- Goal: {}\n- Step: {}",
        row.id, row.kind, row.file, row.scope, row.subscope, row.intended_change,
        row.depends_on, row.goal, row.step
    ))
}

pub fn valid_section_label(line: &str) -> bool {
    let Some(label) = line.strip_prefix("§ ") else {
        return false;
    };
    let Some((major, minor)) = label.split_once('.') else {
        return false;
    };
    !major.is_empty()
        && !minor.is_empty()
        && major.chars().all(|character| character.is_ascii_digit())
        && minor.chars().all(|character| character.is_ascii_digit())
}

pub fn view_text(path: &Path, view: &str, row_text: Option<&str>) -> Result<String, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    match view {
        "full" => Ok(content),
        "inventory-row" => row_text
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| "usage: inventory-row view applies only to a work unit".into()),
        "summary" => Ok(content.lines().take(12).collect::<Vec<_>>().join("\n")),
        "metadata" => range_view(
            &content,
            |line| line == "## Ownership" || line == "## Change target",
            "## Objective",
        ),
        "ownership" => range_view(&content, |line| line == "## Ownership", "## Change target"),
        "instructions" => labelled_paragraph(&content, "## Instructions", "§ 5.1"),
        "acceptance" => labelled_paragraph(&content, "## Acceptance criteria", "§ 6.1"),
        "handoff" => labelled_paragraph(&content, "## Handoff", "§ 7.1"),
        "dependencies" => Ok(content
            .lines()
            .filter(|line| line.to_ascii_lowercase().contains("depends on"))
            .chain(
                row_text
                    .unwrap_or_default()
                    .lines()
                    .filter(|line| line.to_ascii_lowercase().contains("depends on")),
            )
            .collect::<Vec<_>>()
            .join("\n")),
        "testing" => {
            let companion = path.with_file_name(format!(
                "{}-testing.md",
                path.file_stem().unwrap_or_default().to_string_lossy()
            ));
            let testing = fs::read_to_string(&companion)
                .map_err(|_| format!("usage: testing view unavailable for {}", path.display()))?;
            let mut active = false;
            Ok(testing
                .lines()
                .filter(|line| {
                    if *line == "## Automated tests" {
                        active = true;
                        return false;
                    }
                    active && !line.trim().is_empty()
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "execution-summary" => {
            let row = row_text.filter(|text| !text.is_empty()).ok_or_else(|| {
                "usage: execution-summary view applies only to a work unit".to_string()
            })?;
            let acceptance = labelled_paragraph(&content, "## Acceptance criteria", "§ 6.1")?;
            let dependencies = content
                .lines()
                .filter(|line| line.to_ascii_lowercase().contains("depends on"))
                .chain(
                    row.lines()
                        .filter(|line| line.to_ascii_lowercase().contains("depends on")),
                )
                .collect::<Vec<_>>()
                .join("\n");
            let companion = path.with_file_name(format!(
                "{}-testing.md",
                path.file_stem().unwrap_or_default().to_string_lossy()
            ));
            let testing = fs::read_to_string(&companion)
                .map(|value| value.lines().collect::<Vec<_>>().join("\n"))
                .unwrap_or_else(|_| "No testing companion found for this step.".into());
            Ok(format!(
                "## Execution summary\n{}\n\n## Inventory\n{}\n\n## Acceptance criteria\n{}\n\n## Dependencies\n{}\n\n## Testing\n{}\n\n## Status\nRead the owning goal progress tracker for current completion status.",
                range_view(&content, |line| line == "## Ownership" || line == "## Change target", "## Objective")?,
                row,
                acceptance,
                dependencies,
                testing
            ))
        }
        "changed-documents" => Ok(content
            .lines()
            .filter(|line| {
                line.starts_with('#')
                    || valid_section_label(line)
                    || line.starts_with('-')
                    || line.starts_with('*')
            })
            .take(20)
            .collect::<Vec<_>>()
            .join("\n")),
        _ => Err(format!(
            "usage: unsupported view: {view} -- valid views are full, summary, metadata, ownership, instructions, acceptance, handoff, testing, dependencies, execution-summary, changed-documents, inventory-row, and validator. Use --view summary for a bounded overview or --view full with paging for complete content."
        )),
    }
}

fn range_view<F>(content: &str, start: F, end: &str) -> Result<String, String>
where
    F: Fn(&str) -> bool,
{
    let mut active = false;
    let mut output = Vec::new();
    for line in content.lines() {
        if start(line) {
            active = true;
        }
        if active && line == end {
            break;
        }
        if active {
            output.push(line);
        }
    }
    if output.last().is_some_and(|line| line.is_empty()) {
        output.pop();
    }
    Ok(output.join("\n"))
}

fn labelled_paragraph(content: &str, heading: &str, label: &str) -> Result<String, String> {
    let mut active = false;
    let mut found = false;
    for line in content.lines() {
        if line == heading {
            active = true;
            continue;
        }
        if active && line == label {
            found = true;
            continue;
        }
        if found {
            return Ok(line.to_string());
        }
    }
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::{hash_bytes, is_inventory_row, valid_section_label};

    #[test]
    fn hash_matches_sha256_vector() {
        assert_eq!(
            hash_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn inventory_row_detection_requires_numeric_work_unit_id() {
        assert!(is_inventory_row(
            "| W01 | source | file | scope | N/A | change | — | goal | step |"
        ));
        assert!(!is_inventory_row(
            "| W | source | file | scope | N/A | change | — | goal | step |"
        ));
        assert!(!is_inventory_row(
            "| W01x | source | file | scope | N/A | change | — | goal | step |"
        ));
    }

    #[test]
    fn section_labels_are_strict() {
        assert!(valid_section_label("§ 5.1"));
        assert!(!valid_section_label("§ 5.x"));
        assert!(!valid_section_label("§ 5.1 trailing"));
    }
}
