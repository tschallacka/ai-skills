// MODE: DEV
// PACKAGE: PROD
use planning_core::{atomic_write, git_snapshot, require_safe_value};
use planning_inventory::{find, is_unit_id, update_row};
use planning_progress::{step_objective, table_cell};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const COMMAND: &str = "update-work-unit.sh";

fn usage(code: i32) -> ! {
    println!("Usage: {COMMAND} [--plan-dir] <plan-directory> <WNN> [<new-primary-scope>] [<new-file>] [--scope <text>] [--file <path>] [--type <type>] [--depends-on <WNN[,WNN...]|—>] [--description <text>]");
    println!(
        "       {COMMAND} [--plan-dir] <plan-directory> <WNN> --goal <goal> --step <step-name>"
    );
    println!("       {COMMAND} --help");
    println!();
    println!("A move (--goal/--step) relocates the unit between goals in one atomic edit:");
    println!("row cells, step file, its testing twin, both goals' progress trackers and");
    println!("the goal rosters move together, and every dependency edge and coverage link survives untouched.");
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

fn safe(label: &str, value: &str) {
    if let Err(message) = require_safe_value(label, value) {
        die(message, 64);
    }
}

fn replace_line(content: &str, prefix: &str, replacement: &str) -> String {
    let mut output = String::new();
    for line in content.split_inclusive('\n') {
        let (body, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        if body.starts_with(prefix) {
            output.push_str(replacement);
        } else {
            output.push_str(body);
        }
        output.push_str(newline);
    }
    output
}

fn replace_paragraph(content: &str, paragraph: &str, value: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let Some(start) = lines.iter().position(|line| *line == paragraph) else {
        return content.to_string();
    };
    let mut end = start + 1;
    while end < lines.len()
        && !lines[end].is_empty()
        && !lines[end].starts_with('§')
        && !lines[end].starts_with("## ")
    {
        end += 1;
    }
    let mut result: Vec<String> = lines[..=start].iter().map(|line| (*line).into()).collect();
    result.push(value.into());
    result.extend(lines[end..].iter().map(|line| (*line).into()));
    let mut output = result.join("\n");
    if content.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn update_goal_blurb(content: &str, unit: &str, description: &str) -> (String, bool) {
    let prefix = format!("`{unit}`");
    let mut found = false;
    let mut output = String::new();
    for line in content.split_inclusive('\n') {
        let (body, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        if body.starts_with(&prefix) {
            output.push_str(&format!("`{unit}` — {description}"));
            found = true;
        } else {
            output.push_str(body);
        }
        output.push_str(newline);
    }
    (output, found)
}

fn goal_units(inventory: &str, goal: &str) -> Vec<(String, String)> {
    planning_inventory::rows(inventory)
        .filter(|row| row.goal == goal)
        .map(|row| (row.id, row.change))
        .collect()
}

fn testing_row(goal: &str) -> String {
    let mut in_testing = false;
    for line in goal.lines() {
        if line == "## Testing requirement" {
            in_testing = true;
            continue;
        }
        if in_testing && line.starts_with("## ") {
            break;
        }
        if in_testing {
            let required = table_cell(line, 2);
            if required == "yes" || required == "no" {
                return line.to_string();
            }
        }
    }
    "| no | <rationale> |".to_string()
}

fn rewrite_owned_work_units(path: &PathBuf, inventory: &str, goal: &str) -> Result<(), String> {
    let old = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let Some(start) = old.find("## Owned work units") else {
        return Err(format!(
            "Owned work units section not found: {}",
            path.display()
        ));
    };
    let end = old[start..]
        .find("## Goal-size exception")
        .map(|offset| start + offset)
        .unwrap_or(old.len());
    let mut body = String::new();
    let units = goal_units(inventory, goal);
    for (index, (id, change)) in units.iter().enumerate() {
        body.push_str(&format!("§ 9.{}\n`{}` — {}\n\n", index + 1, id, change));
    }
    if units.is_empty() {
        body.push_str("§ 9.1\n<add work units with add-work-unit.sh>\n\n");
    }
    body.push_str("## Testing requirement\n\n| Test required | Rationale |\n|---|---|\n");
    body.push_str(&testing_row(&old));
    body.push('\n');
    let mut output = String::with_capacity(old.len());
    output.push_str(&old[..start]);
    output.push_str("## Owned work units\n\n");
    output.push_str(&body);
    output.push_str(&old[end..]);
    atomic_write(path, output.as_bytes()).map_err(|error| error.to_string())
}

fn step_files(goal_dir: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let steps = goal_dir.join("steps");
    let mut files = fs::read_dir(&steps)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_string();
            (path.is_file() && name.ends_with(".md") && !name.ends_with("-testing.md"))
                .then(|| (name.trim_end_matches(".md").to_string(), path))
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn rebuild_goal_progress(goal_dir: &Path, goal: &str) -> Result<(), String> {
    let files = step_files(goal_dir)?;
    if files.is_empty() {
        if goal_dir.join("progress.md").is_file() {
            eprintln!("plan: could not rebuild goal progress for {goal}");
        }
        return Ok(());
    }
    let progress = goal_dir.join("progress.md");
    let had_progress = progress.is_file();
    let old = fs::read_to_string(&progress).unwrap_or_default();
    let mut output = format!(
        "# Progress: {goal}\n\n**Progress:** `0%  #### ----------------  100%` 💤\n\n| Goalname | Stepname | Description | Completion status |\n|---|---|---|---|\n"
    );
    for (step, path) in files {
        let status = old
            .lines()
            .find(|line| line.starts_with('|') && table_cell(line, 3) == step)
            .map(|line| table_cell(line, 5))
            .unwrap_or_else(|| "💤 incomplete".to_string());
        let description = step_objective(&path, &step).unwrap_or(step.clone());
        output.push_str(&format!("| {goal} | {step} | {description} | {status} |\n"));
    }
    atomic_write(&progress, output.as_bytes()).map_err(|error| error.to_string())?;
    if had_progress {
        eprintln!("note: goal progress was rebuilt from step files; existing statuses carried across where step names match");
    }
    Ok(())
}

fn move_step_file(path: &Path, target: &Path, old_goal: &str, new_goal: &str) {
    let content = fs::read_to_string(path).unwrap_or_else(|error| die(error.to_string(), 73));
    let rewritten = replace_line(
        &content,
        &format!("- Goal: `{old_goal}`"),
        &format!("- Goal: `{new_goal}`"),
    );
    atomic_write(target, rewritten.as_bytes()).unwrap_or_else(|error| die(&error, 73));
    fs::remove_file(path).unwrap_or_else(|error| die(error.to_string(), 73));
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if matches!(
        args.first().map(String::as_str),
        Some("--help") | Some("-h")
    ) {
        usage(0);
    }
    if args.len() < 2 {
        usage(64);
    }
    let (plan, mut index) = if args[0] == "--plan-dir" {
        if args.len() < 2 {
            usage(64);
        }
        (PathBuf::from(&args[1]), 2)
    } else if let Some(path) = args[0].strip_prefix("--plan-dir=") {
        (PathBuf::from(path), 1)
    } else {
        (PathBuf::from(&args[0]), 1)
    };
    let unit = args.get(index).cloned().unwrap_or_else(|| usage(64));
    index += 1;
    let mut positional = Vec::new();
    let mut fields = BTreeMap::new();
    while index < args.len() {
        match args[index].as_str() {
            "--scope" | "--file" | "--type" | "--depends-on" | "--description" | "--goal"
            | "--step" => {
                if index + 1 >= args.len() {
                    usage(64);
                }
                fields.insert(args[index][2..].to_string(), args[index + 1].clone());
                index += 2;
            }
            value if value.starts_with('-') => usage(64),
            value => {
                positional.push(value.to_string());
                index += 1;
            }
        }
    }
    if positional.len() > 2 {
        usage(64);
    }
    if let Some(value) = positional.first() {
        fields.insert("scope".into(), value.clone());
    }
    if let Some(value) = positional.get(1) {
        fields.insert("file".into(), value.clone());
    }
    let move_goal = fields.get("goal").cloned().unwrap_or_default();
    let move_step = fields.get("step").cloned().unwrap_or_default();
    let has_move = !move_goal.is_empty() || !move_step.is_empty();
    let has_field_edit = ["scope", "file", "type", "depends-on", "description"]
        .iter()
        .any(|name| fields.get(*name).is_some_and(|value| !value.is_empty()));
    if has_move && has_field_edit {
        die(
            "a move (--goal/--step) cannot be combined with field edits",
            64,
        );
    }
    if has_move && (move_goal.is_empty() || move_step.is_empty()) {
        die("a move needs both --goal and --step", 64);
    }
    let has_update = fields.values().any(|value| !value.is_empty());
    if !has_update || !plan.is_dir() {
        if !plan.is_dir() {
            die(format!("Plan directory not found: {}", plan.display()), 66);
        }
        usage(64);
    }
    if !is_unit_id(&unit) {
        die("Work-unit ID must use WNN", 64);
    }
    for key in ["scope", "file", "type", "description"] {
        if let Some(value) = fields.get(key) {
            if !value.is_empty() {
                safe(key, value);
            }
        }
    }
    if let Some(depends) = fields.get("depends-on") {
        if !depends.is_empty()
            && depends != "—"
            && depends != "-"
            && !depends.split(',').all(|value| is_unit_id(value.trim()))
        {
            die("Depends-on must be a comma-separated WNN list, or —", 64);
        }
    }
    let inventory_path = plan.join("work-unit-inventory.md");
    if !inventory_path.is_file() {
        die(
            format!(
                "Work-unit inventory not found: {}",
                inventory_path.display()
            ),
            66,
        );
    }
    let inventory =
        fs::read_to_string(&inventory_path).unwrap_or_else(|error| die(error.to_string(), 66));
    let original =
        find(&inventory, &unit).unwrap_or_else(|| die(format!("Work unit not found: {unit}"), 66));
    if has_move {
        let new_goal_dir = plan.join(&move_goal);
        if !new_goal_dir.is_dir() {
            die(format!("goal not found: {move_goal}"), 66);
        }
        let valid_step = move_step.len() >= 8
            && move_step
                .as_bytes()
                .get(0..2)
                .is_some_and(|bytes| bytes[0].is_ascii_digit() && bytes[1].is_ascii_digit())
            && move_step[2..].strip_prefix("-step-").is_some_and(|suffix| {
                !suffix.is_empty()
                    && suffix
                        .chars()
                        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
            });
        if !valid_step {
            die("step name must use NN-step-kebab-case", 64);
        }
        if move_goal == original.goal && move_step == original.step {
            die("that is already this unit's goal and step", 64);
        }
        let source_step = plan
            .join(&original.goal)
            .join("steps")
            .join(format!("{}.md", original.step));
        if !source_step.is_file() {
            die(
                format!("Step file not found: {}", source_step.display()),
                66,
            );
        }
        let target_step = new_goal_dir.join("steps").join(format!("{move_step}.md"));
        if target_step.exists() {
            die(
                format!("target step already exists: {}", target_step.display()),
                64,
            );
        }
        let updated_inventory = update_row(&inventory, &unit, |parts| {
            if let Some(part) = parts.get_mut(8) {
                *part = format!(" {move_goal} ");
            }
            if let Some(part) = parts.get_mut(9) {
                *part = format!(" {move_step} ");
            }
        })
        .0;
        git_snapshot(&plan);
        atomic_write(&inventory_path, updated_inventory.as_bytes())
            .unwrap_or_else(|error| die(error, 73));
        move_step_file(&source_step, &target_step, &original.goal, &move_goal);
        let source_testing = source_step.with_file_name(format!("{}-testing.md", original.step));
        if source_testing.is_file() {
            let target_testing = target_step.with_file_name(format!("{move_step}-testing.md"));
            move_step_file(&source_testing, &target_testing, &original.goal, &move_goal);
        }
        rebuild_goal_progress(&plan.join(&original.goal), &original.goal)
            .unwrap_or_else(|error| die(error, 65));
        rebuild_goal_progress(&new_goal_dir, &move_goal).unwrap_or_else(|error| die(error, 65));
        rewrite_owned_work_units(
            &plan.join(&original.goal).join("goal.md"),
            &updated_inventory,
            &original.goal,
        )
        .unwrap_or_else(|error| die(error, 66));
        rewrite_owned_work_units(
            &new_goal_dir.join("goal.md"),
            &updated_inventory,
            &move_goal,
        )
        .unwrap_or_else(|error| die(error, 66));
        println!(
            "Moved {unit}: {}/{} -> {}/{} (edges and coverage untouched)",
            original.goal, original.step, move_goal, move_step
        );
        return;
    }
    let updates = fields.clone();
    let (inventory_updated, count) = update_row(&inventory, &unit, |parts| {
        let values = [
            ("type", 3usize),
            ("file", 4),
            ("scope", 5),
            ("description", 7),
            ("depends-on", 8),
        ];
        for (name, column) in values {
            if let Some(value) = updates.get(name) {
                if !value.is_empty() {
                    if let Some(part) = parts.get_mut(column - 1) {
                        *part = format!(" {value} ");
                    }
                }
            }
        }
    });
    if count != 1 {
        die(format!("Work unit not found: {unit}"), 66);
    }
    git_snapshot(&plan);
    atomic_write(&inventory_path, inventory_updated.as_bytes())
        .unwrap_or_else(|error| die(error, 73));
    let step_file = plan
        .join(&original.goal)
        .join("steps")
        .join(format!("{}.md", original.step));
    if !step_file.is_file() {
        die(format!("Step file not found: {}", step_file.display()), 66);
    }
    let mut step =
        fs::read_to_string(&step_file).unwrap_or_else(|error| die(error.to_string(), 66));
    if let Some(value) = updates.get("type") {
        if !value.is_empty() {
            step = replace_line(&step, "- Type:", &format!("- Type: `{value}`"));
        }
    }
    if let Some(value) = updates.get("file") {
        if !value.is_empty() {
            step = replace_line(&step, "- File:", &format!("- File: {value}"));
        }
    }
    if let Some(value) = updates.get("scope") {
        if !value.is_empty() {
            step = replace_line(
                &step,
                "- Primary symbol or file scope:",
                &format!("- Primary symbol or file scope: {value}"),
            );
        }
    }
    if let Some(value) = updates.get("description") {
        if !value.is_empty() {
            step = replace_paragraph(&step, "§ 4.1", value);
        }
    }
    atomic_write(&step_file, step.as_bytes()).unwrap_or_else(|error| die(error, 73));
    if let Some(value) = updates.get("description") {
        if !value.is_empty() {
            let goal_file = plan.join(&original.goal).join("goal.md");
            if let Ok(goal) = fs::read_to_string(&goal_file) {
                let (updated_goal, found) = update_goal_blurb(&goal, &unit, value);
                if found {
                    atomic_write(&goal_file, updated_goal.as_bytes())
                        .unwrap_or_else(|error| die(error, 73));
                } else {
                    eprintln!("{COMMAND}: {} has no `{unit}` blurb under \"## Owned work units\"; the description was not synced there", goal_file.file_name().unwrap().to_string_lossy());
                }
            }
        }
    }
    let labels = [
        ("scope", "scope"),
        ("file", "file"),
        ("type", "type"),
        ("depends-on", "depends-on"),
        ("description", "description"),
    ];
    for (name, label) in labels {
        if let Some(value) = updates.get(name) {
            let previous = match name {
                "scope" => &original.scope,
                "file" => &original.file,
                "type" => &original.kind,
                "depends-on" => &original.depends,
                _ => &original.change,
            };
            if !value.is_empty() && previous != value {
                eprintln!(
                    "replaced {unit} {label}: {} -> {value}",
                    if previous.is_empty() {
                        "(empty)"
                    } else {
                        previous
                    }
                );
            }
        }
    }
    if updates.contains_key("file")
        || updates.contains_key("scope")
        || updates.contains_key("depends-on")
    {
        if let Ok(text) = fs::read_to_string(&inventory_path) {
            let graders: Vec<_> = planning_inventory::rows(&text)
                .filter(|row| {
                    row.kind == "verification"
                        && row.depends.split(',').any(|value| value.trim() == unit)
                })
                .map(|row| row.id)
                .collect();
            if !graders.is_empty() {
                eprintln!("plan: {unit} changed behaviour; re-read its grader(s) {} — a grader checks the old behaviour until its own surfaces are updated", graders.join(" "));
            }
        }
    }
    let changed: Vec<_> = labels
        .iter()
        .filter_map(|(name, _)| {
            updates
                .get(*name)
                .filter(|value| !value.is_empty())
                .map(|_| *name)
        })
        .collect();
    println!("Updated {unit}: {}", changed.join(","));
}
