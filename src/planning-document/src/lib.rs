// MODE: DEV
// PACKAGE: PROD
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    Plan,
    Review,
    Reference,
    Goal,
    Step,
    Testing,
}

pub fn document_kind(id: &str) -> Result<DocumentKind, String> {
    match id {
        "plan" => Ok(DocumentKind::Plan),
        "adversarial-review" => Ok(DocumentKind::Review),
        "coverage" | "inventory" | "stories" | "bugs" | "planning-bugs" | "fixes" | "fix-keys"
        | "fixkeys" | "approval" | "progress" => Ok(DocumentKind::Reference),
        value if value.starts_with("goal-progress:") => Ok(DocumentKind::Reference),
        value if value.starts_with("goal:") => Ok(DocumentKind::Goal),
        value if value.starts_with("step:") => {
            if value.ends_with("-testing") {
                Ok(DocumentKind::Testing)
            } else {
                Ok(DocumentKind::Step)
            }
        }
        value if value.starts_with("unit:") => Ok(DocumentKind::Step),
        value => Err(format!("Unknown document ID: {value}")),
    }
}

pub fn section_spec(kind: DocumentKind, section: &str) -> Option<(&'static str, u8)> {
    let value = match (kind, section) {
        (DocumentKind::Plan, "current-state") => ("## Current state", 2),
        (DocumentKind::Plan, "desired-outcome") => ("## Desired outcome", 3),
        (DocumentKind::Plan, "approach") => ("## Approach", 4),
        (DocumentKind::Plan, "scope") => ("## Scope", 5),
        (DocumentKind::Plan, "affected-areas") => ("## Affected areas", 6),
        (DocumentKind::Plan, "constraints-and-decisions") => ("## Constraints and decisions", 7),
        (DocumentKind::Plan, "risks-and-open-questions") => ("## Risks and open questions", 8),
        (DocumentKind::Plan, "environment-facts") => ("## Environment facts", 9),
        (DocumentKind::Plan, "approach-decisions") => ("## Approach decisions", 10),
        (DocumentKind::Goal, "current-state-and-prior-goal-handoffs") => {
            ("## Current state and prior-goal handoffs", 2)
        }
        (DocumentKind::Goal, "outcome-and-definition-of-done") => {
            ("## Outcome and definition of done", 3)
        }
        (DocumentKind::Goal, "why-this-goal-is-needed") => ("## Why this goal is needed", 4),
        (DocumentKind::Goal, "scope") => ("## Scope", 5),
        (DocumentKind::Goal, "affected-areas") => {
            ("## Affected files, systems, data, and interfaces", 6)
        }
        (DocumentKind::Goal, "dependencies-and-handoffs") => ("## Dependencies and handoffs", 7),
        (DocumentKind::Goal, "implementation-approach-risks-and-edge-cases") => {
            ("## Implementation approach, risks, and edge cases", 8)
        }
        (DocumentKind::Goal, "owned-work-units") => ("## Owned work units", 9),
        (DocumentKind::Goal, "goal-size-exception") => ("## Goal-size exception", 11),
        (DocumentKind::Step, "objective") => ("## Objective", 4),
        (DocumentKind::Step, "instructions") => ("## Instructions", 5),
        (DocumentKind::Step, "acceptance-criteria") => ("## Acceptance criteria", 6),
        (DocumentKind::Step, "handoff") => ("## Handoff", 7),
        (DocumentKind::Testing, "automated-tests") => ("## Automated tests", 2),
        (DocumentKind::Testing, "browser-verification") => ("## Browser verification", 3),
        (DocumentKind::Testing, "backend-verification") => ("## Backend verification", 4),
        (DocumentKind::Testing, "manual-verification") => ("## Manual verification", 5),
        (DocumentKind::Review, "review-scope") => ("## Review scope", 1),
        (DocumentKind::Review, "findings") => ("## Findings", 2),
        (DocumentKind::Review, "rationale") => ("## Verdict", 3),
        _ => return None,
    };
    Some(value)
}

