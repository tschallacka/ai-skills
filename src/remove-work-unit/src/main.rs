// MODE: DEV
// PACKAGE: PROD
use planning_core::{atomic_write, git_snapshot};
use planning_progress::{progress_bar, progress_icon, progress_percent, step_objective};
use planning_table::goal_definition_of_done;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const COMMAND: &str = "remove-work-unit.sh";

fn usage(code: i32) -> ! {
    println!("Usage: {COMMAND} [--plan-dir] <plan-directory> <WNN> [--confirm-cascade]\n       {COMMAND} --help\n\nRemoves the inventory row, the id from coverage rows, the goal's Owned work\nunits section, the step file and its -testing companion, and rebuilds the goal\nand plan progress trackers (which resets completion statuses — re-apply them\nwith update-step.sh).\n\nRefuses without --confirm-cascade when other work units list this one in their\nDepends-on column; the flag prunes those links.");
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

fn set_cell(row: &str, column: usize, value: &str) -> String {
    let mut parts: Vec<_> = row.split('|').map(str::to_string).collect();
    if let Some(part) = parts.get_mut(column.saturating_sub(1)) {
        *part = value.to_string();
    }
    parts.join("|")
}

fn prune_list(raw: &str, unit: &str) -> String {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty() && *part != unit)
        .collect::<Vec<_>>()
        .join(",")
}

fn inventory_rows(text: &str) -> Vec<(String, String, String, String)> {
    text.lines()
        .filter(|line| line.starts_with('|') && cell(line, 2).starts_with('W'))
        .map(|line| (cell(line, 2), cell(line, 7), cell(line, 9), cell(line, 10)))
        .collect()
}

fn rebuild_goal(goal_file: &Path, inventory: &str, goal: &str) -> Result<(), String> {
    let old = fs::read_to_string(goal_file).map_err(|error| error.to_string())?;
    let testing = old
        .lines()
        .skip_while(|line| *line != "## Testing requirement")
        .skip(1)
        .take_while(|line| !line.starts_with("## "))
        .find(|line| cell(line, 2) == "yes" || cell(line, 2) == "no")
        .unwrap_or("| no | <rationale> |")
        .to_string();
    let units: Vec<_> = inventory_rows(inventory)
        .into_iter()
        .filter(|(_, _, row_goal, _)| row_goal == goal)
        .map(|(id, _, _, _)| id)
        .collect();
    let mut body = String::new();
    for (index, id) in units.iter().enumerate() {
        let change = inventory_rows(inventory)
            .into_iter()
            .find(|(row_id, _, row_goal, _)| row_id == id && row_goal == goal)
            .map(|(_, change, _, _)| change)
            .unwrap_or_default();
        body.push_str(&format!("§ 9.{}\n`{}` — {}\n\n", index + 1, id, change));
    }
    if units.is_empty() {
        body.push_str("§ 9.1\n<add work units with add-work-unit.sh>\n\n");
    }
    body.push_str(&format!(
        "## Testing requirement\n\n| Test required | Rationale |\n|---|---|\n{}\n",
        testing
    ));
    let Some(start) = old.find("## Owned work units") else {
        return Err(format!(
            "Owned work units section not found: {}",
            goal_file.display()
        ));
    };
    let end = old[start..]
        .find("## Goal-size exception")
        .map(|offset| start + offset)
        .unwrap_or(old.len());
    let mut output = String::new();
    output.push_str(&old[..start]);
    output.push_str("## Owned work units\n\n");
    output.push_str(&body);
    output.push_str(&old[end..]);
    atomic_write(goal_file, output.as_bytes())
}

fn rebuild_goal_progress(goal_dir: &Path, goal: &str) -> Result<(), String> {
    let steps = goal_dir.join("steps");
    let mut files: Vec<_> = fs::read_dir(&steps)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|v| v.to_str()) == Some("md")
                && !path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with("-testing.md")
        })
        .collect();
    files.sort();
    if files.is_empty() {
        return Ok(());
    }
    let progress = goal_dir.join("progress.md");
    let had_progress = progress.is_file();
    let old = fs::read_to_string(&progress).unwrap_or_default();
    let mut output = format!("# Progress: {goal}\n\n**Progress:** `0%  #### ----------------  100%` 💤\n\n| Goalname | Stepname | Description | Completion status |\n|---|---|---|---|\n");
    for path in files {
        let name = path.file_stem().unwrap().to_string_lossy();
        let status = old
            .lines()
            .find(|line| line.starts_with('|') && cell(line, 3) == name)
            .map(|line| cell(line, 5))
            .unwrap_or_else(|| "💤 incomplete".into());
        let description = step_objective(&path, &name).unwrap_or(name.to_string());
        output.push_str(&format!("| {goal} | {name} | {description} | {status} |\n"));
    }
    atomic_write(&progress, output.as_bytes())?;
    if had_progress {
        eprintln!("note: goal progress was rebuilt from step files; existing statuses carried across where step names match");
    }
    Ok(())
}

