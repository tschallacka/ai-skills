// MODE: DEV
// PACKAGE: PROD

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryRow {
    pub id: String,
    pub kind: String,
    pub file: String,
    pub scope: String,
    pub subscope: String,
    pub change: String,
    pub depends: String,
    pub goal: String,
    pub step: String,
}

impl InventoryRow {
    pub fn parse(line: &str) -> Option<Self> {
        if !line.starts_with('|') {
            return None;
        }
        let cells: Vec<String> = line
            .split('|')
            .skip(1)
            .take_while(|_| true)
            .map(clean_cell)
            .collect();
        if cells.len() < 9 || !is_unit_id(&cells[0]) {
            return None;
        }
        Some(Self {
            id: cells[0].clone(),
            kind: cells[1].clone(),
            file: cells[2].clone(),
            scope: cells[3].clone(),
            subscope: cells[4].clone(),
            change: cells[5].clone(),
            depends: cells[6].clone(),
            goal: cells[7].clone(),
            step: cells[8].clone(),
        })
    }
}

pub fn rows(content: &str) -> impl Iterator<Item = InventoryRow> + '_ {
    content.lines().filter_map(InventoryRow::parse)
}

pub fn find(content: &str, id: &str) -> Option<InventoryRow> {
    rows(content).find(|row| row.id == id)
}

pub fn is_unit_id(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next() == Some('W') && chars.clone().count() >= 2 && chars.all(|ch| ch.is_ascii_digit())
}

pub fn clean_cell(value: &str) -> String {
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix('`').unwrap_or(trimmed);
    trimmed
        .strip_suffix('`')
        .unwrap_or(trimmed)
        .replace('\t', " ")
}

pub fn set_cell(line: &str, column: usize, value: &str) -> Option<String> {
    if column < 1 || !line.starts_with('|') {
        return None;
    }
    let mut parts: Vec<&str> = line.split('|').collect();
    let index = column.checked_sub(1)?;
    if index >= parts.len() {
        return None;
    }
    parts[index] = value;
    Some(parts.join("|"))
}

pub fn update_row<F>(content: &str, id: &str, mut update: F) -> (String, usize)
where
    F: FnMut(&mut Vec<String>),
{
    let mut count = 0;
    let mut output = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        let (body, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        let mut replacement = body.to_string();
        if InventoryRow::parse(body).is_some_and(|row| row.id == id) {
            let mut parts: Vec<String> = body.split('|').map(str::to_string).collect();
            update(&mut parts);
            replacement = parts.join("|");
            count += 1;
        }
        output.push_str(&replacement);
        output.push_str(newline);
    }
    (output, count)
}

#[cfg(test)]
mod tests {
    use super::{find, set_cell, InventoryRow};

    #[test]
    fn parses_and_cleans_inventory_cells() {
        let row = InventoryRow::parse(
            "| W01 | source | `src/a.rs` | fn() | N/A | change\there | — | 01-g | 02-s |",
        )
        .unwrap();
        assert_eq!(row.file, "src/a.rs");
        assert_eq!(row.change, "change here");
        assert_eq!(
            find(
                "| W01 | source | file | scope | sub | change | — | g | s |",
                "W01"
            ),
            Some(InventoryRow {
                id: "W01".into(),
                kind: "source".into(),
                file: "file".into(),
                scope: "scope".into(),
                subscope: "sub".into(),
                change: "change".into(),
                depends: "—".into(),
                goal: "g".into(),
                step: "s".into(),
            })
        );
    }

    #[test]
    fn sets_shell_compatible_cell_number() {
        assert_eq!(
            set_cell("| W01 | source | file |", 3, " new ").unwrap(),
            "| W01 | new | file |"
        );
        assert!(super::is_unit_id("W100"));
    }
}
