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
    println!("Usage: {COMMAND} [--plan-dir] <plan-directory> <WNN> [--confirm-cascade]\n       {COMMAND} --help\n\nRemoves the inventory row, the id from coverage rows (dropping a row left with\nno ids), the goal's Owned work units section, the step file and its -testing\ncompanion, and rebuilds the goal and plan progress trackers (which resets\ncompletion statuses — re-apply them with update-step.sh).\n\nRefuses without --confirm-cascade when other work units list this one in their\nDepends-on column, or UI stories in ui-user-stories.md link it; the flag prunes\nthose links. Restore them after a re-add with update-work-unit.sh --depends-on\nand update-ui-story.sh --work-units.\n\nRefuses even with the flag when a UI story links only this unit, since a story\nmust keep at least one link: relink it first with update-ui-story.sh\n<plan> <US-NN> --work-units <WNN,...>.");
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
        *part = format!(" {value} ");
    }
    parts.join("|")
}

/// Whether a comma-separated id list names `unit`, compared as `prune_list`
/// compares, so a list that only differs in spacing is not a change.
fn names(raw: &str, unit: &str) -> bool {
    raw.split(',')
        .any(|part| part.trim().trim_matches('`').trim() == unit)
}

/// Drops `unit` from a comma-separated id list. A token is compared with its
/// backticks and spaces stripped, the way the validator reads it, so a
/// hand-formatted `W02` still matches.
fn prune_list(raw: &str, unit: &str) -> String {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty() && part.trim_matches('`').trim() != unit)
        .collect::<Vec<_>>()
        .join(",")
}

/// The table rows under `## Work units`. The rows above that heading are
/// Definition-of-done coverage, whatever their first cell starts with: an
/// outcome such as "Works end to end" is not a work-unit row (GH #194).
fn unit_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .skip_while(|line| !line.starts_with("## Work units"))
        .filter(|line| line.starts_with('|'))
}

fn inventory_rows(text: &str) -> Vec<(String, String, String, String)> {
    unit_lines(text)
        .filter(|line| cell(line, 2).starts_with('W'))
        .map(|line| (cell(line, 2), cell(line, 7), cell(line, 9), cell(line, 10)))
        .collect()
}

/// The inventory with `unit`'s row removed, its id pruned from every other
/// unit's Depends-on, and from every coverage row above `## Work units`. A
/// coverage row left with no ids is dropped.
fn prune_inventory(inventory: &str, unit: &str) -> String {
    let mut output = String::new();
    let mut in_units = false;
    for line in inventory.lines() {
        if line.starts_with("## Work units") {
            in_units = true;
        }
        let mut line = line.to_string();
        if line.starts_with('|') && in_units {
            if cell(&line, 2) == unit {
                continue;
            }
            if cell(&line, 2).starts_with('W') {
                let deps = cell(&line, 8);
                let pruned = prune_list(&deps, unit);
                if names(&deps, unit) {
                    line = set_cell(&line, 8, if pruned.is_empty() { "—" } else { &pruned });
                }
            }
        } else if line.starts_with('|') {
            let ids = cell(&line, 3);
            let pruned = prune_list(&ids, unit);
            if names(&ids, unit) {
                if pruned.is_empty() {
                    eprintln!("plan: coverage row has no remaining ids after removing {unit}; row dropped");
                    continue;
                }
                eprintln!("plan: pruned coverage id {unit}; remaining in row: {pruned}");
                line = set_cell(&line, 3, &pruned);
            }
        }
        output.push_str(&line);
        output.push('\n');
    }
    output
}

#[derive(Default)]
struct StoryPrune {
    text: String,
    pruned: Vec<String>,
    emptied: Vec<String>,
}

