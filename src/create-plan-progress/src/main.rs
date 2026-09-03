// MODE: DEV
// PACKAGE: PROD
use planning_table::goal_definition_of_done;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn usage(code: i32) -> ! {
    let name = env::args()
        .next()
        .unwrap_or_else(|| "create-plan-progress".into());
    let name = Path::new(&name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("create-plan-progress");
    println!("Usage: {name} [--plan-dir] <plan-directory>");
    println!("       {name} --help");
    std::process::exit(code)
}

fn main() {
    let mut positional: Option<PathBuf> = None;
    let args: Vec<String> = env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => usage(0),
            "--plan-dir" => {
                index += 1;
                positional = args.get(index).map(PathBuf::from);
                if positional.is_none() {
                    usage(64)
                }
            }
            "--" => {
                index += 1;
                if index != args.len() - 1 {
                    usage(64)
                }
                positional = args.get(index).map(PathBuf::from);
            }
            value if value.starts_with('-') => usage(64),
            value => {
                if positional.is_some() {
                    usage(64)
                }
                positional = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }
    let plan = positional.unwrap_or_else(|| usage(64));
    let progress = plan.join("progress.md");
    if progress.exists() {
        eprintln!("Progress file already exists: {}", progress.display());
        std::process::exit(73)
    }
    let mut goals = Vec::new();
    if let Ok(entries) = fs::read_dir(&plan) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("goal.md").is_file() {
                goals.push(path);
            }
        }
    }
    goals.sort_by_key(|path| path.file_name().map(|n| n.to_os_string()));
    if goals.is_empty() {
        eprintln!(
            "No goal directories containing goal.md found in: {}",
            plan.display()
        );
        std::process::exit(66)
    }
    let mut output = format!(
        "# Progress: {}\n\n",
        plan.file_name().unwrap().to_string_lossy()
    );
    output.push_str("**Overall progress:** `0%  #### ----------------  100%` 💤\n\n");
    output.push_str("| Goalname | Description | Completion status |\n|---|---|---|\n");
    for goal in goals {
        let name = goal.file_name().unwrap().to_string_lossy();
        let description = goal_definition_of_done(&goal.join("goal.md"), &name);
        output.push_str(&format!("| {name} | {description} | 💤 incomplete |\n"));
    }
    let temporary = plan.join(format!(".progress.md.tmp.{}", std::process::id()));
    fs::write(&temporary, output).unwrap_or_else(|error| {
        eprintln!("{}", error);
        std::process::exit(66)
    });
    fs::rename(&temporary, &progress).unwrap_or_else(|error| {
        let _ = fs::remove_file(&temporary);
        eprintln!("{}", error);
        std::process::exit(66)
    });
    println!("Created {}", progress.display());
}
