// MODE: DEV
// PACKAGE: PROD
use planning_core::{atomic_write, git_snapshot, require_safe_value};
use planning_document::{
    delete_paragraph, document_kind, insert_paragraph, replace_field, replace_paragraph,
    replace_section, replace_title,
};
use planning_inventory::find;
use planning_table::{csv_to_markdown, replace_testing_requirement, CsvError};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const COMMAND: &str = "update-plan-content.sh";

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code);
}

fn usage(code: i32) -> ! {
    print!(
        "Usage:\n  {COMMAND} -dp|--description-paragraph [--plan-dir] <plan-directory> <N.N> <text>\n                                             ONE paragraph; replaces it. Extra -p flags are an error.\n  {COMMAND} -ds|--description-section [--plan-dir] <plan-directory> <section-id> -p N.1: <content> [-p N.2: <content> ...]\n                                             WHOLE section; paragraphs must be sequential from N.1.\n  {COMMAND} -gp|--goal-paragraph [--plan-dir] <plan-directory> <goal-name> <N.N> <text>\n                                             ONE paragraph; replaces it. Extra -p flags are an error.\n  {COMMAND} -gs|--goal-section [--plan-dir] <plan-directory> <goal-name> <section-id> -p N.1: <content> [-p N.2: <content> ...]\n                                             WHOLE section; paragraphs must be sequential from N.1.\n  {COMMAND} -sp|--step-paragraph [--plan-dir] <plan-directory> <goal>/<step> <N.N> <text>\n                                             ONE paragraph; replaces it. Extra -p flags are an error.\n  {COMMAND} -ss|--step-section [--plan-dir] <plan-directory> <goal>/<step> <section-id> -p N.1: <content> [-p N.2: <content> ...]\n                                             WHOLE section; paragraphs must be sequential from N.1.\n  {COMMAND} -rp|--review-paragraph [--plan-dir] <plan-directory> <N.N> <text>\n                                             ONE paragraph; replaces it. Extra -p flags are an error.\n  {COMMAND} -rs|--review-section [--plan-dir] <plan-directory> <section-id> -p N.1: <content> [-p N.2: <content> ...]\n                                             WHOLE section; paragraphs must be sequential from N.1.\n  {COMMAND} -ap|--append-paragraph [--plan-dir] <plan-directory> <document-id> <section-id> <text>\n                                             Appends one paragraph with the next free number in the section.\n  {COMMAND} -tp|--table-paragraph [--plan-dir] <plan-directory> <document-id> <N.N> <columns> <CSV>\n  {COMMAND} -ia|--insert-after [--plan-dir] <plan-directory> <document-id> <N.N> <text>\n  {COMMAND} -ib|--insert-before [--plan-dir] <plan-directory> <document-id> <N.N> <text>\n  {COMMAND} --delete-paragraph [--plan-dir] <plan-directory> <document-id> <N.N>\n                                              Deletes ONE paragraph and renumbers the\n                                              following paragraphs in the same section.\n  {COMMAND} -t|--title [--plan-dir] <plan-directory> <document-id> <title>\n  {COMMAND} -f|--field [--plan-dir] <plan-directory> <document-id> <field-label> <value>\n  {COMMAND} -tr|--testing-requirement [--plan-dir] <plan-directory> <goal-name> <yes|no> <rationale>\n  {COMMAND} -rv|--review-status [--plan-dir] <plan-directory> <pending|approved>\n  {COMMAND} -dr|--decomposition-review [--plan-dir] <plan-directory> <incomplete|completed>\n\nDocument IDs: plan, review, goal:<goal>, step:<goal>/<step>, or unit:<WNN>.\n"
    );
    std::process::exit(code);
}

fn safe(label: &str, value: &str) {
    if let Err(error) = require_safe_value(label, value) {
        die(error, 64);
    }
}

fn valid_goal_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 4
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b'-'
        && value[3..]
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn normalize_paragraph_id(value: &str, flag: &str) -> Result<String, String> {
    match value {
        "plan" | "review" | "stories" | "coverage" | "inventory" => {
            let form = match flag {
                "-dp" => "-dps (section form)",
                "-gp" => "-dps",
                "-sp" => "-dps",
                "-rp" => "a review form",
                _ => "the correct document-specific form",
            };
            return Err(format!(
                "'{value}' is a document id, not a paragraph number; use {form} with --document {value} instead"
            ));
        }
        value
            if value.starts_with("goal:")
                || value.starts_with("step:")
                || value.starts_with("unit:") =>
        {
            let wanted = match flag {
                "-dp" => "-gp or -sp",
                "-gp" => "-dp",
                "-sp" => "-gp",
                "-rp" => "a review form",
                _ => "the correct document-specific form",
            };
            return Err(format!(
                "'{value}' is a document id, not a paragraph number; you probably want {wanted} instead of {flag}"
            ));
        }
        _ => {}
    }
    let candidate = value.strip_suffix(':').unwrap_or(value);
    let valid = candidate
        .split_once('.')
        .is_some_and(|(section, paragraph)| {
            !section.is_empty()
                && !paragraph.is_empty()
                && section.bytes().all(|byte| byte.is_ascii_digit())
                && paragraph.bytes().all(|byte| byte.is_ascii_digit())
        });
    if !valid {
        return Err("Paragraph must use N.N or N.N:".to_string());
    }
    Ok(format!("{candidate}:"))
}

