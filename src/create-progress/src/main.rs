// MODE: DEV
// PACKAGE: PROD
use planning_core::atomic_write;
use planning_progress::step_objective;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn command_name() -> String {
    env::args()
        .next()
        .and_then(|value| {
            Path::new(&value)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "create-progress".into())
        .trim_end_matches(".exe")
        .to_string()
}

fn usage(code: i32) -> ! {
    println!("Usage: {} <goal-directory> <goal-name>", command_name());
    println!("       {} --help", command_name());
    std::process::exit(code);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if matches!(args.get(1).map(String::as_str), Some("--help") | Some("-h")) {
        usage(0);
    }
    if args.len() != 3 {
        usage(64);
    }
    let goal_dir = PathBuf::from(&args[1]);
    let goal_name = &args[2];
    let steps_dir = goal_dir.join("steps");
    let progress_file = goal_dir.join("progress.md");
    if !steps_dir.is_dir() {
        eprintln!("Steps directory not found: {}", steps_dir.display());
        std::process::exit(66);
    }
    if progress_file.exists() {
        eprintln!("Progress file already exists: {}", progress_file.display());
        std::process::exit(73);
    }

    let mut steps = fs::read_dir(&steps_dir)
        .unwrap_or_else(|error| {
            eprintln!(
                "Steps directory not found: {} ({error})",
                steps_dir.display()
            );
            std::process::exit(66);
        })
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_string();
            if path.is_file() && name.ends_with(".md") && !name.ends_with("-testing.md") {
                Some((name.trim_end_matches(".md").to_string(), path))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    steps.sort_by(|left, right| left.0.cmp(&right.0));
    if steps.is_empty() {
        eprintln!("No step files found in: {}", steps_dir.display());
        std::process::exit(66);
    }

    let mut content = format!("# Progress: {goal_name}\n\n**Progress:** `0%  #### ----------------  100%` 💤\n\n| Goalname | Stepname | Description | Completion status |\n|---|---|---|---|\n");
    for (step_name, path) in steps {
        let description = step_objective(&path, &step_name).unwrap_or(step_name.clone());
        content.push_str(&format!(
            "| {goal_name} | {step_name} | {description} | 💤 incomplete |\n"
        ));
    }
    atomic_write(&progress_file, content.as_bytes()).unwrap_or_else(|error| {
        eprintln!("{}: {}", command_name(), error);
        std::process::exit(73);
    });
    println!("Created {}", progress_file.display());
}
