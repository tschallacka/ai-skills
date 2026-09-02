// MODE: DEV
// PACKAGE: PROD
use planning_core::{git_snapshot, require_safe_value};
use std::env;
use std::fs;
use std::path::PathBuf;

const COMMAND: &str = "add-adversarial-finding.sh";

fn usage(code: i32) -> ! {
    println!(
        "Usage: {COMMAND} [--plan-dir] <plan-directory> <AR-NN> <finding> <resolution> [open|in-progress|resolved]\n       {COMMAND} [--plan-dir] <plan-directory> <AR-NN> <finding> <resolution> [--status <status>] [--work-unit <WNN>]\n       {COMMAND} --help\n\n  --status <status>     open (default), in-progress, or resolved.\n  --work-unit <WNN>     Gate the finding on a work unit; re-mints the plan's\n                        fix keys. Omitted, the Work unit cell stays N/A."
    );
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

fn cell(row: &str, column: usize) -> String {
    row.split('|')
        .nth(column.saturating_sub(1))
        .unwrap_or_default()
        .trim()
        .trim_matches('`')
        .to_string()
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|value| value == "-h" || value == "--help")
    {
        usage(0)
    }
    let mut plan_option = None;
    let mut status = None;
    let mut work_unit = None;
    let mut positional = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--plan-dir" => {
                index += 1;
                plan_option = args.get(index).cloned().or_else(|| usage(64));
            }
            value if value.starts_with("--plan-dir=") => {
                plan_option = Some(value[11..].to_string())
            }
            "--status" => {
                index += 1;
                status = args.get(index).cloned().or_else(|| usage(64));
            }
            "--work-unit" => {
                index += 1;
                work_unit = args.get(index).cloned().or_else(|| usage(64));
            }
            "--" => {
                positional.extend(args.iter().skip(index + 1).cloned());
                break;
            }
            value if value.starts_with('-') => {
                eprintln!("{COMMAND}: unknown option: {value}");
                usage(64)
            }
            value => positional.push(value.to_string()),
        }
        index += 1;
    }
    let plan_value = if let Some(plan) = plan_option {
        if positional.len() < 3 || positional.len() > 4 {
            usage(64)
        }
        plan
    } else {
        if positional.len() < 4 || positional.len() > 5 {
            usage(64)
        }
        positional.remove(0)
    };
    if positional.len() < 3 || positional.len() > 4 {
        usage(64)
    }
    let finding_id = positional[0].clone();
    let finding = positional[1].clone();
    let resolution = positional[2].clone();
    let status = status
        .or_else(|| positional.get(3).cloned())
        .unwrap_or_else(|| "open".into());
    let work_unit = work_unit.unwrap_or_else(|| "N/A".into());
    let plan = PathBuf::from(plan_value);
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 64)
    }
    git_snapshot(&plan);
    if !finding_id.starts_with("AR-")
        || finding_id.len() < 4
        || !finding_id[3..].bytes().all(|byte| byte.is_ascii_digit())
    {
        die("Finding ID must use AR-NN", 64)
    }
    if work_unit != "N/A"
        && !(work_unit.starts_with('W')
            && work_unit.len() >= 3
            && work_unit[1..].bytes().all(|byte| byte.is_ascii_digit()))
    {
        die("Work unit must be a work-unit ID such as W01", 64)
    }
    safe("finding", &finding);
    safe("resolution", &resolution);
    let status_cell = match status.as_str() {
        "open" => "💤 open",
        "in-progress" => "⏳ in progress",
        "resolved" => "✅ resolved",
        _ => die("Finding status must be open, in-progress, or resolved", 64),
    };
    let review = plan.join("adversarial-review.md");
    if !review.is_file() {
        eprintln!(
            "{COMMAND}: Adversarial review not found: {}",
            review.display()
        );
        std::process::exit(66)
    }
    let text = fs::read_to_string(&review).unwrap_or_else(|error| die(error.to_string(), 64));
    if text
        .lines()
        .any(|line| line.starts_with('|') && cell(line, 2) == finding_id)
    {
        eprintln!("{COMMAND}: Finding already exists: {finding_id}");
        std::process::exit(73)
    }
    let mut in_findings = false;
    let mut last = None;
    for (line_number, line) in text.lines().enumerate() {
        if line == "## Findings" {
            in_findings = true;
            continue;
        }
        if in_findings && line.starts_with("## ") {
            in_findings = false;
        }
        if in_findings && line.starts_with('|') {
            last = Some(line_number);
        }
    }
    let after = last.unwrap_or_else(|| {
        die(
            format!("Review has no Findings table: {}", review.display()),
            64,
        )
    });
    let row = format!("| {finding_id} | {finding} | {resolution} | {status_cell} | {work_unit} |");
    let mut output = String::new();
    for (line_number, line) in text.lines().enumerate() {
        output.push_str(line);
        output.push('\n');
        if line_number == after {
            output.push_str(&row);
            output.push('\n');
        }
    }
    fs::write(&review, output).unwrap_or_else(|error| die(error.to_string(), 64));
    if work_unit != "N/A" {
        // The fix-key re-mint is a separate Rust command and is wired here
        // once that command is migrated; ungated findings have no key output.
    }
    println!("Added {finding_id}");
}

fn safe(label: &str, value: &str) {
    if let Err(message) = require_safe_value(label, value) {
        die(message, 64)
    }
}