fn rebuild_plan_progress(plan: &Path) -> Result<(), String> {
    let progress = plan.join("progress.md");
    if !progress.is_file() {
        return Ok(());
    }
    let mut rows = Vec::new();
    for entry in fs::read_dir(plan)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
    {
        let dir = entry.path();
        if !dir.is_dir() || !dir.join("goal.md").is_file() {
            continue;
        }
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let goal_progress = fs::read_to_string(dir.join("progress.md")).unwrap_or_default();
        let status = if goal_progress.contains("**Progress:** `100%") {
            "✅ completed"
        } else if goal_progress.contains("⏳ in progress") {
            "⏳ in progress"
        } else {
            "💤 incomplete"
        };
        rows.push((
            name.clone(),
            goal_definition_of_done(&dir.join("goal.md"), &name),
            status,
        ));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    if rows.is_empty() {
        return Err("No goal directories found".into());
    }
    let completed = rows.iter().filter(|row| row.2 == "✅ completed").count() as i64;
    let total = rows.len() as i64;
    let percent = progress_percent(completed, total);
    let mut output = format!("# Progress: {}\n\n**Overall progress:** `{}%  {}  100%` {}\n\n| Goalname | Description | Completion status |\n|---|---|---|\n", plan.file_name().unwrap().to_string_lossy(), percent, progress_bar(completed, total, 20), progress_icon(completed, percent));
    for (name, desc, status) in rows {
        output.push_str(&format!("| {name} | {desc} | {status} |\n"));
    }
    atomic_write(&progress, output.as_bytes())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "-h" || arg == "--help")
    {
        usage(0);
    }
    let mut confirm = false;
    let mut plan = None;
    let mut positional = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--confirm-cascade" => confirm = true,
            "--plan-dir" => {
                index += 1;
                plan = Some(args.get(index).cloned().unwrap_or_else(|| usage(64)));
            }
            "--" => {
                positional.extend(args.iter().skip(index + 1).cloned());
                break;
            }
            value if value.starts_with('-') => {
                eprintln!("{COMMAND}: unknown option: {value}");
                usage(64);
            }
            value => positional.push(value.to_string()),
        }
        index += 1;
    }
    if let Some(value) = plan {
        positional.insert(0, value);
    }
    if positional.len() != 2 {
        usage(64);
    }
    let plan = PathBuf::from(&positional[0]);
    let unit = &positional[1];
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 66);
    }
    git_snapshot(&plan);
    if unit.len() < 3
        || !unit.starts_with('W')
        || !unit[1..].bytes().all(|byte| byte.is_ascii_digit())
    {
        die(
            format!("invalid work-unit id '{unit}' — must be WNN (e.g. W01, W02)"),
            64,
        );
    }
    let inventory_path = plan.join("work-unit-inventory.md");
    if !inventory_path.is_file() {
        die(
            format!(
                "work-unit inventory not found: {} (the plan appears incomplete)",
                inventory_path.display()
            ),
            66,
        );
    }
    let inventory =
        fs::read_to_string(&inventory_path).unwrap_or_else(|error| die(error.to_string(), 65));
    let target = inventory
        .lines()
        .find(|line| line.starts_with('|') && cell(line, 2) == *unit)
        .unwrap_or_else(|| {
            die(
                format!(
                    "work unit {unit} not found in {} — nothing to remove (check the id)",
                    inventory_path.display()
                ),
                66,
            )
        });
    let goal = cell(target, 9);
    let step = cell(target, 10);
    let goal_file = plan.join(&goal).join("goal.md");
    let step_file = plan.join(&goal).join("steps").join(format!("{step}.md"));
    if !goal_file.is_file() {
        die(format!("goal file missing for {unit}: {} (goal '{goal}' exists in the inventory but not on disk)", goal_file.display()), 65);
    }
    if !step_file.is_file() {
        die(format!("step file missing for {unit}: {} (rerun add-work-unit.sh to recreate it, then remove again)", step_file.display()), 65);
    }
    let dependents: Vec<_> = inventory_rows(&inventory)
        .into_iter()
        .filter(|(id, _, _, _)| id != unit)
        .filter_map(|(id, _, _, _)| {
            let row = inventory.lines().find(|line| cell(line, 2) == id)?;
            let deps = cell(row, 8);
            deps.split(',')
                .map(str::trim)
                .any(|dep| dep == unit)
                .then_some(id)
        })
        .collect();
    if !dependents.is_empty() && !confirm {
        die(format!("refusing to remove {unit}: {} still list it in Depends-on; rerun with --confirm-cascade to prune those links (and restore them after a re-add with update-work-unit.sh --depends-on)", dependents.join(" ")), 73);
    }
    let mut output = String::new();
    for line in inventory.lines() {
        if line.starts_with('|') && cell(line, 2) == *unit {
            continue;
        }
        let mut line = line.to_string();
        if line.starts_with('|') {
            if cell(&line, 2).starts_with('W') {
                let deps = cell(&line, 8);
                let pruned = prune_list(&deps, unit);
                if pruned != deps {
                    line = set_cell(&line, 8, if pruned.is_empty() { "—" } else { &pruned });
                }
            } else {
                let ids = cell(&line, 3);
                let pruned = prune_list(&ids, unit);
                if pruned != ids {
                    if pruned.is_empty() {
                        eprintln!("plan: coverage row has no remaining ids after removing {unit}; row dropped");
                        continue;
                    }
                    eprintln!("plan: pruned coverage id {unit}; remaining in row: {pruned}");
                    line = set_cell(&line, 3, &pruned);
                }
            }
        }
        output.push_str(&line);
        output.push('\n');
    }
    atomic_write(&inventory_path, output.as_bytes()).unwrap_or_else(|error| die(error, 65));
    if !dependents.is_empty() {
        for dependent in &dependents {
            eprintln!("plan: pruned Depends-on {unit} from {dependent}");
        }
        eprintln!("plan: restore pruned links with update-work-unit.sh --depends-on");
    }
    rebuild_goal(&goal_file, &output, &goal).unwrap_or_else(|error| die(error, 65));
    let _ = fs::remove_file(&step_file);
    let _ = fs::remove_file(step_file.with_file_name(format!("{step}-testing.md")));
    let _ = rebuild_goal_progress(&plan.join(&goal), &goal);
    let _ = rebuild_plan_progress(&plan);
    println!("Removed work unit {unit} ({step})");
}