/// `ui-user-stories.md` with `unit` pruned from every story's related work
/// units (the eighth cell). A story whose only link was `unit` is reported in
/// `emptied`: the validator requires a link, so the caller refuses rather
/// than leave a story with none (GH #194).
fn prune_story_links(text: &str, unit: &str) -> StoryPrune {
    let mut result = StoryPrune::default();
    for line in text.lines() {
        let mut line = line.to_string();
        if line.starts_with('|') && cell(&line, 2).starts_with("US-") {
            let story = cell(&line, 2);
            let related = cell(&line, 9);
            let pruned = prune_list(&related, unit);
            if names(&related, unit) {
                if pruned.is_empty() {
                    result.emptied.push(story.clone());
                }
                result.pruned.push(story);
                line = set_cell(&line, 9, &pruned);
            }
        }
        result.text.push_str(&line);
        result.text.push('\n');
    }
    result
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
    // file_name() is None for a path that is exactly "." or ".." (or the
    // filesystem root), which a caller can legitimately pass -- canonicalize
    // resolves those to a real absolute path first (B338).
    let plan_name = plan
        .canonicalize()
        .ok()
        .and_then(|resolved| {
            resolved
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .or_else(|| plan.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| ".".to_string());
    let mut output = format!("# Progress: {plan_name}\n\n**Overall progress:** `{}%  {}  100%` {}\n\n| Goalname | Description | Completion status |\n|---|---|---|\n", percent, progress_bar(completed, total, 20), progress_icon(completed, percent));
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
            value if value.starts_with("--plan-dir=") => {
                plan = Some(value["--plan-dir=".len()..].to_string());
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
    let target = unit_lines(&inventory)
        .find(|line| cell(line, 2) == *unit)
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
            let row = unit_lines(&inventory).find(|line| cell(line, 2) == id)?;
            let deps = cell(row, 8);
            (names(&deps, unit)).then_some(id)
        })
        .collect();
    let stories_path = plan.join("ui-user-stories.md");
    let stories = fs::read_to_string(&stories_path).ok();
    let story_prune = stories
        .as_deref()
        .map(|text| prune_story_links(text, unit))
        .unwrap_or_default();
    if !story_prune.emptied.is_empty() {
        die(format!("refusing to remove {unit}: {} would be left with no linked work units; relink each first with update-ui-story.sh <plan> <US-NN> --work-units <WNN,...>, then remove again", story_prune.emptied.join(" ")), 73);
    }
    if (!dependents.is_empty() || !story_prune.pruned.is_empty()) && !confirm {
        let mut holders = Vec::new();
        if !dependents.is_empty() {
            holders.push(format!(
                "{} still list it in Depends-on",
                dependents.join(" ")
            ));
        }
        if !story_prune.pruned.is_empty() {
            holders.push(format!(
                "UI stories {} still link it",
                story_prune.pruned.join(" ")
            ));
        }
        die(format!("refusing to remove {unit}: {}; rerun with --confirm-cascade to prune those links (and restore them after a re-add with update-work-unit.sh --depends-on and update-ui-story.sh --work-units)", holders.join("; ")), 73);
    }
    let output = prune_inventory(&inventory, unit);
    atomic_write(&inventory_path, output.as_bytes()).unwrap_or_else(|error| die(error, 65));
    if !dependents.is_empty() {
        for dependent in &dependents {
            eprintln!("plan: pruned Depends-on {unit} from {dependent}");
        }
        eprintln!("plan: restore pruned links with update-work-unit.sh --depends-on");
    }
    if !story_prune.pruned.is_empty() {
        atomic_write(&stories_path, story_prune.text.as_bytes())
            .unwrap_or_else(|error| die(error, 65));
        for story in &story_prune.pruned {
            eprintln!("plan: pruned work-unit link {unit} from UI story {story}");
        }
        eprintln!("plan: restore pruned story links with update-ui-story.sh --work-units");
    }
    rebuild_goal(&goal_file, &output, &goal).unwrap_or_else(|error| die(error, 65));
    let _ = fs::remove_file(&step_file);
    let _ = fs::remove_file(step_file.with_file_name(format!("{step}-testing.md")));
    let _ = rebuild_goal_progress(&plan.join(&goal), &goal);
    let _ = rebuild_plan_progress(&plan);
    println!("Removed work unit {unit} ({step})");
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVENTORY: &str = "# Work-unit inventory\n\n## Definition-of-done coverage\n\n| Required outcome or proof | Work unit IDs | Notes |\n|---|---|---|\n| Outcome one | W01,W02 | notes |\n| Works end to end | W02, W03 | notes |\n| Only two | `W02` | notes |\n\n## Work units\n\n| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n| W01 | verification | N/A | N/A | N/A | one | -- | 01-x | 01-a |\n| W02 | verification | N/A | N/A | N/A | two | W01 | 01-x | 02-b |\n| W03 | verification | N/A | N/A | N/A | three | W01, W02 | 01-x | 03-c |\n";

    #[test]
    fn coverage_rows_are_pruned_even_when_the_outcome_starts_with_w_gh194() {
        let output = prune_inventory(INVENTORY, "W02");
        assert!(output.contains("| Outcome one | W01 | notes |"), "{output}");
        assert!(
            output.contains("| Works end to end | W03 | notes |"),
            "{output}"
        );
        assert!(
            !output.contains("Only two"),
            "emptied coverage row kept: {output}"
        );
        assert!(!output.contains("| W02 |"), "unit row kept: {output}");
        assert!(!output.contains("| W01, W02 |"), "{output}");
        assert!(output.contains("| 01-x | 03-c |"), "{output}");
        assert!(output.contains("| three | W01 |"), "{output}");
    }

    #[test]
    fn a_coverage_outcome_named_like_a_unit_is_not_taken_for_its_row() {
        let inventory = INVENTORY.replace("| Outcome one |", "| W02 |");
        let rows = inventory_rows(&inventory);
        assert_eq!(rows.iter().filter(|row| row.0 == "W02").count(), 1);
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn story_links_are_pruned_and_a_story_left_with_none_is_reported_gh194() {
        let stories = "# UI user stories: x\n\n| ID | Persona | Actions | Interaction | Expected | Status | Evidence | Related work units | Cache |\n|---|---|---|---|---|---|---|---|---|\n| US-01 | a | click | click | ok | 💤 untested | — | W01,W02,W03 | `ui-story-runs/US-01.md` |\n| US-02 | a | click | click | ok | 💤 untested | — | W02 | `ui-story-runs/US-02.md` |\n| US-03 | a | click | click | ok | 💤 untested | — | W01, W03 | `ui-story-runs/US-03.md` |\n";
        let result = prune_story_links(stories, "W02");
        assert_eq!(result.pruned, vec!["US-01", "US-02"]);
        assert_eq!(result.emptied, vec!["US-02"]);
        assert!(result.text.contains("| W01,W03 |"), "{}", result.text);
        assert!(
            result.text.contains("| W01, W03 |"),
            "US-03 rewritten: {}",
            result.text
        );
    }

    #[test]
    fn names_ignores_spacing_and_backticks() {
        assert!(names("W01, `W02`", "W02"));
        assert!(!names("W01, W03", "W02"));
        assert!(!names("W020", "W02"));
    }
}
