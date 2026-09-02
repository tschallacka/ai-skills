// MODE: DEV
// PACKAGE: PROD
use planning_core::{git_snapshot, require_safe_value};
use planning_progress::step_objective;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const COMMAND: &str = "add-work-unit.sh";

fn usage(code: i32) -> ! {
    println!(
        "Usage: {COMMAND} [--plan-dir] <plan-directory> [--repo-root DIR] --id <WNN> --type <type> --file <path|N/A>\n           --scope <scope> --subscope <subscope|N/A> --change <intended change>\n           --depends-on <WNN,...|--> --goal <NN-name> --step <NN-step-name>\n       {COMMAND} --help\n\nTypes: source markup style test config docs data generated discovery verification"
    );
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

fn safe(label: &str, value: &str) {
    if let Err(message) = require_safe_value(label, value) {
        die(message, 64)
    }
}

fn cell(row: &str, column: usize) -> String {
    row.split('|')
        .nth(column.saturating_sub(1))
        .unwrap_or_default()
        .trim()
        .trim_matches('`')
        .to_string()
}

fn write_temp(path: &Path, content: &str) {
    fs::write(path, content).unwrap_or_else(|error| die(error.to_string(), 64));
}

fn replace_placeholder(goal: &str, unit: &str, intended: &str) -> Option<String> {
    let mut lines: Vec<&str> = goal.lines().collect();
    let mut found = false;
    for index in 0..lines.len() {
        if lines[index] != "§ 9.1" {
            continue;
        }
        if found {
            return None;
        }
        found = true;
        if lines.get(index + 1).copied() != Some("<add work units with add-work-unit.sh>") {
            return None;
        }
        lines[index + 1] = Box::leak(format!("`{unit}` — {intended}").into_boxed_str());
    }
    found.then(|| {
        let mut output = lines.join("\n");
        output.push('\n');
        output
    })
}

fn insert_owned(goal: &str, unit: &str, intended: &str) -> Result<String, &'static str> {
    if let Some(output) = replace_placeholder(goal, unit, intended) {
        return Ok(output);
    }
    let lines: Vec<&str> = goal.lines().collect();
    let mut max = 0usize;
    let mut in_owned = false;
    let mut testing = None;
    for (index, line) in lines.iter().enumerate() {
        if *line == "## Owned work units" {
            in_owned = true;
        } else if in_owned && line.starts_with("## ") {
            if *line == "## Testing requirement" {
                testing = Some(index);
            }
            break;
        } else if in_owned && line.starts_with("§ 9.") {
            if let Some(number) = line
                .strip_prefix("§ 9.")
                .and_then(|value| value.parse().ok())
            {
                max = max.max(number);
            }
        }
    }
    let Some(testing_index) = testing else {
        return Err("Goal has no numbered Owned work units section");
    };
    let mut insertion = testing_index;
    while insertion > 0 && lines[insertion - 1].is_empty() {
        insertion -= 1;
    }
    let mut output = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index == insertion {
            output.push_str(line);
            output.push('\n');
            output.push_str(&format!("§ 9.{}\n`{unit}` — {intended}\n\n", max + 1));
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    Ok(output)
}

fn step_files(goal_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut files: Vec<_> = fs::read_dir(goal_dir.join("steps"))
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_string();
            (path.is_file() && name.ends_with(".md") && !name.ends_with("-testing.md"))
                .then(|| (name.trim_end_matches(".md").to_string(), path))
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn make_goal_progress(goal_dir: &Path, goal: &str) -> Result<(), String> {
    let steps = step_files(goal_dir);
    if steps.is_empty() {
        return Ok(());
    }
    let progress = goal_dir.join("progress.md");
    let had_progress = progress.is_file();
    let old_status: BTreeMap<String, String> = fs::read_to_string(&progress)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.starts_with('|'))
        .filter_map(|line| {
            let step = cell(line, 3);
            (step != "Stepname" && !step.chars().all(|c| c == '-')).then(|| (step, cell(line, 5)))
        })
        .collect();
    let mut output = format!(
        "# Progress: {goal}\n\n**Progress:** `0%  #### ----------------  100%` 💤\n\n| Goalname | Stepname | Description | Completion status |\n|---|---|---|---|\n"
    );
    for (step, path) in steps {
        let description = step_objective(&path, &step).unwrap_or(step.clone());
        let status = old_status
            .get(&step)
            .map(String::as_str)
            .unwrap_or("💤 incomplete");
        output.push_str(&format!("| {goal} | {step} | {description} | {status} |\n"));
    }
    fs::write(progress, output).map_err(|error| error.to_string())?;
    if had_progress {
        eprintln!(
            "note: goal progress was rebuilt from step files; existing statuses carried across where step names match"
        );
    }
    Ok(())
}

fn make_plan_progress(plan: &Path) -> Result<(), String> {
    let progress = plan.join("progress.md");
    if !progress.is_file() {
        return Ok(());
    }
    let mut goals = Vec::new();
    for entry in fs::read_dir(plan)
        .map_err(|error| error.to_string())?
        .flatten()
    {
        let dir = entry.path();
        if !dir.is_dir() || !dir.join("goal.md").is_file() {
            continue;
        }
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let content = fs::read_to_string(dir.join("goal.md")).unwrap_or_default();
        let mut description = name.clone();
        let mut in_outcome = false;
        for line in content.lines() {
            if line == "## Outcome and definition of done" {
                in_outcome = true;
                continue;
            }
            if in_outcome && line.starts_with("## ") {
                break;
            }
            if in_outcome && !line.trim().is_empty() && !line.starts_with("§ ") {
                description = line.trim().chars().take(100).collect();
                break;
            }
        }
        let goal_progress = fs::read_to_string(dir.join("progress.md")).unwrap_or_default();
        let status = if goal_progress.contains("**Progress:** `100%") {
            "✅ completed"
        } else if goal_progress.contains("⏳ in progress") {
            "⏳ in progress"
        } else {
            "💤 incomplete"
        };
        goals.push((name, description, status));
    }
    goals.sort_by(|a, b| a.0.cmp(&b.0));
    if goals.is_empty() {
        return Err("No goal directories found".into());
    }
    let completed = goals
        .iter()
        .filter(|(_, _, status)| *status == "✅ completed")
        .count();
    let total = goals.len();
    let percent = (completed * 100 + total / 2) / total;
    let filled = percent * 20 / 100;
    let bar = format!("{}{}", "#".repeat(filled), "-".repeat(20 - filled));
    let icon = if percent == 100 {
        "✅"
    } else if completed > 0 {
        "⏳"
    } else {
        "💤"
    };
    let mut output = format!("# Progress: {}\n\n**Overall progress:** `{percent}%  {bar}  100%` {icon}\n\n| Goalname | Description | Completion status |\n|---|---|---|\n", plan.file_name().unwrap().to_string_lossy());
    for (name, description, status) in goals {
        output.push_str(&format!("| {name} | {description} | {status} |\n"));
    }
    fs::write(progress, output).map_err(|error| error.to_string())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut plan_option = None;
    let mut repo_root = env::var("PLAN_REPO_ROOT").ok();
    let mut values: BTreeMap<&str, String> = BTreeMap::new();
    let mut plan = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => usage(0),
            "--plan-dir" => {
                index += 1;
                plan_option = args.get(index).cloned().or_else(|| usage(64));
            }
            value if value.starts_with("--plan-dir=") => plan_option = Some(value[11..].into()),
            "--repo-root" => {
                index += 1;
                repo_root = Some(args.get(index).cloned().unwrap_or_else(|| usage(64)));
            }
            option @ ("--id" | "--type" | "--file" | "--scope" | "--subscope" | "--change"
            | "--depends-on" | "--goal" | "--step") => {
                index += 1;
                values.insert(
                    &option[2..],
                    args.get(index).cloned().unwrap_or_else(|| usage(64)),
                );
            }
            "--" => {
                if let Some(value) = args.get(index + 1) {
                    if plan.is_none() {
                        plan = Some(value.clone());
                    }
                }
                break;
            }
            value if value.starts_with('-') => {
                eprintln!("{COMMAND}: unknown option: {value}");
                usage(64)
            }
            value => {
                if plan.is_some() {
                    usage(64)
                }
                plan = Some(value.into());
            }
        }
        index += 1;
    }
    let plan = plan_option.or(plan).unwrap_or_else(|| usage(64));
    let required = [
        "id",
        "type",
        "file",
        "scope",
        "subscope",
        "change",
        "depends-on",
        "goal",
        "step",
    ];
    if required.iter().any(|key| !values.contains_key(key)) {
        usage(64)
    }
    let value = |key: &str| values.get(key).unwrap();
    let plan = PathBuf::from(plan);
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 64)
    }
    git_snapshot(&plan);
    let unit_id = value("id");
    let unit_type = value("type");
    let unit_file = value("file");
    let scope = value("scope");
    let subscope = value("subscope");
    let intended = value("change");
    let depends = value("depends-on");
    let goal_name = value("goal");
    let step_name = value("step");
    if !unit_id.starts_with('W')
        || unit_id.len() < 3
        || !unit_id[1..].bytes().all(|b| b.is_ascii_digit())
    {
        die("Work-unit ID must use W01", 64)
    }
    if !matches!(
        unit_type.as_str(),
        "source"
            | "markup"
            | "style"
            | "test"
            | "config"
            | "docs"
            | "data"
            | "generated"
            | "discovery"
            | "verification"
    ) {
        die(format!("Unsupported work-unit type: {unit_type}"), 64)
    }
    if !goal_name.as_bytes().first().is_some_and(u8::is_ascii_digit)
        || !goal_name.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
        || !goal_name[2..].starts_with('-')
    {
        die("Goal name must use 01-kebab-case", 64)
    }
    if !step_name.starts_with(|c: char| c.is_ascii_digit()) || !step_name.contains("-step-") {
        die("Step name must use 01-step-kebab-case", 64)
    }
    let goal_file = plan.join(goal_name).join("goal.md");
    if !goal_file.is_file() {
        eprintln!("{COMMAND}: Goal does not exist: {goal_name}");
        std::process::exit(66)
    }
    for (label, item) in [
        ("unit_file", unit_file),
        ("scope", scope),
        ("subscope", subscope),
        ("intended", intended),
        ("depends_on", depends),
    ] {
        safe(label, item);
    }
    if unit_type == "verification" && unit_file != "N/A" {
        die("Verification work units must use file N/A", 64)
    }
    if unit_type != "verification" && unit_type != "discovery" && unit_file == "N/A" {
        die(
            "Only verification and discovery work units may use file N/A",
            64,
        )
    }
    if unit_file.contains('*') || unit_file.ends_with('/') {
        die(
            "File must be one concrete file, not a glob or directory",
            64,
        )
    }
    if let Some(root) = repo_root {
        if !Path::new(&root).is_dir() {
            die(format!("repository root not found: {root}"), 66)
        }
        if !matches!(
            unit_type.as_str(),
            "discovery" | "verification" | "generated"
        ) && !matches!(scope.as_str(), "N/A")
            && !scope.starts_with('#')
            && !scope.starts_with('.')
        {
            let target = Path::new(&root).join(unit_file);
            if !target.is_file() {
                die(
                    format!("Target file does not exist under --repo-root: {unit_file}"),
                    66,
                )
            }
            let text = fs::read_to_string(target).unwrap_or_default();
            if !text.contains(scope) {
                die(
                    format!("Primary symbol or file scope was not found in {unit_file}: {scope}"),
                    66,
                )
            }
        }
    }
    let inventory = plan.join("work-unit-inventory.md");
    if !inventory.is_file() {
        eprintln!(
            "{COMMAND}: Work-unit inventory not found: {}",
            inventory.display()
        );
        std::process::exit(66)
    }
    let inventory_text =
        fs::read_to_string(&inventory).unwrap_or_else(|error| die(error.to_string(), 66));
    if inventory_text
        .lines()
        .any(|line| line.starts_with('|') && cell(line, 2) == *unit_id)
    {
        eprintln!("{COMMAND}: Work-unit ID already exists: {unit_id}");
        std::process::exit(73)
    }
    let step_file = plan
        .join(goal_name)
        .join("steps")
        .join(format!("{step_name}.md"));
    if step_file.exists() {
        eprintln!("{COMMAND}: Step already exists: {}", step_file.display());
        std::process::exit(73)
    }
    let step_number = step_name.split('-').next().unwrap_or_default();
    if let Ok(entries) = fs::read_dir(step_file.parent().unwrap()) {
        for entry in entries.flatten() {
            let file = entry.file_name().to_string_lossy().into_owned();
            if file.starts_with(&format!("{step_number}-step-"))
                && file.ends_with(".md")
                && !file.ends_with("-testing.md")
            {
                eprintln!("{COMMAND}: step number {step_number} is already used by {file} in {goal_name}; pick the next free number");
                std::process::exit(73)
            }
        }
    }
    let row = format!("| {unit_id} | {unit_type} | `{unit_file}` | `{scope}` | `{subscope}` | {intended} | {depends} | {goal_name} | {step_name} |\n");
    let mut inventory_out = String::new();
    let mut inserted = false;
    for line in inventory_text.lines() {
        if line == "## Decomposition review" && !inserted {
            inventory_out.push_str(&row);
            inventory_out.push('\n');
            inserted = true;
        }
        inventory_out.push_str(line);
        inventory_out.push('\n');
    }
    if !inserted {
        die("Inventory has no Decomposition review section", 64)
    }
    let step = format!("# Step: {step_name}\n\n## Ownership\n\n- Goal: `{goal_name}`\n- Work unit: `{unit_id}`\n- Type: `{unit_type}`\n\n## Change target\n\n- File: `{unit_file}`\n- Primary symbol or file scope: `{scope}`\n- Subscope: `{subscope}`\n\n## Objective\n\n§ 4.1\n{intended}\n\n## Instructions\n\n§ 5.1\n<direct action on this one target>\n\n## Acceptance criteria\n\n§ 6.1\n<observable result for this target>\n\n## Handoff\n\n§ 7.1\n<what the next named work unit can rely on>\n\n## Atomicity check\n\n- [ ] This step owns exactly one inventory work unit.\n- [ ] No other file, symbol, test target, or verification flow changes here.\n- [ ] Any follow-on target has a separately named work unit and step.\n");
    let goal_text =
        fs::read_to_string(&goal_file).unwrap_or_else(|error| die(error.to_string(), 64));
    let owned =
        insert_owned(&goal_text, unit_id, intended).unwrap_or_else(|message| die(message, 64));
    let inventory_tmp = inventory.with_extension(format!("md.tmp.{}", std::process::id()));
    let step_tmp = step_file.with_extension(format!("md.tmp.{}", std::process::id()));
    let owned_tmp = goal_file.with_extension(format!("md.tmp.{}", std::process::id()));
    write_temp(&inventory_tmp, &inventory_out);
    write_temp(&step_tmp, &step);
    write_temp(&owned_tmp, &owned);
    fs::rename(&inventory_tmp, &inventory).unwrap_or_else(|error| die(error.to_string(), 64));
    fs::rename(&step_tmp, &step_file).unwrap_or_else(|error| die(error.to_string(), 64));
    fs::rename(&owned_tmp, &goal_file).unwrap_or_else(|error| die(error.to_string(), 64));
    println!("Added {unit_id} and {}", step_file.display());
    let testing = goal_text
        .lines()
        .skip_while(|line| *line != "## Testing requirement")
        .skip(1)
        .find_map(|line| {
            let value = cell(line, 2);
            (value == "yes" || value == "no").then_some(value)
        })
        .unwrap_or_default();
    if testing == "yes" && unit_type != "test" && unit_type != "verification" {
        eprintln!("Reminder: goal {goal_name} requires testing; continue with its test/proof step. Review any existing testing instructions for accuracy and completeness.");
    }
    make_goal_progress(&plan.join(goal_name), goal_name).unwrap_or(());
    make_plan_progress(&plan).unwrap_or(());
}
