// MODE: DEV
// PACKAGE: PROD
use planning_core::git_snapshot;
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
        .unwrap_or_else(|| "add-ui-story-links".into())
}
fn usage(code: i32) -> ! {
    let n = name();
    println!("Usage: {n} [--plan-dir] <plan-directory> <US-NN> <WNN[,WNN...]>\n       {n} --help");
    std::process::exit(code)
}
fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}: {}", name(), message.as_ref());
    std::process::exit(code)
}
fn trimmed_cell(line: &str, column: usize) -> String {
    line.split('|').nth(column).unwrap_or("").trim().to_string()
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => usage(0),
            "--plan-dir" => {
                i += 1;
                positional.push(args.get(i).cloned().unwrap_or_else(|| usage(64)));
            }
            value if value.starts_with("--plan-dir=") => {
                positional.push(value["--plan-dir=".len()..].to_string())
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
    if positional.len() != 3 {
        usage(64)
    }
    let plan = PathBuf::from(&positional[0]);
    let id = &positional[1];
    let units = &positional[2];
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 64)
    }
    if id.len() < 5 || !id.starts_with("US-") || !id[3..].bytes().all(|b| b.is_ascii_digit()) {
        die("Story ID must use US-01", 64)
    }
    let unit_parts: Vec<String> = units.split(',').map(|u| u.trim().to_string()).collect();
    if unit_parts.is_empty()
        || unit_parts.iter().any(|u| {
            u.len() < 3 || !u.starts_with('W') || !u[1..].bytes().all(|b| b.is_ascii_digit())
        })
    {
        die("Work units must be comma-separated IDs such as W01,W02", 64)
    }
    let stories = plan.join("ui-user-stories.md");
    let inventory = plan.join("work-unit-inventory.md");
    if !stories.is_file() {
        die(
            "UI story artifact not found; run create-ui-validation.sh first",
            66,
        )
    }
    if !inventory.is_file() {
        die(
            format!("Work-unit inventory not found: {}", inventory.display()),
            66,
        )
    }
    let inventory_text = fs::read_to_string(&inventory).unwrap_or_default();
    for unit in &unit_parts {
        if !inventory_text
            .lines()
            .any(|line| line.starts_with('|') && trimmed_cell(line, 1) == *unit)
        {
            die(format!("Related work unit not found: {unit}"), 66)
        }
    }
    let text = fs::read_to_string(&stories).unwrap_or_else(|e| die(e.to_string(), 64));
    if !text
        .lines()
        .any(|line| line.starts_with('|') && trimmed_cell(line, 1) == *id)
    {
        die(format!("Story ID not found: {id}"), 64)
    }
    git_snapshot(&plan);
    let mut output = String::new();
    let mut touched = false;
    for line in text.lines() {
        if line.starts_with('|') && trimmed_cell(line, 1) == *id {
            let mut cells: Vec<String> = line.split('|').map(str::to_string).collect();
            if cells.len() >= 10 {
                cells[8] = format!(" {} ", units);
                output.push_str(&cells.join("|"));
                output.push('\n');
                touched = true;
            } else {
                output.push_str(line);
                output.push('\n');
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    if !touched {
        die("UI story table row not found", 64)
    }
    let temporary = stories.with_extension(format!("md.tmp.{}", std::process::id()));
    fs::write(&temporary, output).unwrap_or_else(|e| die(e.to_string(), 66));
    fs::rename(&temporary, &stories).unwrap_or_else(|e| {
        let _ = fs::remove_file(&temporary);
        die(e.to_string(), 66)
    });
    println!("Updated {id} related_work_units={units}");
}
