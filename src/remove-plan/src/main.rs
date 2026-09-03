// MODE: DEV
// PACKAGE: PROD
use planning_core::canonical_directory;
use std::env;
use std::fs;
use std::path::PathBuf;

const COMMAND: &str = "remove-plan.sh";

fn usage(code: i32) -> ! {
    println!("Usage: {COMMAND} [--plan-dir] <plan-directory>\n       {COMMAND} --help");
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "-h" || arg == "--help")
    {
        usage(0)
    }
    let mut plan = None;
    let mut positional = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
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
                usage(64)
            }
            value => positional.push(value.to_string()),
        }
        index += 1;
    }
    if let Some(plan) = plan {
        positional.push(plan);
    }
    if positional.len() != 1 {
        usage(64)
    }
    let input = PathBuf::from(&positional[0]);
    if !input.is_dir() {
        die(format!("Plan directory not found: {}", input.display()), 66)
    }
    if !input.join("plan-description.md").is_file() {
        die(
            format!(
                "not a plan directory (no plan-description.md): {}",
                input.display()
            ),
            64,
        )
    }
    let plan = canonical_directory(&input).unwrap_or_else(|_| input.clone());
    let root = canonical_directory(plan.parent().unwrap_or_else(|| std::path::Path::new(".")))
        .unwrap_or_else(|_| {
            plan.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_path_buf()
        });
    fs::remove_dir_all(&plan).unwrap_or_else(|error| die(error.to_string(), 73));
    let remaining = fs::read_dir(&root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|entry| entry.path().join("plan-description.md").is_file());
    let git = root.join(".git");
    if !remaining && git.is_dir() {
        fs::remove_dir_all(&git).unwrap_or_else(|error| die(error.to_string(), 73));
        eprintln!(
            "{COMMAND}: cleared plans-root git history at {} (no plans remain)",
            root.display()
        );
    }
    println!("Removed plan {}", input.display());
}
