// MODE: DEV
// PACKAGE: PROD
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

pub fn table_cell(row: &str, column: usize) -> String {
    row.split('|')
        .nth(column.saturating_sub(1))
        .unwrap_or_default()
        .trim()
        .trim_matches('`')
        .to_string()
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
    use std::path::PathBuf;

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
        let path = PathBuf::from(format!("/tmp/planning-progress-{}", std::process::id()));
        fs::write(&path, "| Goalname | Stepname | Description | Completion status |\n| --- | --- | --- | --- |\n| G1 | one | x | ✅ completed |\n| G2 | two | x | ⏳ in progress |\n").unwrap();
        assert_eq!(count_progress_rows(&path, 5).unwrap(), (1, 2));
        fs::remove_file(path).unwrap();
    }
}
