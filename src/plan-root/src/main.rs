// MODE: DEV
// PACKAGE: PROD
use planning_core::{global_scoped_root, project_root_for};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

fn command_name() -> String {
    env::args()
        .next()
        .and_then(|value| {
            Path::new(&value)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "plan-root".into())
        .trim_end_matches(".exe")
        .to_string()
}

fn usage(code: i32) -> ! {
    println!("Usage: {} resolve [directory]", command_name());
    println!("       {} project-root [directory]", command_name());
    println!("       {} --help", command_name());
    std::process::exit(code);
}

fn die(message: impl AsRef<str>) -> ! {
    eprintln!("plan-root: {}", message.as_ref());
    std::process::exit(1);
}

fn consistent_project_plans(project: &Path) -> bool {
    let candidate = project.join(".plans");
    let env_file = candidate.join(".env");
    if !candidate.is_dir() || !env_file.is_file() {
        return false;
    }
    fs::read_to_string(env_file)
        .map(|content| {
            content
                .lines()
                .any(|line| line == format!("PLANS_ROOT={}", candidate.display()))
        })
        .unwrap_or(false)
}

fn choose_root(project: &Path) -> (PathBuf, String, bool) {
    let mut answer = String::new();
    let interactive = io::stdin().is_terminal();
    if interactive {
        eprint!("Store plans globally under the tsch-ai-skills XDG home, or in this project's ./.plans? [g/p] (default: p) ");
        let _ = io::stdin().read_line(&mut answer);
    } else {
        let _ = io::stdin().read_to_string(&mut answer);
    }
    let answer = answer.trim().to_string();
    let root = match answer.as_str() {
        "g" | "G" | "global" => global_scoped_root(project).unwrap_or_else(|message| die(message)),
        "p" | "P" | "project" | "" | "y" | "Y" | "yes" | "n" | "N" | "no" => project.join(".plans"),
        value => {
            eprintln!(
                "plan-root: invalid choice ({}); using project storage",
                value
            );
            project.join(".plans")
        }
    };
    if !interactive {
        let suffix = if answer.is_empty() {
            String::new()
        } else {
            format!(" (piped answer: {})", answer)
        };
        eprintln!(
            "plan-root: automated run detected; defaulting to project storage ({}{})",
            project.join(".plans").display(),
            suffix
        );
    }
    (root, answer, interactive)
}

fn append_gitignore(project: &Path) {
    let file = project.join(".gitignore");
    let existing = fs::read_to_string(&file).unwrap_or_default();
    if existing.lines().any(|line| line == "/.plans") {
        return;
    }
    let prefix = if existing.is_empty() || existing.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    let mut handle = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .unwrap_or_else(|error| die(error.to_string()));
    writeln!(handle, "{prefix}/.plans").unwrap_or_else(|error| die(error.to_string()));
}

fn resolve(directory: Option<&str>) {
    if let Some(root) = env::var_os("PLANS_ROOT") {
        println!("{}", PathBuf::from(root).display());
        return;
    }
    let project = project_root_for(directory).unwrap_or_else(|message| die(message));
    if consistent_project_plans(&project) {
        println!("{}", project.join(".plans").display());
        return;
    }
    let scoped = global_scoped_root(&project).unwrap_or_else(|message| die(message));
    if scoped.is_dir() {
        println!("{}", scoped.display());
        return;
    }
    let (root, answer, interactive) = choose_root(&project);
    if root == project.join(".plans") {
        if !interactive && matches!(answer.as_str(), "y" | "Y" | "yes" | "YES") {
            append_gitignore(&project);
        } else if interactive {
            eprint!("Add /.plans to this project's .gitignore? [y/N] ");
            let mut confirm = String::new();
            let _ = io::stdin().read_line(&mut confirm);
            if matches!(confirm.trim(), "y" | "Y" | "yes" | "YES") {
                append_gitignore(&project);
            }
        }
    }
    println!("{}", root.display());
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--help") | Some("-h") => usage(0),
        Some("resolve") => resolve(args.get(2).map(String::as_str)),
        Some("project-root") => println!(
            "{}",
            project_root_for(args.get(2).map(String::as_str))
                .unwrap_or_else(|message| die(message))
                .display()
        ),
        _ => usage(64),
    }
}
