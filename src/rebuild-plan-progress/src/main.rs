// MODE: DEV
// PACKAGE: PROD
use planning_core::git_snapshot;
use planning_progress::{progress_bar, progress_icon, progress_percent};
use planning_table::goal_definition_of_done;
use std::env;
use std::fs;
use std::path::PathBuf;

fn usage(code: i32) -> ! {
    println!("Usage: rebuild-plan-progress.sh [--plan-dir] <plan-directory>");
    println!("       rebuild-plan-progress.sh --help");
    println!();
    println!("Rebuilds the plan-level progress tracker from the goals' progress files.");
    std::process::exit(code);
}

fn parse_plan() -> PathBuf {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        usage(0);
    }
    let mut plan = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--plan-dir" {
            index += 1;
            plan = args.get(index).map(PathBuf::from);
        } else if let Some(value) = arg.strip_prefix("--plan-dir=") {
            plan = Some(PathBuf::from(value));
        } else if arg == "--" {
            index += 1;
            if index != args.len() - 1 {
                usage(64);
            }
            plan = args.get(index).map(PathBuf::from);
        } else if arg.starts_with('-') {
            eprintln!("rebuild-plan-progress.sh: unknown option: {arg}");
            usage(64);
        } else if plan.is_some() {
            usage(64);
        } else {
            plan = Some(PathBuf::from(arg));
        }
        index += 1;
    }
    plan.unwrap_or_else(|| usage(64))
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}", message.as_ref());
    std::process::exit(code);
}

fn goal_status(progress_text: &str) -> (&'static str, bool) {
    if progress_text.contains("**Progress:** `100%") {
        ("✅ completed", true)
    } else if progress_text.contains("⏳ in progress") {
        ("⏳ in progress", false)
    } else {
        ("💤 incomplete", false)
    }
}

fn main() {
    let plan = parse_plan();
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 66);
    }
    let progress = plan.join("progress.md");
    if !progress.is_file() {
        die(
            format!("Plan progress file not found: {}", progress.display()),
            66,
        );
    }
    git_snapshot(&plan);
    let mut goals = Vec::new();
    for entry in fs::read_dir(&plan)
        .unwrap_or_else(|error| die(error.to_string(), 66))
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() && path.join("goal.md").is_file() {
            goals.push(path);
        }
    }
    goals.sort();
    let mut completed = 0usize;
    let mut total = 0usize;
    let mut rows = String::new();
    for goal in goals {
        let name = goal
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let goal_progress = goal.join("progress.md");
        let progress_text = fs::read_to_string(&goal_progress).unwrap_or_default();
        let (status, is_completed) = goal_status(&progress_text);
        if is_completed {
            completed += 1;
        }
        total += 1;
        let description = goal_definition_of_done(&goal.join("goal.md"), &name);
        rows.push_str(&format!("| {name} | {description} | {status} |\n"));
    }
    if total == 0 {
        die("No goal directories found", 66);
    }
    let percent = progress_percent(completed as i64, total as i64);
    let bar = progress_bar(completed as i64, total as i64, 20);
    let icon = progress_icon(completed as i64, percent);
    let plan_name = plan.file_name().unwrap_or_default().to_string_lossy();
    let output = format!("# Progress: {plan_name}\n\n**Overall progress:** `{percent}%  {bar}  100%` {icon}\n\n| Goalname | Description | Completion status |\n|---|---|---|\n{rows}");
    let temporary = progress.with_file_name(format!(".progress.md.tmp.{}", std::process::id()));
    fs::write(&temporary, output).unwrap_or_else(|error| die(error.to_string(), 66));
    fs::rename(&temporary, &progress).unwrap_or_else(|error| die(error.to_string(), 66));
    println!(
        "Updated {} ({}/{} goals, {}%)",
        progress.display(),
        completed,
        total,
        percent
    );
}

#[cfg(test)]
mod tests {
    use super::goal_status;

    #[test]
    fn status_detection_matches_progress_contract() {
        assert_eq!(goal_status("**Progress:** `100%"), ("✅ completed", true));
        assert_eq!(goal_status("⏳ in progress"), ("⏳ in progress", false));
        assert_eq!(goal_status("no status"), ("💤 incomplete", false));
    }
}