fn reject_swallowed_flags(content: &str, flag: &str) -> Result<(), String> {
    let tokens: Vec<&str> = content.split_whitespace().collect();
    let flag_shaped = tokens.windows(3).any(|window| {
        matches!(
            window[0],
            "-p" | "-dp" | "-gp" | "-sp" | "-rp" | "-tp" | "-ia" | "-ib"
        ) && window[1].split_once('.').is_some_and(|(a, b)| {
            !a.is_empty()
                && !b.is_empty()
                && a.bytes().all(|c| c.is_ascii_digit())
                && b.trim_end_matches(':').bytes().all(|c| c.is_ascii_digit())
                && b.ends_with(':')
        }) && window[2].is_empty()
    }) || content
        .split_whitespace()
        .enumerate()
        .any(|(index, token)| {
            matches!(
                token,
                "-p" | "-dp" | "-gp" | "-sp" | "-rp" | "-tp" | "-ia" | "-ib"
            ) && content
                .split_whitespace()
                .nth(index + 1)
                .is_some_and(|next| {
                    next.split_once('.').is_some_and(|(a, b)| {
                        !a.is_empty()
                            && !b.is_empty()
                            && a.bytes().all(|c| c.is_ascii_digit())
                            && b.ends_with(':')
                    })
                })
        });
    if !flag_shaped {
        return Ok(());
    }
    let section_form = match flag {
        "-ia" | "-ib" => flag.to_string(),
        _ => format!("{}s", flag.trim_end_matches('p')),
    };
    Err(format!(
        "Content for {flag} absorbed flag-shaped text ('-p N.N:' style); {flag} takes exactly one paragraph and no flags. Use the section form instead:\n  {COMMAND} {section_form} [--plan-dir] <plan-directory> ... -p N.1: '...' -p N.2: '...'\nSection form requires sequential paragraphs starting at N.1."
    ))
}

fn paragraph_content_error(label: &str, content: &str) -> Option<String> {
    if content.contains(['\n', '\r']) {
        return Some(if label.is_empty() {
            "Paragraph content must be one line".to_string()
        } else {
            format!("Paragraph {label} must be one line")
        });
    }
    if content.contains('§') {
        return Some(
            "Paragraph content must not contain the reserved paragraph marker §".to_string(),
        );
    }
    if content.trim().is_empty() {
        return Some(if label.is_empty() {
            "Paragraph content must not be empty".to_string()
        } else {
            format!("Paragraph {label} has empty content")
        });
    }
    None
}

fn document_path(plan: &Path, id: &str) -> Result<PathBuf, String> {
    let path = match id {
        "plan" => plan.join("plan-description.md"),
        "adversarial-review" => plan.join("adversarial-review.md"),
        "coverage" | "inventory" => plan.join("work-unit-inventory.md"),
        "progress" => plan.join("progress.md"),
        "stories" => plan.join("ui-user-stories.md"),
        "bugs" => plan.join("bugs.md"),
        "planning-bugs" => plan.join("planning-bugs.json"),
        "fixes" => plan.join("fixes.md"),
        "fix-keys" | "fixkeys" => plan.join("fix-keys.json"),
        "approval" => plan.join("approval.json"),
        value if value.starts_with("goal-progress:") => {
            plan.join(format!("{}/progress.md", &value[14..]))
        }
        value if value.starts_with("goal:") => plan.join(format!("{}/goal.md", &value[5..])),
        value if value.starts_with("step:") => {
            let rest = &value[5..];
            let (goal, step) = rest
                .split_once('/')
                .ok_or_else(|| "Step document IDs use step:<goal>/<step>".to_string())?;
            plan.join(goal).join("steps").join(format!("{step}.md"))
        }
        value if value.starts_with("unit:") => {
            let inventory = fs::read_to_string(plan.join("work-unit-inventory.md"))
                .map_err(|error| error.to_string())?;
            let row = find(&inventory, &value[5..])
                .ok_or_else(|| format!("Work unit not found: {}", &value[5..]))?;
            plan.join(row.goal)
                .join("steps")
                .join(format!("{}.md", row.step))
        }
        _ => return Err(format!("Unknown document ID: {id}")),
    };
    Ok(path)
}

