// MODE: DEV
// PACKAGE: PROD
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub fn table_cell(row: &str, column: usize) -> String {
    row.split('|')
        .nth(column.saturating_sub(1))
        .unwrap_or_default()
        .trim()
        .trim_matches('`')
        .to_string()
}

pub fn goal_definition_of_done(path: &std::path::Path, fallback: &str) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut in_section = false;
    for line in text.lines() {
        if line == "## Outcome and definition of done" {
            in_section = true;
            continue;
        }
        if in_section && line.starts_with("## ") {
            break;
        }
        if !in_section || line.trim().is_empty() {
            continue;
        }
        if line
            .strip_prefix("§ ")
            .map(|value| {
                let mut parts = value.split_whitespace();
                let Some(number) = parts.next() else {
                    return false;
                };
                parts.next().is_none()
                    && number.split_once('.').is_some_and(|(a, b)| {
                        !a.is_empty()
                            && !b.is_empty()
                            && a.bytes().all(|byte| byte.is_ascii_digit())
                            && b.bytes().all(|byte| byte.is_ascii_digit())
                    })
            })
            .unwrap_or(false)
        {
            continue;
        }
        let value = line.trim_start();
        if value.len() > 100 {
            return format!("{}...", &value[..100]);
        }
        return value.to_string();
    }
    fallback.to_string()
}

pub fn table_cells(row: &str) -> Vec<String> {
    row.split('|')
        .skip(1)
        .take_while(|_| true)
        .collect::<Vec<_>>()
        .into_iter()
        .take_while(|_| true)
        .map(|cell| cell.trim().trim_matches('`').to_string())
        .filter(|cell| !cell.is_empty())
        .collect()
}

pub fn review_finding_ids(path: &Path) -> Result<Vec<String>, String> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Ok(Vec::new()),
    };
    let mut in_findings = false;
    let mut ids = BTreeSet::new();
    for line in content.lines() {
        if line.starts_with("## Findings") {
            in_findings = true;
            continue;
        }
        if in_findings && line.starts_with("## ") {
            in_findings = false;
        }
        if in_findings && line.starts_with('|') {
            let id = table_cell(line, 2);
            if is_finding_id(&id) {
                ids.insert(id);
            }
        }
    }
    Ok(ids.into_iter().collect())
}

pub fn review_gated_pairs(path: &Path) -> Result<Vec<(String, String)>, String> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Ok(Vec::new()),
    };
    let mut in_findings = false;
    let mut pairs = Vec::new();
    for line in content.lines() {
        if line == "## Findings" {
            in_findings = true;
            continue;
        }
        if in_findings && line == "## Verdict" {
            break;
        }
        if in_findings && line.starts_with('|') {
            let finding = table_cell(line, 2);
            let work_unit = table_cell(line, 6);
            if is_finding_id(&finding) && is_work_unit_id(&work_unit) {
                pairs.push((finding, work_unit));
            }
        }
    }
    Ok(pairs)
}

pub fn testing_requirement(path: &Path) -> Result<Option<String>, String> {
    let content =
        fs::read_to_string(path).map_err(|_| format!("no such file: {}", path.display()))?;
    let mut in_section = false;
    for line in content.lines() {
        if line == "## Testing requirement" {
            in_section = true;
            continue;
        }
        if in_section && line.starts_with("## ") {
            break;
        }
        if in_section {
            let value = table_cell(line, 2);
            if value == "yes" || value == "no" {
                return Ok(Some(value));
            }
        }
    }
    Ok(None)
}

