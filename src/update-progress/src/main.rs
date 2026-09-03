// MODE: DEV
// PACKAGE: PROD
use planning_progress::{count_progress_rows, progress_bar, progress_icon, progress_percent};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

fn command_name() -> String {
    env::args()
        .next()
        .and_then(|value| {
            Path::new(&value)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "update-progress".into())
        .trim_end_matches(".exe")
        .to_string()
}

fn usage(code: i32) -> ! {
    println!("Usage: {} <goal-directory>", command_name());
    println!("       {} --help", command_name());
    std::process::exit(code);
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}: {}", command_name(), message.as_ref());
    std::process::exit(code);
}

fn atomic_write(path: &Path, content: &[u8]) {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("progress.md");
    let temporary = parent.join(format!(".{name}.tmp.{}", std::process::id()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .unwrap_or_else(|error| die(error.to_string(), 73));
    if let Ok(metadata) = fs::metadata(path) {
        let permissions = metadata.permissions();
        let _ = file.set_permissions(permissions);
    }
    file.write_all(content)
        .unwrap_or_else(|error| die(error.to_string(), 73));
    drop(file);
    fs::rename(&temporary, path).unwrap_or_else(|error| {
        let _ = fs::remove_file(&temporary);
        die(error.to_string(), 73)
    });
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--help") | Some("-h") => usage(0),
        Some(goal_dir) if args.len() == 2 => {
            let goal = PathBuf::from(goal_dir);
            let progress = goal.join("progress.md");
            if !progress.is_file() {
                die(
                    format!("Progress file not found: {}", progress.display()),
                    66,
                );
            }
            let (completed, total) =
                count_progress_rows(&progress, 5).unwrap_or_else(|message| die(message, 66));
            let percent = progress_percent(completed as i64, total as i64) as usize;
            let bar = progress_bar(completed as i64, total as i64, 20);
            let icon = progress_icon(completed as i64, percent as i64);
            let content =
                fs::read_to_string(&progress).unwrap_or_else(|error| die(error.to_string(), 66));
            let replacement = format!("**Progress:** `{}%  {}  100%` {}", percent, bar, icon);
            let mut found = false;
            let updated = content
                .lines()
                .map(|line| {
                    if line.starts_with("**Progress:**") {
                        found = true;
                        replacement.as_str()
                    } else {
                        line
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let updated = if content.ends_with('\n') {
                format!("{}\n", updated)
            } else {
                updated
            };
            if found {
                atomic_write(&progress, updated.as_bytes());
            }
            println!(
                "Updated {} ({}/{} steps, {}%)",
                progress.display(),
                completed,
                total,
                percent
            );
        }
        _ => usage(64),
    }
}
