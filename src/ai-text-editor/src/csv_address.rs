// MODE: DEV
// PACKAGE: PROD
//! CSV/TSV addressing for `read` and `replace`: a cell is named by row (or a
//! `where` match on other columns) and column (header name or 1-based index),
//! and resolves to the exact byte span the cell occupies, so an edit splices
//! only that cell and every other byte -- quoting, delimiter, BOM, line
//! endings -- stays as it was. Addressing stops at the cell: a packed value
//! such as `color=Red,size=M` is one opaque cell.

use serde_json::Value;

/// How the file is written, detected and reported back, never normalised.
#[derive(Debug, Clone, PartialEq)]
pub struct Dialect {
    pub delimiter: u8,
    pub bom: bool,
    /// "\n", "\r\n", "mixed", or "none" for a single line with no ending.
    pub line_ending: &'static str,
    pub header: bool,
}

/// One field: its raw byte span (quotes included) and its decoded value.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub start: usize,
    pub end: usize,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub fields: Vec<Field>,
}

/// A resolved cell: 1-based data row and column, the column's header name
/// when there is a header, its raw byte span, and its decoded value.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub row: usize,
    pub column: usize,
    pub name: Option<String>,
    pub start: usize,
    pub end: usize,
    pub value: String,
}

/// A refusal: a stable code and a message naming what to do instead.
pub type Refusal = (&'static str, String);

const BOM: &[u8] = b"\xEF\xBB\xBF";

/// Splits `bytes` into records, honouring quotes, doubled quotes, quoted
/// line breaks, CRLF and a leading BOM. A malformed quote is refused with
/// its byte offset rather than guessed around.
pub fn scan(bytes: &[u8], delimiter: u8) -> Result<Vec<Record>, Refusal> {
    let mut records = Vec::new();
    let mut pos = if bytes.starts_with(BOM) { BOM.len() } else { 0 };
    while pos < bytes.len() {
        let mut fields = Vec::new();
        loop {
            let start = pos;
            let mut value = Vec::new();
            if bytes.get(pos) == Some(&b'"') {
                pos += 1;
                loop {
                    match bytes.get(pos) {
                        None => {
                            return Err((
                                "csv_malformed",
                                format!(
                                    "the quoted field starting at byte {start} is never closed"
                                ),
                            ))
                        }
                        Some(b'"') if bytes.get(pos + 1) == Some(&b'"') => {
                            value.push(b'"');
                            pos += 2;
                        }
                        Some(b'"') => {
                            pos += 1;
                            break;
                        }
                        Some(&byte) => {
                            value.push(byte);
                            pos += 1;
                        }
                    }
                }
                match bytes.get(pos) {
                    None | Some(b'\n') | Some(b'\r') => {}
                    Some(&byte) if byte == delimiter => {}
                    Some(_) => {
                        return Err((
                            "csv_malformed",
                            format!("a character follows the closing quote of the field at byte {start}; quote the whole field or none of it"),
                        ))
                    }
                }
            } else {
                while let Some(&byte) = bytes.get(pos) {
                    if byte == delimiter || byte == b'\n' || byte == b'\r' {
                        break;
                    }
                    value.push(byte);
                    pos += 1;
                }
            }
            fields.push(Field {
                start,
                end: pos,
                value: String::from_utf8_lossy(&value).into_owned(),
            });
            match bytes.get(pos) {
                Some(&byte) if byte == delimiter => pos += 1,
                Some(b'\r') if bytes.get(pos + 1) == Some(&b'\n') => {
                    pos += 2;
                    break;
                }
                Some(b'\n') | Some(b'\r') => {
                    pos += 1;
                    break;
                }
                _ => break,
            }
        }
        records.push(Record { fields });
    }
    Ok(records)
}

/// The delimiter: tab for a `.tsv`/`.tab` file, otherwise whichever of
/// `,` `;` tab `|` occurs most often outside quotes on the first line.
pub fn detect_delimiter(bytes: &[u8], extension: Option<&str>) -> u8 {
    if matches!(
        extension.map(str::to_ascii_lowercase).as_deref(),
        Some("tsv") | Some("tab")
    ) {
        return b'\t';
    }
    let mut counts = [(b',', 0usize), (b';', 0), (b'\t', 0), (b'|', 0)];
    let mut quoted = false;
    for &byte in bytes {
        if byte == b'\n' && !quoted {
            break;
        }
        if byte == b'"' {
            quoted = !quoted;
        } else if !quoted {
            for (candidate, count) in counts.iter_mut() {
                if byte == *candidate {
                    *count += 1;
                }
            }
        }
    }
    counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .filter(|(_, count)| *count > 0)
        .map(|(candidate, _)| *candidate)
        .unwrap_or(b',')
}

fn line_ending(bytes: &[u8]) -> &'static str {
    let crlf = bytes.windows(2).filter(|pair| pair == b"\r\n").count();
    let lf = bytes.iter().filter(|&&byte| byte == b'\n').count();
    match (crlf, lf) {
        (0, 0) => "none",
        (0, _) => "\n",
        (crlf, lf) if crlf == lf => "\r\n",
        _ => "mixed",
    }
}