fn section_spec(id: &str, section: &str) -> Result<(&'static str, u8), String> {
    let kind = document_kind(id)?;
    planning_document::section_spec(kind, section)
        .ok_or_else(|| {
            let kind_name = match kind {
                planning_document::DocumentKind::Plan => "plan",
                planning_document::DocumentKind::Goal => "goal",
                planning_document::DocumentKind::Step => "step",
                planning_document::DocumentKind::Testing => "testing",
                planning_document::DocumentKind::Review => "review",
                planning_document::DocumentKind::Reference => "reference",
            };
            let valid = match kind {
                planning_document::DocumentKind::Plan => "current-state desired-outcome approach approach-decisions scope affected-areas constraints-and-decisions risks-and-open-questions environment-facts",
                planning_document::DocumentKind::Goal => "current-state-and-prior-goal-handoffs outcome-and-definition-of-done why-this-goal-is-needed scope affected-areas dependencies-and-handoffs implementation-approach-risks-and-edge-cases owned-work-units goal-size-exception",
                planning_document::DocumentKind::Step => "objective instructions acceptance-criteria handoff atomicity-check",
                planning_document::DocumentKind::Testing => "automated-tests browser-verification backend-verification manual-verification",
                planning_document::DocumentKind::Review | planning_document::DocumentKind::Reference => "",
            };
            let close = valid
                .split_whitespace()
                .find(|candidate| candidate.starts_with(section) || section.starts_with(candidate));
            let mut message = format!(
                "Section '{section}' is not a mutable narrative section for a {kind_name} document.\n"
            );
            if let Some(close) = close {
                message.push_str(&format!("Closest match: {close}\n"));
            }
            message.push_str(&format!("Valid {kind_name} section ids: {valid}"));
            message
        })
}

fn plan_dir(args: &[String], index: &mut usize) -> PathBuf {
    if args.get(*index).is_some_and(|value| value == "--plan-dir") {
        *index += 1;
        let value = args.get(*index).unwrap_or_else(|| usage(64));
        *index += 1;
        return PathBuf::from(value);
    }
    if let Some(value) = args
        .get(*index)
        .and_then(|value| value.strip_prefix("--plan-dir="))
    {
        *index += 1;
        return PathBuf::from(value);
    }
    let value = args.get(*index).unwrap_or_else(|| usage(64));
    *index += 1;
    PathBuf::from(value)
}

fn write_document(plan: &Path, id: &str, rendered: String, mode: &str) {
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 66);
    }
    git_snapshot(plan);
    let file = document_path(plan, id).unwrap_or_else(|error| die(error, 64));
    if !file.is_file() {
        die(format!("Document not found: {}", file.display()), 66);
    }
    atomic_write(&file, rendered.as_bytes()).unwrap_or_else(|error| die(error, 73));
    invalidate_context(plan, id);
    emit_step_testing_reminder(plan, id);
    println!("Updated {mode}");
}

fn invalidate_context(plan: &Path, id: &str) {
    let context = plan.join("context");
    if context.is_dir() {
        atomic_write(
            &context.join("mutation-handoff"),
            format!("{id}\n").as_bytes(),
        )
        .unwrap_or_else(|error| die(error, 73));
    }
}

fn git_file_timestamp(plan: &Path, path: &Path) -> Option<i64> {
    let relative = path.strip_prefix(plan).ok()?.to_str()?;
    let directory = plan.to_str()?;
    let output = Command::new("git")
        .args(["-C", directory, "log", "-1", "--format=%ct", "--", relative])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()?.trim().parse().ok()
}

fn emit_step_testing_reminder(plan: &Path, id: &str) {
    if !(id.starts_with("step:") || id.starts_with("unit:")) {
        return;
    }
    let Ok(step) = document_path(plan, id) else {
        return;
    };
    let Some(goal_dir) = step.parent().and_then(Path::parent) else {
        return;
    };
    let goal = goal_dir.join("goal.md");
    let Ok(Some(required)) = planning_table::testing_requirement(&goal) else {
        return;
    };
    if required != "yes" {
        return;
    }
    let stem = step
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let companion = step.with_file_name(format!("{stem}-testing.md"));
    if !companion.is_file() {
        eprintln!(
            "Reminder: this goal requires testing; continue with its test/proof step before marking the goal complete."
        );
        return;
    }
    let (Some(step_at), Some(companion_at)) = (
        git_file_timestamp(plan, &step),
        git_file_timestamp(plan, &companion),
    ) else {
        return;
    };
    if companion_at < step_at {
        eprintln!(
            "Reminder: {} was already behind {} before this edit; review it for accuracy and completeness.",
            companion.file_name().and_then(|name| name.to_str()).unwrap_or_default(),
            step.file_name().and_then(|name| name.to_str()).unwrap_or_default()
        );
    }
}