pub fn section_headings(document: &str) -> Vec<&str> {
    document
        .lines()
        .filter(|line| line.starts_with("## "))
        .collect()
}

pub fn replace_field(document: &str, label: &str, value: &str) -> Result<String, String> {
    let prefix = format!("- {label}:");
    let mut found = 0;
    let mut output = String::with_capacity(document.len());
    for line in document.split_inclusive('\n') {
        let (body, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        if body.starts_with(&prefix) {
            found += 1;
            output.push_str(&format!("- {label}: {value}"));
        } else {
            output.push_str(body);
        }
        output.push_str(newline);
    }
    if found != 1 {
        return Err(format!("Field was not found exactly once: {label}"));
    }
    Ok(output)
}

pub fn replace_title(document: &str, title: &str) -> Result<String, String> {
    let mut found = 0;
    let mut output = String::with_capacity(document.len());
    for line in document.split_inclusive('\n') {
        let (body, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        if body.starts_with("# ") {
            found += 1;
            let Some((prefix, _)) = body.split_once(':') else {
                return Err("Document title heading has no ': title' part to replace".into());
            };
            output.push_str(&format!("{prefix}: {title}"));
        } else {
            output.push_str(body);
        }
        output.push_str(newline);
    }
    if found != 1 {
        return Err("Document title was not found exactly once".into());
    }
    Ok(output)
}

pub fn replace_paragraph(
    document: &str,
    paragraph: &str,
    replacement: &str,
) -> Result<String, String> {
    let replacement = replacement.trim_end_matches('\n');
    let lines: Vec<&str> = document.lines().collect();
    let Some(start) = lines.iter().position(|line| *line == paragraph) else {
        return Err(format!("Paragraph was not found exactly once: {paragraph}"));
    };
    if lines
        .iter()
        .skip(start + 1)
        .filter(|line| **line == paragraph)
        .count()
        > 0
    {
        return Err(format!("Paragraph was not found exactly once: {paragraph}"));
    }
    let mut end = start + 1;
    while end < lines.len()
        && !lines[end].is_empty()
        && !lines[end].starts_with('§')
        && !lines[end].starts_with("## ")
    {
        end += 1;
    }
    let mut output: Vec<String> = lines[..=start].iter().map(|line| (*line).into()).collect();
    output.push(replacement.into());
    if end < lines.len()
        && (!lines[end].is_empty())
        && (lines[end].starts_with('§') || lines[end].starts_with("## "))
    {
        output.push(String::new());
    }
    output.extend(lines[end..].iter().map(|line| (*line).into()));
    let mut rendered = output.join("\n");
    if document.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

pub fn replace_section(document: &str, heading: &str, body: &str) -> Result<String, String> {
    let lines: Vec<&str> = document.lines().collect();
    let Some(start) = lines.iter().position(|line| *line == heading) else {
        return Err(missing_section_message("document", heading, document));
    };
    if lines.iter().skip(start + 1).any(|line| *line == heading) {
        return Err(format!("{heading} was not found exactly once"));
    }
    let end = lines[start + 1..]
        .iter()
        .position(|line| line.starts_with("## "))
        .map(|offset| start + 1 + offset)
        .unwrap_or(lines.len());
    if let Some(first) = lines[start + 1..end]
        .iter()
        .map(|line| line.trim())
        .find(|line| !line.is_empty())
    {
        if first.starts_with("- ") && first.contains(':') {
            return Err(format!(
                "Section '{heading}' holds fields (- Label: value); rewriting it would remove them, and a field there may belong to another gate. Write one field at a time with --field."
            ));
        }
        if first.starts_with('|') {
            return Err(format!(
                "Section '{heading}' is a table; rewriting it as paragraphs would discard every row. Use the helper that owns that table."
            ));
        }
    }
    let mut output: Vec<String> = lines[..=start].iter().map(|line| (*line).into()).collect();
    output.push(String::new());
    output.extend(body.lines().map(str::to_string));
    if end < lines.len()
        && !lines[end].is_empty()
        && output.last().is_some_and(|line| !line.is_empty())
    {
        output.push(String::new());
    }
    output.extend(lines[end..].iter().map(|line| (*line).into()));
    let mut rendered = output.join("\n");
    if document.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

pub fn insert_paragraph(
    document: &str,
    paragraph: &str,
    after: bool,
    replacement: &str,
) -> Result<String, String> {
    let replacement = replacement.trim_end_matches('\n');
    let lines: Vec<&str> = document.lines().collect();
    let target = paragraph
        .strip_prefix("§ ")
        .and_then(|value| value.split_once('.'))
        .ok_or_else(|| format!("Paragraph was not found exactly once: {paragraph}"))?;
    let target_section = target.0;
    let target_number: usize = target
        .1
        .parse()
        .map_err(|_| format!("Paragraph was not found exactly once: {paragraph}"))?;
    let insertion_number = if after {
        target_number + 1
    } else {
        target_number
    };
    let mut output = Vec::new();
    let mut pending_after = false;
    let mut found = 0;
    for line in lines {
        let parsed = line
            .strip_prefix("§ ")
            .and_then(|value| value.split_once('.'));
        if pending_after && (parsed.is_some() || line.starts_with("## ")) {
            emit_inserted_paragraph(&mut output, target_section, insertion_number, replacement);
            pending_after = false;
        }
        let mut rendered = line.to_string();
        if let Some((section, number)) = parsed {
            if section == target_section {
                let number: usize = number.parse().unwrap_or(0);
                if number == target_number {
                    found += 1;
                    if found > 1 {
                        return Err(format!("Paragraph was not found exactly once: {paragraph}"));
                    }
                    if !after {
                        emit_inserted_paragraph(
                            &mut output,
                            target_section,
                            insertion_number,
                            replacement,
                        );
                    } else {
                        pending_after = true;
                    }
                }
                if number >= insertion_number {
                    rendered = format!("§ {section}.{}", number + 1);
                }
            }
        }
        output.push(rendered);
    }
    if pending_after {
        emit_inserted_paragraph(&mut output, target_section, insertion_number, replacement);
    }
    if found != 1 {
        return Err(format!("Paragraph was not found exactly once: {paragraph}"));
    }
    let mut rendered = output.join("\n");
    if document.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

fn emit_inserted_paragraph(
    output: &mut Vec<String>,
    section: &str,
    number: usize,
    replacement: &str,
) {
    if output.last().is_some_and(|line| !line.is_empty()) {
        output.push(String::new());
    }
    output.push(format!("§ {section}.{number}"));
    output.extend(replacement.lines().map(str::to_string));
    output.push(String::new());
}

pub fn delete_paragraph(document: &str, paragraph: &str) -> Result<String, String> {
    let lines: Vec<&str> = document.lines().collect();
    let Some(start) = lines.iter().position(|line| *line == paragraph) else {
        return Err(format!("Paragraph was not found exactly once: {paragraph}"));
    };
    let wanted = paragraph
        .strip_prefix("§ ")
        .ok_or_else(|| "Paragraph ID must use the form '§ 2.1'".to_string())?;
    let (section, number) = wanted
        .split_once('.')
        .ok_or_else(|| "Paragraph ID must use the form '§ 2.1'".to_string())?;
    let number: usize = number
        .parse()
        .map_err(|_| "Paragraph ID must use the form '§ 2.1'".to_string())?;
    let mut end = start + 1;
    while end < lines.len() && !lines[end].starts_with('§') && !lines[end].starts_with("## ") {
        end += 1;
    }
    let mut output = Vec::new();
    output.extend(lines[..start].iter().map(|line| (*line).to_string()));
    for line in &lines[end..] {
        if let Some(rest) = line.strip_prefix("§ ") {
            if let Some((candidate_section, candidate_number)) = rest.split_once('.') {
                if candidate_section == section {
                    if let Ok(candidate_number) = candidate_number.parse::<usize>() {
                        if candidate_number > number {
                            output.push(format!("§ {section}.{}", candidate_number - 1));
                            continue;
                        }
                    }
                }
            }
        }
        output.push((*line).to_string());
    }
    let mut rendered = output.join("\n");
    if document.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

pub fn missing_section_message(file_name: &str, heading: &str, document: &str) -> String {
    let present = section_headings(document).join(" ");
    let mut message = format!("{} not found in {}", heading, file_name);
    if !present.is_empty() {
        message.push_str(&format!("; it has: {}", present));
    }
    message
        .push_str(" -- a section form rewrites a section that already exists, it cannot add one. ");
    if file_name.ends_with("-testing.md") {
        message.push_str("A testing companion carries only the sections its creator emitted; create-step-testing.sh emits \"## Automated tests\".");
    } else {
        message.push_str(
            "Create the document with the helper that owns it, then rewrite the section.",
        );
    }
    message
}

#[cfg(test)]
mod tests {
    use super::{
        delete_paragraph, document_kind, insert_paragraph, missing_section_message,
        replace_section, section_spec, DocumentKind,
    };

    #[test]
    fn document_kinds_preserve_testing_companions() {
        assert_eq!(document_kind("goal:01-a").unwrap(), DocumentKind::Goal);
        assert_eq!(
            document_kind("step:01-a/02-step-testing").unwrap(),
            DocumentKind::Testing
        );
        assert_eq!(document_kind("unit:W01").unwrap(), DocumentKind::Step);
    }

    #[test]
    fn section_specs_keep_heading_and_paragraph_number() {
        assert_eq!(
            section_spec(DocumentKind::Plan, "current-state"),
            Some(("## Current state", 2))
        );
        assert_eq!(
            section_spec(DocumentKind::Review, "rationale"),
            Some(("## Verdict", 3))
        );
        assert_eq!(section_spec(DocumentKind::Review, "scope"), None);
    }

    #[test]
    fn missing_section_names_present_headings_and_remedy() {
        let message = missing_section_message(
            "step-testing.md",
            "## Browser verification",
            "## Automated tests\n## Handoff\n",
        );
        assert!(message.contains("it has: ## Automated tests ## Handoff"));
        assert!(message.contains("create-step-testing.sh emits"));
    }

    #[test]
    fn paragraph_insert_and_delete_renumber_with_section_boundaries() {
        let document =
            "## Current state\n\n§ 2.1\nfirst\n\n§ 2.2\nsecond\n\n## Scope\n\n§ 5.1\nscope\n";
        let inserted = insert_paragraph(document, "§ 2.1", true, "middle").unwrap();
        assert!(inserted.contains("§ 2.2\nmiddle\n\n§ 2.3\nsecond"));
        let deleted = delete_paragraph(&inserted, "§ 2.2").unwrap();
        assert!(deleted.contains("§ 2.2\nsecond"));
        assert!(!deleted.contains("§ 2.3"));
        assert!(deleted.contains("## Scope\n\n§ 5.1\nscope"));
    }

    #[test]
    fn section_rewrite_refuses_structured_content() {
        let fields = "## Fields\n\n- Status: pending\n\n## Next\n";
        let table = "## Findings\n\n| ID | Status |\n|---|---|\n| AR-01 | open |\n\n## Next\n";
        assert!(replace_section(fields, "## Fields", "§ 1.1\ntext")
            .unwrap_err()
            .contains("holds fields"));
        assert!(replace_section(table, "## Findings", "§ 1.1\ntext")
            .unwrap_err()
            .contains("is a table"));
    }
}
