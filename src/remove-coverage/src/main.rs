// MODE: DEV
// PACKAGE: PROD
use planning_core::{atomic_write, git_snapshot};
use std::env;
use std::fs;
use std::path::PathBuf;

fn name() -> String {
    env::args()
        .next()
        .and_then(|p| {
            PathBuf::from(p)
                .file_name()
                .map(|v| v.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "remove-coverage".into())
}
fn usage(code: i32) -> ! {
    let n = name();
    println!("Usage: {n} [--plan-dir] <plan-directory> <required-outcome-or-proof>\n       {n} --help\n\nRemoves the coverage row whose Required outcome cell matches exactly.");
    std::process::exit(code)
}
fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}: {}", name(), message.as_ref());
    std::process::exit(code)
}
fn cell(value: &str) -> String {
    let value = value.trim();
    if value.starts_with('`') && value.ends_with('`') && value.len() >= 2 {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut plan_option = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => usage(0),
            "--plan-dir" => {
                i += 1;
                plan_option = args.get(i).cloned().or_else(|| usage(64));
            }
            value if value.starts_with("--plan-dir=") => {
                plan_option = Some(value["--plan-dir=".len()..].to_string())
            }
            "--" => {
                positional.extend(args.iter().skip(i + 1).cloned());
                break;
            }
            value if value.starts_with('-') => {
                eprintln!("{}: unknown option: {value}", name());
                usage(64)
            }
            value => positional.push(value.to_string()),
        }
        i += 1;
    }
    let (plan_value, outcome) = match (plan_option, positional.as_slice()) {
        (Some(plan), [outcome]) => (plan, outcome.clone()),
        (None, [plan, outcome]) => (plan.clone(), outcome.clone()),
        _ => usage(64),
    };
    let plan = PathBuf::from(plan_value);
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 64)
    }
    if outcome.is_empty() {
        usage(64)
    }
    if outcome.contains('|') || outcome.contains(['\n', '\r']) {
        die(
            "outcome must be one line and must not contain a Markdown table separator (|)",
            64,
        )
    }
    let inventory = plan.join("work-unit-inventory.md");
    if !inventory.is_file() {
        die(
            format!("Work-unit inventory not found: {}", inventory.display()),
            66,
        )
    };
    git_snapshot(&plan);
    let text = fs::read_to_string(&inventory).unwrap_or_else(|e| die(e.to_string(), 66));
    let mut in_coverage = false;
    let mut found = false;
    let mut units = String::new();
    let mut output = String::new();
    for line in text.lines() {
        if line.starts_with("## Definition-of-done coverage") {
            in_coverage = true;
            output.push_str(line);
            output.push('\n');
            continue;
        }
        if in_coverage && line.starts_with("## ") {
            in_coverage = false
        }
        if in_coverage && line.starts_with('|') {
            let required = cell(line.split('|').nth(1).unwrap_or(""));
            let header_or_separator = required.contains("Required outcome")
                || required.chars().all(|c| c == '-' || c.is_whitespace());
            if !header_or_separator && required == outcome {
                units = cell(line.split('|').nth(2).unwrap_or(""));
                found = true;
                continue;
            }
        }
        output.push_str(line);
        output.push('\n');
    }
    if !found {
        die(format!("no coverage row with required outcome '{outcome}' in {} (check the wording; add-coverage.sh lists rows via plan-content.sh)",inventory.display()),66)
    }
    atomic_write(&inventory, output.as_bytes()).unwrap_or_else(|e| die(e, 73));
    eprintln!(
        "dropped coverage row for outcome {outcome}: work units {}",
        if units.is_empty() { "none" } else { &units }
    );
    println!("Removed coverage for {outcome}");
}