/// Encodes `value` as one field, quoting it only when it has to be: when it
/// holds the delimiter, a quote, or a line break.
pub fn encode(value: &str, delimiter: u8) -> String {
    let needs_quotes = value
        .bytes()
        .any(|byte| byte == delimiter || byte == b'"' || byte == b'\n' || byte == b'\r');
    if needs_quotes {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

enum ColumnRef {
    Name(String),
    Index(usize),
}

fn column_ref(value: &Value) -> Result<ColumnRef, Refusal> {
    match value {
        Value::String(name) => Ok(ColumnRef::Name(name.clone())),
        Value::Number(number) => number
            .as_u64()
            .filter(|index| *index > 0)
            .map(|index| ColumnRef::Index(index as usize))
            .ok_or((
                "csv_address_invalid",
                "a column index is a positive whole number, 1-based".into(),
            )),
        _ => Err((
            "csv_address_invalid",
            "a column is a header name or a 1-based index".into(),
        )),
    }
}

fn column_index(reference: &ColumnRef, names: Option<&[String]>) -> Result<usize, Refusal> {
    match reference {
        ColumnRef::Index(index) => Ok(*index),
        ColumnRef::Name(name) => {
            let Some(names) = names else {
                return Err((
                    "csv_column_unknown",
                    format!("column \"{name}\" is named, but header is false; name columns by 1-based index"),
                ));
            };
            names
                .iter()
                .position(|candidate| candidate == name)
                .map(|index| index + 1)
                .ok_or_else(|| {
                    (
                        "csv_column_unknown",
                        format!(
                            "no column is named \"{name}\"; the header has: {}",
                            names.join(", ")
                        ),
                    )
                })
        }
    }
}

/// Resolves a `csv` address against the document. The address is
/// `{row | where, column, all, header, delimiter}`:
/// - `row`: 1-based data row (the header, when there is one, is not counted);
/// - `where`: `{column: value, ...}`, every pair must match (AND); more than
///   one matching row is refused unless `all: true`;
/// - neither: every data row, only with `all: true`;
/// - `column`: a header name, a 1-based index, or a list of them; omitted
///   means every column of the row;
/// - `header`: whether the first record is a header (default true);
/// - `delimiter`: one character, overriding detection.
pub fn resolve(
    bytes: &[u8],
    extension: Option<&str>,
    address: &Value,
) -> Result<(Dialect, Vec<Cell>), Refusal> {
    let object = address.as_object().ok_or((
        "csv_address_invalid",
        "csv is an object: {row | where, column, all, header, delimiter}".to_owned(),
    ))?;
    let header = object
        .get("header")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let all = object.get("all").and_then(Value::as_bool).unwrap_or(false);
    let delimiter = match object.get("delimiter") {
        None => detect_delimiter(bytes, extension),
        Some(Value::String(text)) if text.len() == 1 => text.as_bytes()[0],
        Some(_) => {
            return Err((
                "csv_address_invalid",
                "delimiter is a single character such as \",\", \";\" or \"\\t\"".into(),
            ))
        }
    };
    let dialect = Dialect {
        delimiter,
        bom: bytes.starts_with(BOM),
        line_ending: line_ending(bytes),
        header,
    };
    let records = scan(bytes, delimiter)?;
    let names: Option<Vec<String>> = if header {
        Some(
            records
                .first()
                .map(|record| {
                    record
                        .fields
                        .iter()
                        .map(|field| field.value.clone())
                        .collect()
                })
                .unwrap_or_default(),
        )
    } else {
        None
    };
    let data = &records[usize::from(header).min(records.len())..];

    let rows: Vec<usize> = match (object.get("row"), object.get("where")) {
        (Some(_), Some(_)) => {
            return Err((
                "csv_address_invalid",
                "name a row by number or by where, not both".into(),
            ))
        }
        (Some(row), None) => {
            let row = row.as_u64().filter(|row| *row > 0).ok_or((
                "csv_address_invalid",
                "row is a 1-based data row number".to_owned(),
            ))? as usize;
            if row > data.len() {
                return Err((
                    "csv_row_out_of_range",
                    format!(
                        "row {row} does not exist; the file has {} data rows",
                        data.len()
                    ),
                ));
            }
            vec![row]
        }
        (None, Some(conditions)) => {
            let conditions = conditions.as_object().ok_or((
                "csv_address_invalid",
                "where is an object of column: value pairs, all of which must match".to_owned(),
            ))?;
            let mut tests = Vec::new();
            for (column, wanted) in conditions {
                let reference = match column.parse::<usize>() {
                    Ok(index) if !header && index > 0 => ColumnRef::Index(index),
                    _ => ColumnRef::Name(column.clone()),
                };
                let index = column_index(&reference, names.as_deref())?;
                let wanted = match wanted {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                };
                tests.push((index, wanted));
            }
            let matched: Vec<usize> = data
                .iter()
                .enumerate()
                .filter(|(_, record)| {
                    tests.iter().all(|(index, wanted)| {
                        record
                            .fields
                            .get(index - 1)
                            .is_some_and(|field| &field.value == wanted)
                    })
                })
                .map(|(position, _)| position + 1)
                .collect();
            if matched.is_empty() {
                return Err((
                    "csv_where_no_match",
                    "no data row matches every where condition".into(),
                ));
            }
            if matched.len() > 1 && !all {
                return Err((
                    "csv_where_ambiguous",
                    format!(
                        "{} rows match ({}); add conditions to narrow it to one, or pass all: true to address every one",
                        matched.len(),
                        matched
                            .iter()
                            .take(20)
                            .map(usize::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
            matched
        }
        (None, None) if all => (1..=data.len()).collect(),
        (None, None) => {
            return Err((
                "csv_address_invalid",
                "name a row (row or where), or pass all: true for every data row".into(),
            ))
        }
    };

    let columns: Option<Vec<usize>> = match object.get("column") {
        None => None,
        Some(Value::Array(list)) => Some(
            list.iter()
                .map(|item| column_ref(item).and_then(|r| column_index(&r, names.as_deref())))
                .collect::<Result<_, _>>()?,
        ),
        Some(single) => Some(vec![column_index(&column_ref(single)?, names.as_deref())?]),
    };

    let mut cells = Vec::new();
    for row in rows {
        let record = &data[row - 1];
        let wanted: Vec<usize> = columns
            .clone()
            .unwrap_or_else(|| (1..=record.fields.len()).collect());
        for column in wanted {
            let field = record.fields.get(column - 1).ok_or_else(|| {
                (
                    "csv_column_missing",
                    format!(
                        "row {row} has {} fields, so it has no column {column}",
                        record.fields.len()
                    ),
                )
            })?;
            cells.push(Cell {
                row,
                column,
                name: names
                    .as_ref()
                    .and_then(|names| names.get(column - 1).cloned()),
                start: field.start,
                end: field.end,
                value: field.value.clone(),
            });
        }
    }
    Ok((dialect, cells))
}

/// Writes `values` into `cells` (sorted by position), returning the byte span
/// the edit covers and its replacement. Every byte between the cells is kept
/// as it was, so only the written cells change. The result is checked to
/// scan again with the same number of records and fields; an edit that would
/// break the structure is refused.
pub fn splice(
    bytes: &[u8],
    delimiter: u8,
    cells: &[Cell],
    values: &[String],
) -> Result<(usize, usize, Vec<u8>), Refusal> {
    let mut order: Vec<usize> = (0..cells.len()).collect();
    order.sort_by_key(|&index| cells[index].start);
    let first = cells[order[0]].start;
    let last = cells[order[order.len() - 1]].end;
    let mut middle = Vec::new();
    let mut cursor = first;
    for &index in &order {
        let cell = &cells[index];
        middle.extend_from_slice(&bytes[cursor..cell.start]);
        middle.extend_from_slice(encode(&values[index], delimiter).as_bytes());
        cursor = cell.end;
    }
    middle.extend_from_slice(&bytes[cursor..last]);
    let mut after = bytes[..first].to_vec();
    after.extend_from_slice(&middle);
    after.extend_from_slice(&bytes[last..]);
    let shape = |records: &[Record]| -> Vec<usize> {
        records.iter().map(|record| record.fields.len()).collect()
    };
    if shape(&scan(&after, delimiter)?) != shape(&scan(bytes, delimiter)?) {
        return Err((
            "csv_structure_changed",
            "the edit would change the number of rows or fields; nothing was written".into(),
        ));
    }
    Ok((first, last, middle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const CATALOG: &str =
        "sku,store_view_code,name,qty\nA1,,Apple,10\nA1,nl,Appel,10\nB2,,\"Pear, green\",5\n";

    #[test]
    fn a_quoted_field_with_the_delimiter_and_a_doubled_quote_decodes() {
        let records = scan(b"a,\"b, \"\"c\"\"\"\n", b',').unwrap();
        assert_eq!(records[0].fields[1].value, "b, \"c\"");
        assert_eq!(records[0].fields[1].start, 2);
    }

    #[test]
    fn a_where_map_matches_on_every_condition() {
        let (_, cells) = resolve(
            CATALOG.as_bytes(),
            Some("csv"),
            &json!({"where": {"sku": "A1", "store_view_code": "nl"}, "column": "name"}),
        )
        .unwrap();
        assert_eq!(cells.len(), 1);
        assert_eq!((cells[0].row, cells[0].value.as_str()), (2, "Appel"));
    }

    #[test]
    fn a_where_matching_several_rows_is_refused_without_all() {
        let refused = resolve(
            CATALOG.as_bytes(),
            Some("csv"),
            &json!({"where": {"sku": "A1"}, "column": "qty"}),
        )
        .unwrap_err();
        assert_eq!(refused.0, "csv_where_ambiguous");
    }

    #[test]
    fn splicing_rewrites_only_the_addressed_cells() {
        let bytes = CATALOG.as_bytes();
        let (_, cells) = resolve(bytes, Some("csv"), &json!({"row": 3, "column": [3, 4]})).unwrap();
        let (start, end, middle) =
            splice(bytes, b',', &cells, &["Pear; ripe".into(), "".into()]).unwrap();
        let mut after = bytes[..start].to_vec();
        after.extend_from_slice(&middle);
        after.extend_from_slice(&bytes[end..]);
        assert_eq!(
            String::from_utf8(after).unwrap(),
            "sku,store_view_code,name,qty\nA1,,Apple,10\nA1,nl,Appel,10\nB2,,Pear; ripe,\n"
        );
    }

    #[test]
    fn a_bom_and_a_semicolon_dialect_are_detected_and_the_header_still_matches() {
        let bytes = b"\xEF\xBB\xBFsku;name\r\nA1;Apple\r\n";
        let (dialect, cells) =
            resolve(bytes, Some("csv"), &json!({"row": 1, "column": "sku"})).unwrap();
        assert_eq!(
            (dialect.delimiter, dialect.bom, dialect.line_ending),
            (b';', true, "\r\n")
        );
        assert_eq!(cells[0].value, "A1");
    }

    #[test]
    fn a_value_with_a_line_break_is_quoted_and_the_structure_survives() {
        let bytes = b"a,b\n1,2\n";
        let (_, cells) = resolve(bytes, None, &json!({"row": 1, "column": "b"})).unwrap();
        let (_, _, middle) = splice(bytes, b',', &cells, &["two\nlines".into()]).unwrap();
        assert_eq!(middle, b"\"two\nlines\"");
    }
}