fn session_id(json: &str) -> Option<String> {
    let marker = "\"session_id\"";
    let start = json.find(marker)? + marker.len();
    let after_colon = &json[start..];
    let colon = after_colon.find(':')?;
    let after_colon = &after_colon[colon + 1..];
    let opening_quote = after_colon.find('"')?;
    let value = &after_colon[opening_quote + 1..];
    let closing_quote = value.find('"')?;
    let value = &value[..closing_quote];
    (!value.is_empty()).then(|| value.to_string())
}

fn auto_create_paragraph(
    document: &str,
    section: u8,
    paragraph: usize,
    content: &str,
) -> Option<Result<(String, String), String>> {
    let labels: Vec<usize> = document
        .lines()
        .filter_map(|line| {
            let label = line.strip_prefix("§ ")?;
            let (section_id, number) = label.split_once('.')?;
            (section_id.parse::<u8>().ok()? == section)
                .then(|| number.parse::<usize>().ok())
                .flatten()
        })
        .collect();
    let max = labels.iter().copied().max().unwrap_or(0);
    if max == 0
        || labels.len() != max
        || labels
            .iter()
            .copied()
            .enumerate()
            .any(|(index, number)| number != index + 1)
        || paragraph != max + 1
    {
        return None;
    }
    let last_label = format!("§ {section}.{max}");
    let lines: Vec<&str> = document.lines().collect();
    let start = lines.iter().position(|line| *line == last_label)?;
    let mut trailing = false;
    let mut paragraph_ended = false;
    for line in &lines[start + 1..] {
        if line.starts_with("## ") || line.starts_with('§') {
            break;
        }
        if line.trim().is_empty() {
            paragraph_ended = true;
        } else if paragraph_ended {
            trailing = true;
            break;
        }
    }
    if trailing {
        return None;
    }
    Some(
        insert_paragraph(document, &last_label, true, content)
            .map(|rendered| {
                let mut section_readout = String::new();
                let mut show = false;
                let new_label = format!("§ {section}.{paragraph}");
                for line in rendered.lines() {
                    if line == last_label || line == new_label {
                        show = true;
                    }
                    if show && !line.trim().is_empty() {
                        section_readout.push_str("  ");
                        section_readout.push_str(line);
                        section_readout.push('\n');
                        if !line.starts_with("§ ") {
                            show = false;
                        }
                    }
                    if show && line.starts_with("## ") {
                        break;
                    }
                }
                (
                    rendered,
                    format!(
                        "update-plan-content: added paragraph § {section}.{paragraph} (auto-created; verify no duplication)\nupdate-plan-content: section now reads:\n{section_readout}"
                    ),
                )
            }),
    )
}

fn missing_paragraph_message(document: &str, section: u8, paragraph: &str) -> String {
    let mut existing = String::new();
    for line in document.lines() {
        let Some(label) = line.strip_prefix("§ ") else {
            continue;
        };
        let Some((section_id, _)) = label.split_once('.') else {
            continue;
        };
        if section_id == section.to_string() {
            existing.push_str(label);
            existing.push(' ');
        }
    }
    if existing.is_empty() {
        existing.push_str("none");
    }
    format!(
        "Paragraph § {paragraph} not found; section {section} currently has: {existing}. Use -dp with an existing paragraph to replace it, or -ds <section> -p N.N: to add sequential paragraphs."
    )
}

