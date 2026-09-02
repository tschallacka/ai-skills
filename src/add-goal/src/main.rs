// MODE: DEV
// PACKAGE: PROD
use planning_core::{git_snapshot, require_safe_value};
use planning_progress::{progress_bar, progress_icon, progress_percent};
use planning_table::goal_definition_of_done;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const COMMAND: &str = "add-goal.sh";

fn usage(code: i32) -> ! {
    println!(
        "Usage: {COMMAND} [--plan-dir] <plan-directory> <goal-name> <title> <outcome>\n       {COMMAND} --help"
    );
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

fn valid_goal(value: &str) -> bool {
    value.len() >= 4
        && value.as_bytes()[0].is_ascii_digit()
        && value.as_bytes()[1].is_ascii_digit()
        && value.as_bytes()[2] == b'-'
        && value[3..]
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn rebuild_plan_progress(plan: &Path) -> Result<(), String> {
    let progress = plan.join("progress.md");
    let mut goals = Vec::new();
    for entry in fs::read_dir(plan)
        .map_err(|error| error.to_string())?
        .flatten()
    {
        let path = entry.path();
        if !path.is_dir() || !path.join("goal.md").is_file() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let description = goal_definition_of_done(&path.join("goal.md"), &name);
        let goal_progress = fs::read_to_string(path.join("progress.md")).unwrap_or_default();
        let status = if goal_progress.contains("**Progress:** `100%") {
            "✅ completed"
        } else if goal_progress.contains("⏳ in progress") {
            "⏳ in progress"
        } else {
            "💤 incomplete"
        };
        goals.push((name, description, status));
    }
    goals.sort_by(|left, right| left.0.cmp(&right.0));
    if goals.is_empty() {
        return Err("No goal directories found".into());
    }
    let completed = goals
        .iter()
        .filter(|(_, _, status)| *status == "✅ completed")
        .count();
    let total = goals.len();
    let percent = progress_percent(completed as i64, total as i64);
    let icon = progress_icon(completed as i64, percent);
    let bar = progress_bar(completed as i64, total as i64, 20);
    let plan_name = plan.file_name().unwrap().to_string_lossy();
    let mut output = format!(
        "# Progress: {plan_name}\n\n**Overall progress:** `{percent}%  {bar}  100%` {icon}\n\n| Goalname | Description | Completion status |\n|---|---|---|\n"
    );
    for (name, description, status) in goals {
        output.push_str(&format!("| {name} | {description} | {status} |\n"));
    }
    let temporary = plan.join(format!(".progress.md.tmp.{}", std::process::id()));
    fs::write(&temporary, output).map_err(|error| error.to_string())?;
    if let Err(error) = fs::rename(&temporary, progress) {
        let _ = fs::remove_file(temporary);
        return Err(error.to_string());
    }
    Ok(())
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
    let mut positional = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--plan-dir" => {
                index += 1;
                plan_option = args.get(index).cloned().or_else(|| usage(64));
            }
            value if value.starts_with("--plan-dir=") => {
                plan_option = Some(value["--plan-dir=".len()..].to_string())
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
    let values = match (plan_option, positional.as_slice()) {
        (Some(plan), [goal, title, outcome]) => {
            [plan, goal.clone(), title.clone(), outcome.clone()]
        }
        (None, [plan, goal, title, outcome]) => {
            [plan.clone(), goal.clone(), title.clone(), outcome.clone()]
        }
        _ => usage(64),
    };
    let plan = PathBuf::from(&values[0]);
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 64)
    }
    git_snapshot(&plan);
    let goal_name = &values[1];
    let title = &values[2];
    let outcome = &values[3];
    if !valid_goal(goal_name) {
        die("Goal name must use 01-kebab-case", 64)
    }
    if let Err(message) = require_safe_value("title", title) {
        die(message, 64)
    }
    if let Err(message) = require_safe_value("outcome", outcome) {
        die(message, 64)
    }
    let goal_dir = plan.join(goal_name);
    if goal_dir.exists() {
        eprintln!("{COMMAND}: Goal already exists: {}", goal_dir.display());
        std::process::exit(73)
    }
    let steps = goal_dir.join("steps");
    fs::create_dir_all(&steps).unwrap_or_else(|error| die(error.to_string(), 64));
    let goal = format!("# Goal: {title}\n\n## Current state and prior-goal handoffs\n\n§ 2.1\n<confirmed facts and prerequisite handoffs>\n\n## Outcome and definition of done\n\n§ 3.1\n{outcome}\n\n## Why this goal is needed\n\n§ 4.1\n<how this goal contributes to the initiative>\n\n## Scope\n\n§ 5.1\n<included and explicitly excluded behavior>\n\n## Affected files, systems, data, and interfaces\n\n§ 6.1\n<concrete affected areas>\n\n## Dependencies and handoffs\n\n§ 7.1\n<prerequisites and precise downstream handoffs>\n\n## Implementation approach, risks, and edge cases\n\n§ 8.1\n<approach, risks, and edge cases>\n\n## Owned work units\n\n§ 9.1\n<add work units with add-work-unit.sh>\n\n## Testing requirement\n\n| Test required | Rationale |\n|---|---|\n| no | <set to yes when this goal has a testable behavior; explain research or other untestable goals> |\n\n## Goal-size exception\n");
    let temporary = goal_dir.join(format!("goal.md.tmp.{}", std::process::id()));
    fs::write(&temporary, goal).unwrap_or_else(|error| die(error.to_string(), 64));
    if let Err(error) = fs::rename(&temporary, goal_dir.join("goal.md")) {
        let _ = fs::remove_file(temporary);
        die(error.to_string(), 64)
    }
    if rebuild_plan_progress(&plan).is_err() {
        // The shell helper delegates this best-effort rebuild after the goal
        // has landed; creation itself remains successful when the tracker is
        // absent or malformed.
    }
    println!("Created {}", goal_dir.display());
}
