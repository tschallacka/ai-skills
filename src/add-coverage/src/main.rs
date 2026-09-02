// MODE: DEV
// PACKAGE: PROD
use planning_core::{git_snapshot, require_safe_value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn name() -> String {
    "add-coverage.sh".into()
}

fn usage(code: i32) -> ! {
    let command = name();
    println!(
        "Usage: {command} [--plan-dir] <plan-directory> <required-outcome-or-proof> <WNN[,WNN...]> <notes> [--replace]\n       {command} --help"
    );
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}: {}", name(), message.as_ref());
    std::process::exit(code)
}

fn cell(value: &str) -> String {
    let value = value.trim();
    if value.starts_with('`') && value.ends_with('`') {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

fn safe(label: &str, value: &str) {
    if let Err(message) = require_safe_value(label, value) {
        die(message, 64)
    }
}

fn valid_work_units(value: &str) -> bool {
    let mut parts = value.split(',');
    let Some(first) = parts.next() else {
        return false;
    };
    std::iter::once(first).chain(parts).all(|part| {
        let part = part.trim();
        part.len() >= 3
            && part.starts_with('W')
            && part[1..].bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension(format!("md.tmp.{}", std::process::id()))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut plan_option = None;
    let mut replace = false;
    let mut positional = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => usage(0),
            "--replace" => replace = true,
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
                eprintln!("{}: unknown option: {value}", name());
                usage(64)
            }
            value => positional.push(value.to_string()),
        }
        index += 1;
    }

    let (plan_value, values) = match (plan_option, positional.as_slice()) {
        (Some(plan), [outcome, units, notes]) => (plan, [outcome, units, notes]),
        (None, [plan, outcome, units, notes]) => (plan.clone(), [outcome, units, notes]),
        _ => usage(64),
    };
    let outcome = values[0].as_str();
    let work_units = values[1].as_str();
    let notes = values[2].as_str();
    let plan = PathBuf::from(plan_value);
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 64)
    }
    safe("outcome", outcome);
    safe("work_units", work_units);
    safe("notes", notes);
    if !valid_work_units(work_units) {
        die("Work units must be comma-separated IDs such as W01,W02", 64)
    }

    let inventory = plan.join("work-unit-inventory.md");
    if !inventory.is_file() {
        eprintln!(
            "{}: Work-unit inventory not found: {}",
            name(),
            inventory.display()
        );
        std::process::exit(66)
    }
    git_snapshot(&plan);
    let text = fs::read_to_string(&inventory).unwrap_or_else(|error| die(error.to_string(), 64));
    let lines: Vec<&str> = text.lines().collect();
    let replaced_existing = replace
        && lines
            .iter()
            .filter(|line| line.starts_with('|'))
            .any(|line| cell(line.split('|').nth(1).unwrap_or_default()) == outcome);
    let row = format!("| {outcome} | {work_units} | {notes} |");
    let mut output = String::new();
    let mut inserted = false;
    let mut found = false;
    let mut dropped = Vec::new();

    for line in lines {
        if line == "## Work units" && !inserted {
            if !replace || !found {
                output.push_str(&row);
                output.push('\n');
                output.push('\n');
            }
            output.push_str(line);
            output.push('\n');
            inserted = true;
            continue;
        }
        if replace
            && line.starts_with('|')
            && cell(line.split('|').nth(1).unwrap_or_default()) == outcome
        {
            if !found {
                output.push_str(&row);
                output.push('\n');
                found = true;
            } else {
                dropped.push(cell(line.split('|').nth(2).unwrap_or_default()));
            }
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    if !inserted {
        die("Inventory has no Work units section", 64)
    }

    let mut record = None;
    let mut row_number = 0;
    for line in output.lines() {
        if line == "## Work units" {
            break;
        }
        if line.starts_with('|') {
            let required = cell(line.split('|').nth(1).unwrap_or_default());
            if !required.contains("Required outcome")
                && !required
                    .chars()
                    .all(|character| character == '-' || character.is_whitespace())
            {
                row_number += 1;
                if required == outcome && record.is_none() {
                    record = Some(format!("coverage:{row_number:02}"));
                }
            }
        }
    }
    let record =
        record.unwrap_or_else(|| die("Coverage row was written but could not be located", 64));
    let temporary = temporary_path(&inventory);
    fs::write(&temporary, output).unwrap_or_else(|error| die(error.to_string(), 64));
    if let Err(error) = fs::rename(&temporary, &inventory) {
        let _ = fs::remove_file(&temporary);
        die(error.to_string(), 64)
    }
    if replace {
        for units in dropped {
            eprintln!("dropped duplicate coverage row for outcome {outcome}: work units {units}");
        }
        if replaced_existing {
            println!("Replaced coverage for {work_units} ({record})");
        } else {
            println!("Added coverage for {work_units} ({record}) -- --replace matched no existing outcome, so this is a new row, not a replacement. If you meant to replace one, its wording differs; look for a stale row under the old outcome.");
        }
    } else {
        println!("Added coverage for {work_units} ({record})");
    }
}

#[cfg(test)]
mod tests {
    use super::{cell, valid_work_units};

    #[test]
    fn trims_and_strips_backticks_from_cells() {
        assert_eq!(cell(" `outcome` "), "outcome");
        assert_eq!(cell(" notes "), "notes");
    }

    #[test]
    fn accepts_only_work_unit_lists() {
        assert!(valid_work_units("W01,W02"));
        assert!(valid_work_units("W01, W02"));
        assert!(!valid_work_units("W01,"));
        assert!(!valid_work_units("goal"));
    }
}