fn update_review_status(plan: &Path, requested: &str) {
    if requested != "pending" && requested != "approved" {
        die("Review status must be pending or approved", 64);
    }
    let review = plan.join("adversarial-review.md");
    let description = plan.join("plan-description.md");
    if !review.is_file() || !description.is_file() {
        die(
            "Both plan-description.md and adversarial-review.md are required",
            66,
        );
    }
    let review_text =
        fs::read_to_string(&review).unwrap_or_else(|error| die(error.to_string(), 66));
    let description_text =
        fs::read_to_string(&description).unwrap_or_else(|error| die(error.to_string(), 66));
    let mut invalidate_session = None;
    let (review_value, description_value) = if requested == "pending" {
        ("`💤 pending`", "💤 pending")
    } else {
        let open = review_text.lines().any(|line| {
            line.starts_with('|')
                && planning_table::table_cell(line, 2).starts_with("AR-")
                && matches!(
                    planning_table::table_cell(line, 5).as_str(),
                    "💤 open" | "⏳ in progress"
                )
        });
        if open {
            die("Cannot approve a review with unresolved findings", 64);
        }
        if let Ok(keys) = fs::read_to_string(plan.join("fix-keys.json")) {
            let id =
                session_id(&keys).unwrap_or_else(|| die("fix-keys.json has no session_id", 64));
            let claimant = env::var("CLAIMED_BY").unwrap_or_else(|_| id.clone());
            let verifier = env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|parent| parent.join("verify-fix-keys")))
                .unwrap_or_else(|| PathBuf::from("verify-fix-keys"));
            let result = Command::new(verifier)
                .arg(plan)
                .arg("--claimed-by")
                .arg(claimant)
                .output()
                .unwrap_or_else(|error| die(error.to_string(), 64));
            if !result.status.success() {
                let output = String::from_utf8_lossy(&result.stdout).to_string()
                    + &String::from_utf8_lossy(&result.stderr);
                die(
                    format!("Cannot approve: fix-keys verification failed: {output}"),
                    64,
                );
            }
            invalidate_session = Some(
                env::var_os("TMPDIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("planning-agent/review-fix-keys")
                    .join(id),
            );
        }
        ("`✅ approved`", "✅ approved")
    };
    let review_rendered = replace_field(&review_text, "Status", review_value)
        .unwrap_or_else(|_| die("Review must contain exactly one Status field", 64));
    let description_rendered = replace_field(&description_text, "Status", description_value)
        .unwrap_or_else(|_| die("Plan description must contain exactly one Status field", 64));
    git_snapshot(plan);
    let review_backup = review.with_file_name(format!(
        ".{}.review-backup.{}",
        review
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("review"),
        std::process::id()
    ));
    let description_backup = description.with_file_name(format!(
        ".{}.description-backup.{}",
        description
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("description"),
        std::process::id()
    ));
    fs::copy(&review, &review_backup).unwrap_or_else(|error| die(error.to_string(), 73));
    fs::copy(&description, &description_backup).unwrap_or_else(|error| die(error.to_string(), 73));
    if let Err(error) = atomic_write(&description, description_rendered.as_bytes()) {
        let _ = fs::remove_file(&review_backup);
        let _ = fs::remove_file(&description_backup);
        die(error, 73);
    }
    if let Err(error) = atomic_write(&review, review_rendered.as_bytes()) {
        let _ = fs::copy(&description_backup, &description);
        let _ = fs::remove_file(&review_backup);
        let _ = fs::remove_file(&description_backup);
        die(
            format!("could not install review status; description rolled back: {error}"),
            73,
        );
    }
    let _ = fs::remove_file(&review_backup);
    let _ = fs::remove_file(&description_backup);
    if let Some(session) = invalidate_session {
        let _ = fs::remove_dir_all(session);
    }
    invalidate_context(plan, "plan");
    println!("Updated review-status");
}

