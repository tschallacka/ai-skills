// MODE: DEV
// PACKAGE: PROD
use planning_table::table_cell;
use std::fs;
use std::path::Path;

pub fn progress_percent(completed: i64, total: i64) -> i64 {
    if total > 0 {
        (completed * 100 + total / 2) / total
    } else {
        0
    }
}

pub fn progress_bar(completed: i64, total: i64, width: usize) -> String {
    let percent = progress_percent(completed, total);
    let filled = ((percent * width as i64) / 100).max(0) as usize;
    format!(
        "{}{}",
        "#".repeat(filled),
        "-".repeat(width.saturating_sub(filled))
    )
}

pub fn progress_icon(completed: i64, percent: i64) -> &'static str {
    if percent == 100 {
        "✅"
    } else if completed > 0 {
        "⏳"
    } else {
        "💤"
    }
}

pub fn status_label(status: &str) -> Option<&'static str> {
    match status {
        "incomplete" => Some("💤 incomplete"),
        "in-progress" | "in_progress" => Some("⏳ in progress"),
        "completed" => Some("✅ completed"),
        _ => None,
    }
}

pub fn count_progress_rows(path: &Path, status_column: usize) -> Result<(usize, usize), String> {
    let content =
        fs::read_to_string(path).map_err(|_| format!("no such file: {}", path.display()))?;
    let mut completed = 0;
    let mut total = 0;
    for row in content.lines().filter(|line| line.starts_with('|')) {
        let goal = table_cell(row, 2);
        let status = table_cell(row, status_column);
        if goal == "Goalname"
            || goal.chars().all(|ch| ch == '-')
            || status.chars().all(|ch| ch == '-')
        {
            continue;
        }
        total += 1;
        if status.contains("completed") {
            completed += 1;
        }
    }
    Ok((completed, total))
}

pub fn step_objective(path: &Path, fallback: &str) -> Result<String, String> {
    let content =
        fs::read_to_string(path).map_err(|_| format!("no such file: {}", path.display()))?;
    let mut in_objective = false;
    let mut after_label = false;
    for line in content.lines() {
        if line == "## Objective" {
            in_objective = true;
            continue;
        }
        if in_objective && is_paragraph_label(line) {
            after_label = true;
            continue;
        }
        if after_label && !line.trim().is_empty() {
            let text = line.trim();
            return Ok(if text.chars().count() > 100 {
                format!("{}...", text.chars().take(100).collect::<String>())
            } else {
                text.to_string()
            });
        }
        if in_objective && line.starts_with("## ") {
            break;
        }
    }
    Ok(fallback.to_string())
}

/// One tracker row whose Description cell no longer matches its step's objective.
pub struct StaleRow {
    pub step: String,
    pub current: String,
    pub expected: String,
}

/// The Description cell a goal tracker row must carry for `step`.
pub fn row_objective(steps_dir: &Path, step: &str) -> String {
    step_objective(&steps_dir.join(format!("{step}.md")), step).unwrap_or_else(|_| step.to_string())
}

/// The row's own cell, untrimmed of backticks, so it compares with the objective text as written.
fn raw_cell(row: &str, column: usize) -> &str {
    row.split('|')
        .nth(column.saturating_sub(1))
        .unwrap_or_default()
        .trim()
}

/// A data row of a tracker whose step file exists, with its step name and stale description.
fn stale_row(row: &str, steps_dir: &Path) -> Option<StaleRow> {
    if !row.starts_with('|') || row.split('|').count() < 5 {
        return None;
    }
    let step = table_cell(row, 3);
    if step.is_empty() || !steps_dir.join(format!("{step}.md")).is_file() {
        return None;
    }
    let current = raw_cell(row, 4).to_string();
    let expected = row_objective(steps_dir, &step);
    (current != expected).then_some(StaleRow {
        step,
        current,
        expected,
    })
}

/// The tracker rows whose Description cell differs from their step's objective.
/// A row whose step file is missing is not reported here.
pub fn stale_rows(progress: &str, steps_dir: &Path) -> Vec<StaleRow> {
    progress
        .lines()
        .filter_map(|row| stale_row(row, steps_dir))
        .collect()
}