pub fn replace_testing_requirement(
    document: &str,
    required: &str,
    rationale: &str,
) -> Result<String, String> {
    if required != "yes" && required != "no" {
        return Err("Test requirement must be yes or no".into());
    }
    let mut in_section = false;
    let mut header = false;
    let mut separator = false;
    let mut rows = 0;
    let mut output = String::with_capacity(document.len());
    for line in document.split_inclusive('\n') {
        let (body, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        if body == "## Testing requirement" {
            in_section = true;
        } else if in_section && body.starts_with("## ") {
            in_section = false;
        } else if in_section && body == "| Test required | Rationale |" {
            header = true;
        } else if in_section && header && body == "|---|---|" {
            separator = true;
        } else if in_section && separator && body.starts_with('|') && body.ends_with('|') {
            rows += 1;
            if rows > 1 {
                return Err(format!(
                    "Testing requirement table was not found exactly once: {required}"
                ));
            }
            output.push_str(&format!("| {required} | {rationale} |"));
            output.push_str(newline);
            continue;
        }
        output.push_str(body);
        output.push_str(newline);
    }
    if !header || !separator || rows != 1 {
        return Err("Testing requirement table was not found exactly once".into());
    }
    Ok(output)
}

pub fn csv_to_markdown(columns: usize, csv: &str) -> Result<String, CsvError> {
    if columns == 0 {
        return Err(CsvError::InvalidColumnCount);
    }
    let csv = csv.replace("\\n", "\n");
    let mut output = String::new();
    let mut row_number = 0;
    for line in csv.lines() {
        row_number += 1;
        if line.trim().is_empty() {
            return Err(CsvError::BlankRow(row_number));
        }
        let fields = parse_csv_line(line).map_err(|_| CsvError::UnbalancedQuote(row_number))?;
        if fields.len() != columns {
            return Err(CsvError::WrongColumnCount(
                row_number,
                fields.len(),
                columns,
            ));
        }
        output.push('|');
        for (index, field) in fields.iter().enumerate() {
            let cleaned = field.replace("\\|", "");
            if cleaned.contains('|') {
                return Err(CsvError::UnescapedPipe(row_number, index + 1));
            }
            if field.contains('\r') {
                return Err(CsvError::CarriageReturn(row_number));
            }
            output.push(' ');
            output.push_str(field);
            output.push_str(" |");
        }
        output.push('\n');
        if row_number == 1 {
            output.push('|');
            for _ in 0..columns {
                output.push_str("---|");
            }
            output.push('\n');
        }
    }
    if row_number == 0 {
        return Err(CsvError::EmptyInput(columns));
    }
    Ok(output)
}

#[derive(Debug, PartialEq, Eq)]
pub enum CsvError {
    InvalidColumnCount,
    UnbalancedQuote(usize),
    WrongColumnCount(usize, usize, usize),
    UnescapedPipe(usize, usize),
    BlankRow(usize),
    EmptyInput(usize),
    CarriageReturn(usize),
}

fn parse_csv_line(line: &str) -> Result<Vec<String>, ()> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let chars: Vec<char> = line.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '"' {
            if quoted && chars.get(index + 1) == Some(&'"') {
                field.push('"');
                index += 1;
            } else {
                quoted = !quoted;
            }
        } else if ch == ',' && !quoted {
            fields.push(field);
            field = String::new();
        } else {
            field.push(ch);
        }
        index += 1;
    }
    if quoted {
        return Err(());
    }
    fields.push(field);
    Ok(fields)
}

fn is_finding_id(value: &str) -> bool {
    value
        .strip_prefix("AR-")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_work_unit_id(value: &str) -> bool {
    value
        .strip_prefix('W')
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::{
        csv_to_markdown, replace_testing_requirement, review_gated_pairs, table_cell, CsvError,
    };
    use std::fs;

    #[test]
    fn cells_use_the_shell_library_one_based_columns() {
        assert_eq!(table_cell("| `G1` | text |", 2), "G1");
    }

    #[test]
    fn csv_renderer_matches_markdown_shape() {
        assert_eq!(
            csv_to_markdown(2, "a,b\nc,d\n").unwrap(),
            "| a | b |\n|---|---|\n| c | d |\n"
        );
        assert_eq!(
            csv_to_markdown(2, "a\n"),
            Err(CsvError::WrongColumnCount(1, 1, 2))
        );
    }

    #[test]
    fn gated_pairs_stop_at_verdict() {
        let path = std::env::temp_dir().join(format!(
            "planning-table-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, "## Findings\n| ID | Missing | Required | Status | Work unit |\n| AR-01 | x | y | open | W01 |\n## Verdict\n| AR-02 | x | y | open | W02 |\n").unwrap();
        assert_eq!(
            review_gated_pairs(&path).unwrap(),
            vec![("AR-01".into(), "W01".into())]
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn csv_decodes_shell_newline_escapes() {
        assert_eq!(
            csv_to_markdown(2, "a,b\\nc,d").unwrap(),
            "| a | b |\n|---|---|\n| c | d |\n"
        );
    }

    #[test]
    fn testing_requirement_replaces_only_its_data_row() {
        let document = "## Testing requirement\n\n| Test required | Rationale |\n|---|---|\n| no | old |\n\n## Next\n";
        assert_eq!(
            replace_testing_requirement(document, "yes", "new").unwrap(),
            "## Testing requirement\n\n| Test required | Rationale |\n|---|---|\n| yes | new |\n\n## Next\n"
        );
    }
}