fn paragraph_args(args: &[String], section: u8) -> Result<String, String> {
    if args.is_empty() {
        return Err("Paragraphs must be supplied with repeated -p N.N: content arguments".into());
    }
    let mut records = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] != "-p" {
            return Err(format!(
                "Expected -p before paragraph {}",
                records.len() + 1
            ));
        }
        let spec = args
            .get(index + 1)
            .ok_or("Missing paragraph label after -p")?;
        let (label, inline) = spec
            .split_once(':')
            .ok_or("Paragraph must use N.N: content, for example 2.1: First paragraph")?;
        let (sec, number) = label
            .split_once('.')
            .ok_or("Paragraph must use N.N: content, for example 2.1: First paragraph")?;
        let sec_number: u8 = sec
            .parse()
            .map_err(|_| "Paragraph must use N.N: content, for example 2.1: First paragraph")?;
        let para_number: usize = number
            .parse()
            .map_err(|_| "Paragraph must use N.N: content, for example 2.1: First paragraph")?;
        if sec_number != section {
            return Err(format!(
                "Paragraph {sec}.{number} belongs to section {sec}, expected section {section}"
            ));
        }
        if para_number != records.len() + 1 {
            return Err(format!(
                "Paragraphs must be sequential, starting at {section}.1"
            ));
        }
        let mut text = inline.trim_start().to_string();
        index += 2;
        if text.is_empty() {
            let next = args
                .get(index)
                .filter(|value| value.as_str() != "-p")
                .ok_or_else(|| format!("Missing content for paragraph {label}"))?;
            text = next.clone();
            index += 1;
            while let Some(next) = args.get(index).filter(|value| value.as_str() != "-p") {
                text.push(' ');
                text.push_str(next);
                index += 1;
            }
        }
        if text.is_empty() || text.contains(['\n', '\r']) || text.contains('§') {
            return Err(format!("Paragraph {label} has invalid content"));
        }
        records.push(format!("§ {label}\n{text}"));
    }
    Ok(records.join("\n\n"))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        usage(0);
    }
    let mode = args
        .first()
        .map(String::as_str)
        .unwrap_or_else(|| usage(64));
    let mut index = 1;
    let plan = plan_dir(&args, &mut index);
    match mode {
        "-t" | "--title" => {
            if args.len() != index + 2 {
                usage(64);
            }
            let id = &args[index];
            let title = &args[index + 1];
            safe("title", title);
            let file = document_path(&plan, id).unwrap_or_else(|error| die(error, 64));
            let text = fs::read_to_string(&file).unwrap_or_else(|error| die(error.to_string(), 66));
            let rendered = replace_title(&text, title).unwrap_or_else(|error| die(error, 64));
            write_document(&plan, id, rendered, "title");
        }
        "-f" | "--field" => {
            if args.len() != index + 3 {
                usage(64);
            }
            let id = &args[index];
            let label = &args[index + 1];
            let value = &args[index + 2];
            safe(label, value);
            let file = document_path(&plan, id).unwrap_or_else(|error| die(error, 64));
            let text = fs::read_to_string(&file).unwrap_or_else(|error| die(error.to_string(), 66));
            let rendered =
                replace_field(&text, label, value).unwrap_or_else(|error| die(error, 64));
            write_document(&plan, id, rendered, "field");
        }
        "-dp"
        | "--description-paragraph"
        | "-gp"
        | "--goal-paragraph"
        | "-sp"
        | "--step-paragraph"
        | "-rp"
        | "--review-paragraph" => {
            let paragraph_index = match mode {
                "-dp" | "--description-paragraph" | "-rp" | "--review-paragraph" => index,
                "-gp" | "--goal-paragraph" => index + 1,
                _ => index + 1,
            };
            if args.len() <= paragraph_index {
                usage(64);
            }
            if args.len() == paragraph_index + 1 {
                usage(64);
            }
            let id = match mode {
                "-dp" | "--description-paragraph" => "plan".to_string(),
                "-rp" | "--review-paragraph" => "adversarial-review".to_string(),
                "-gp" | "--goal-paragraph" => format!("goal:{}", args[index]),
                _ => format!("step:{}", args[index]),
            };
            let flag = match mode {
                "-dp" | "--description-paragraph" => "-dp",
                "-gp" | "--goal-paragraph" => "-gp",
                "-sp" | "--step-paragraph" => "-sp",
                _ => "-rp",
            };
            let normalized = normalize_paragraph_id(&args[paragraph_index], flag)
                .unwrap_or_else(|error| die(error, 64));
            let raw_content = args[paragraph_index + 1..].join(" ");
            reject_swallowed_flags(&raw_content, mode).unwrap_or_else(|error| die(error, 64));
            let content = raw_content.trim_start().to_string();
            let (section_text, paragraph_text) = normalized
                .trim_end_matches(':')
                .split_once('.')
                .expect("validated paragraph id");
            let section: u8 = section_text.parse().expect("validated section number");
            let paragraph_number: usize =
                paragraph_text.parse().expect("validated paragraph number");
            if let Some(error) = paragraph_content_error("", &content) {
                die(error, 64);
            }
            safe("paragraph", &content);
            let file = document_path(&plan, &id).unwrap_or_else(|error| die(error, 64));
            let text = fs::read_to_string(&file).unwrap_or_else(|error| die(error.to_string(), 66));
            let paragraph = format!("§ {}", normalized.trim_end_matches(':'));
            let (rendered, notice) = if let Some(result) =
                auto_create_paragraph(&text, section, paragraph_number, &content)
            {
                let (rendered, notice) = result.unwrap_or_else(|error| die(error, 64));
                (rendered, Some(notice))
            } else {
                let paragraph_count = text.lines().filter(|line| *line == paragraph).count();
                let error = if paragraph_count == 0 {
                    missing_paragraph_message(&text, section, &args[paragraph_index])
                } else {
                    format!("Paragraph was not found exactly once: {paragraph}")
                };
                (
                    replace_paragraph(&text, &paragraph, &content)
                        .unwrap_or_else(|_| die(error, 64)),
                    None,
                )
            };
            write_document(&plan, &id, rendered, "paragraph");
            if let Some(notice) = notice {
                eprint!("{notice}");
            }
        }
        "-ds"
        | "--description-section"
        | "-gs"
        | "--goal-section"
        | "-ss"
        | "--step-section"
        | "-rs"
        | "--review-section" => {
            let goal_or_section = index
                + if mode == "-gs"
                    || mode == "--goal-section"
                    || mode == "-ss"
                    || mode == "--step-section"
                {
                    1
                } else {
                    0
                };
            let id = match mode {
                "-ds" | "--description-section" => "plan".to_string(),
                "-rs" | "--review-section" => "adversarial-review".to_string(),
                "-gs" | "--goal-section" => format!("goal:{}", args[index]),
                _ => format!("step:{}", args[index]),
            };
            let section_id = args.get(goal_or_section).unwrap_or_else(|| usage(64));
            let (heading, number) =
                section_spec(&id, section_id).unwrap_or_else(|error| die(error, 1));
            let body = paragraph_args(&args[goal_or_section + 1..], number)
                .unwrap_or_else(|error| die(error, 64));
            let file = document_path(&plan, &id).unwrap_or_else(|error| die(error, 64));
            let text = fs::read_to_string(&file).unwrap_or_else(|error| die(error.to_string(), 66));
            let rendered = replace_section(&text, heading, &body).unwrap_or_else(|error| {
                let code = if error.contains("holds fields") || error.contains("is a table") {
                    65
                } else {
                    64
                };
                let message = if error.contains("not found in document") {
                    planning_document::missing_section_message(
                        &file.display().to_string(),
                        heading,
                        &text,
                    )
                } else {
                    error
                };
                die(message, code)
            });
            write_document(&plan, &id, rendered, "section");
        }
        "-tp" | "--table-paragraph" => {
            if args.len() != index + 4 {
                usage(64);
            }
            let id = &args[index];
            let paragraph = format!("§ {}", args[index + 1]);
            let columns: usize = args[index + 2]
                .parse()
                .unwrap_or_else(|_| die("Table column count must be a positive integer", 64));
            let table = csv_to_markdown(columns, &args[index + 3])
                .unwrap_or_else(|error| {
                    let message = match error {
                        CsvError::InvalidColumnCount => {
                            "Table column count must be a positive integer".to_string()
                        }
                        CsvError::UnbalancedQuote(row) => format!(
                            "CSV row {row} has an unbalanced double quote; a quoted cell needs a closing quote, and a literal quote inside one is doubled"
                        ),
                        CsvError::WrongColumnCount(row, found, expected) => format!(
                            "CSV row {row} has {found} columns, expected {expected} comma-separated columns on every row"
                        ),
                        CsvError::UnescapedPipe(row, column) => format!(
                            "CSV row {row}, column {column} contains an unescaped pipe character, which would break the Markdown table; spell a literal pipe as \\| in the cell, or reword"
                        ),
                        CsvError::BlankRow(row) => format!(
                            "CSV row {row} is blank; remove the empty row rather than leaving a gap between records"
                        ),
                        CsvError::EmptyInput(expected) => format!(
                            "CSV input is empty; expected {expected} comma-separated columns on at least one row"
                        ),
                        CsvError::CarriageReturn(row) => format!(
                            "CSV row {row} contains a carriage return: the file has CRLF line endings. Convert it to LF"
                        ),
                    };
                    die(message, 65)
                });
            let file = document_path(&plan, id).unwrap_or_else(|error| die(error, 64));
            let text = fs::read_to_string(&file).unwrap_or_else(|error| die(error.to_string(), 66));
            let rendered =
                replace_paragraph(&text, &paragraph, &table).unwrap_or_else(|error| die(error, 64));
            write_document(&plan, id, rendered, "table-paragraph");
        }
        "--delete-paragraph" => {
            if args.len() != index + 2 {
                usage(64);
            }
            let id = &args[index];
            let paragraph = format!("§ {}", args[index + 1]);
            let file = document_path(&plan, id).unwrap_or_else(|error| die(error, 64));
            let text = fs::read_to_string(&file).unwrap_or_else(|error| die(error.to_string(), 66));
            let rendered =
                delete_paragraph(&text, &paragraph).unwrap_or_else(|error| die(error, 64));
            write_document(&plan, id, rendered, "delete-paragraph");
        }
        "-ia" | "--insert-after" | "-ib" | "--insert-before" => {
            if args.len() != index + 3 {
                usage(64);
            }
            let id = &args[index];
            let paragraph = format!("§ {}", args[index + 1]);
            let file = document_path(&plan, id).unwrap_or_else(|error| die(error, 64));
            let text = fs::read_to_string(&file).unwrap_or_else(|error| die(error.to_string(), 66));
            let rendered = insert_paragraph(
                &text,
                &paragraph,
                mode == "-ia" || mode == "--insert-after",
                &args[index + 2],
            )
            .unwrap_or_else(|error| die(error, 64));
            write_document(
                &plan,
                id,
                rendered,
                if mode == "-ia" || mode == "--insert-after" {
                    "insert-after"
                } else {
                    "insert-before"
                },
            );
        }
        "-ap" | "--append-paragraph" => {
            if args.len() != index + 3 {
                usage(64);
            }
            let id = &args[index];
            let section_id = &args[index + 1];
            let content = &args[index + 2];
            safe("paragraph", content);
            if content.contains(['\n', '\r']) || content.contains('§') || content.trim().is_empty()
            {
                die(
                    "Paragraph content must be one line and must not be empty",
                    64,
                );
            }
            let (_, section) = section_spec(id, section_id).unwrap_or_else(|error| die(error, 1));
            let file = document_path(&plan, id).unwrap_or_else(|error| die(error, 64));
            let text = fs::read_to_string(&file).unwrap_or_else(|error| die(error.to_string(), 66));
            let max = text
                .lines()
                .filter_map(|line| {
                    let label = line.strip_prefix("§ ")?;
                    let (left, right) = label.split_once('.')?;
                    (left.parse::<u8>().ok()? == section)
                        .then(|| right.parse::<usize>().ok())
                        .flatten()
                })
                .max()
                .unwrap_or(0);
            if max == 0 {
                die(format!("Section '{section_id}' has no numbered paragraphs; author it with the section form (-ds/-gs/-ss/-rs) from N.1"), 64);
            }
            let previous = format!("§ {section}.{max}");
            let rendered = insert_paragraph(&text, &previous, true, content)
                .unwrap_or_else(|error| die(error, 64));
            git_snapshot(&plan);
            atomic_write(&file, rendered.as_bytes()).unwrap_or_else(|error| die(error, 73));
            invalidate_context(&plan, id);
            emit_step_testing_reminder(&plan, id);
            eprintln!(
                "update-plan-content: appended paragraph § {section}.{} after § {section}.{max}",
                max + 1
            );
            println!("Updated append-paragraph");
        }
        "-tr" | "--testing-requirement" => {
            if args.len() != index + 3 {
                usage(64);
            }
            let goal = &args[index];
            let required = &args[index + 1];
            let rationale = &args[index + 2];
            if !valid_goal_name(goal) {
                die("Goal name must use 01-kebab-case", 64);
            }
            safe("rationale", rationale);
            let id = format!("goal:{goal}");
            let file = document_path(&plan, &id).unwrap_or_else(|error| die(error, 64));
            let text = fs::read_to_string(&file).unwrap_or_else(|error| die(error.to_string(), 66));
            let rendered = replace_testing_requirement(&text, required, rationale)
                .unwrap_or_else(|error| die(error, 64));
            write_document(&plan, &id, rendered, "testing-requirement");
        }
        "-dr" | "--decomposition-review" => {
            if args.len() != index + 1 {
                usage(64);
            }
            let mark = match args[index].as_str() {
                "incomplete" => ' ',
                "completed" => 'x',
                _ => die(
                    "Decomposition review status must be incomplete or completed",
                    64,
                ),
            };
            if !plan.is_dir() {
                die(format!("Plan directory not found: {}", plan.display()), 66);
            }
            let file = plan.join("work-unit-inventory.md");
            let text = fs::read_to_string(&file).unwrap_or_else(|error| die(error.to_string(), 66));
            let mut rendered = String::with_capacity(text.len());
            for line in text.split_inclusive('\n') {
                let (body, newline) = line
                    .strip_suffix('\n')
                    .map_or((line, ""), |body| (body, "\n"));
                if body.starts_with("- [") && body.as_bytes().get(4) == Some(&b']') {
                    rendered.push_str(&format!("- [{mark}]{}", &body[5..]));
                } else {
                    rendered.push_str(body);
                }
                rendered.push_str(newline);
            }
            git_snapshot(&plan);
            atomic_write(&file, rendered.as_bytes()).unwrap_or_else(|error| die(error, 73));
            invalidate_context(&plan, "plan");
            println!("Updated decomposition-review");
        }
        "-rv" | "--review-status" => {
            if args.len() != index + 1 {
                usage(64);
            }
            if !plan.is_dir() {
                die(format!("Plan directory not found: {}", plan.display()), 66);
            }
            update_review_status(&plan, &args[index]);
        }
        _ => usage(64),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        auto_create_paragraph, normalize_paragraph_id, paragraph_content_error,
        reject_swallowed_flags,
    };

    #[test]
    fn auto_creates_next_contiguous_paragraph() {
        let document =
            "# Plan: X\n\n## Current state\n\n§ 2.1\nold\n\n## Desired outcome\n\n§ 3.1\nnext\n";
        let result = auto_create_paragraph(document, 2, 2, "new");
        assert!(result.is_some(), "{result:?}");
        let (rendered, _) = result.unwrap().unwrap();
        assert!(rendered.contains("§ 2.2\nnew"));
    }

    #[test]
    fn normalizes_and_rejects_invalid_paragraph_arguments_like_shell() {
        assert_eq!(normalize_paragraph_id("2.1", "-dp").unwrap(), "2.1:");
        assert_eq!(normalize_paragraph_id("2.1:", "-dp").unwrap(), "2.1:");
        assert_eq!(
            normalize_paragraph_id("2.x", "-dp").unwrap_err(),
            "Paragraph must use N.N or N.N:"
        );
        assert!(reject_swallowed_flags("text -p 2.2: next", "-dp").is_err());
    }

    #[test]
    fn paragraph_content_errors_preserve_shell_wording() {
        assert_eq!(
            paragraph_content_error("", "").as_deref(),
            Some("Paragraph content must not be empty")
        );
        assert_eq!(
            paragraph_content_error("", "line\nbreak").as_deref(),
            Some("Paragraph content must be one line")
        );
        assert_eq!(
            paragraph_content_error("", "contains § marker").as_deref(),
            Some("Paragraph content must not contain the reserved paragraph marker §")
        );
    }
}