/// `progress` with every stale Description cell replaced by the step's objective.
/// Every other cell and line is kept byte for byte; returns the text and the rows changed.
pub fn refresh_rows(progress: &str, steps_dir: &Path) -> (String, usize) {
    let mut output = String::with_capacity(progress.len());
    let mut changed = 0;
    for line in progress.split_inclusive('\n') {
        let (body, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        match stale_row(body, steps_dir) {
            Some(stale) => {
                let mut cells: Vec<String> = body.split('|').map(str::to_string).collect();
                cells[3] = format!(" {} ", stale.expected);
                output.push_str(&cells.join("|"));
                changed += 1;
            }
            None => output.push_str(body),
        }
        output.push_str(newline);
    }
    (output, changed)
}

/// Refresh one goal's `progress.md` in place. `Ok(None)` when the goal has no tracker;
/// otherwise the number of rows changed (the file is rewritten only when that is non-zero).
pub fn refresh_goal_rows(goal_dir: &Path) -> Result<Option<usize>, String> {
    let progress = goal_dir.join("progress.md");
    if !progress.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&progress).map_err(|error| error.to_string())?;
    let (updated, changed) = refresh_rows(&text, &goal_dir.join("steps"));
    if changed > 0 {
        planning_core::atomic_write(&progress, updated.as_bytes())?;
    }
    Ok(Some(changed))
}

fn is_paragraph_label(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("§ ") else {
        return false;
    };
    let Some((section, paragraph)) = rest.split_once('.') else {
        return false;
    };
    !section.is_empty()
        && !paragraph.is_empty()
        && section.chars().all(|ch| ch.is_ascii_digit())
        && paragraph.chars().all(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{count_progress_rows, progress_bar, progress_icon, progress_percent, status_label};
    use std::fs;

    #[test]
    fn percentage_uses_half_up_rounding() {
        assert_eq!(progress_percent(1, 3), 33);
        assert_eq!(progress_percent(2, 3), 67);
        assert_eq!(progress_percent(0, 0), 0);
    }

    #[test]
    fn renderers_keep_the_documented_glyphs() {
        assert_eq!(progress_bar(1, 2, 4), "##--");
        assert_eq!(progress_icon(0, 0), "💤");
        assert_eq!(progress_icon(1, 50), "⏳");
        assert_eq!(progress_icon(1, 100), "✅");
        assert_eq!(status_label("in_progress"), Some("⏳ in progress"));
    }

    #[test]
    fn count_skips_headers_and_separator_rows() {
        let path = std::env::temp_dir().join(format!(
            "planning-progress-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, "| Goalname | Stepname | Description | Completion status |\n| --- | --- | --- | --- |\n| G1 | one | x | ✅ completed |\n| G2 | two | x | ⏳ in progress |\n").unwrap();
        assert_eq!(count_progress_rows(&path, 5).unwrap(), (1, 2));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refresh_rewrites_a_stale_objective_and_keeps_the_status() {
        let root = std::env::temp_dir().join(format!(
            "planning-progress-refresh-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let steps = root.join("steps");
        fs::create_dir_all(&steps).unwrap();
        fs::write(
            steps.join("01-step-one.md"),
            "# Step\n\n## Objective\n\n§ 4.1\nNew objective text.\n\n## Instructions\n",
        )
        .unwrap();
        let tracker = "| Goalname | Stepname | Description | Completion status |\n|---|---|---|---|\n| G | 01-step-one | Old text | ✅ completed |\n";

        let stale = super::stale_rows(tracker, &steps);
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].current, "Old text");
        assert_eq!(stale[0].expected, "New objective text.");

        let (refreshed, changed) = super::refresh_rows(tracker, &steps);
        assert_eq!(changed, 1);
        assert_eq!(
            refreshed,
            "| Goalname | Stepname | Description | Completion status |\n|---|---|---|---|\n| G | 01-step-one | New objective text. | ✅ completed |\n"
        );
        let (_, again) = super::refresh_rows(&refreshed, &steps);
        assert_eq!(again, 0);
        fs::remove_dir_all(root).unwrap();
    }
}
