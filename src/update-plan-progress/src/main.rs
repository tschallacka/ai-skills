// MODE: DEV
// PACKAGE: PROD
use planning_core::{atomic_write, git_snapshot};
use planning_progress::{progress_bar, progress_icon, progress_percent, status_label};
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
        .unwrap_or_else(|| "update-plan-progress".into())
        .trim_end_matches(".exe")
        .to_string()
}

fn usage(code: i32) -> ! {
    println!(
        "Usage: {} [--plan-dir] <plan-directory> <goal-name> <incomplete|in-progress|completed>",
        command_name()
    );
    println!("       {} --help", command_name());
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}: {}", command_name(), message.as_ref());
    std::process::exit(code)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if matches!(
        args.first().map(String::as_str),
        Some("--help") | Some("-h")
    ) {
        usage(0);
    }

    let (plan_dir, values) = if args.first().is_some_and(|arg| arg == "--plan-dir") {
        if args.len() < 2 {
            usage(64);
        }
        (PathBuf::from(&args[1]), &args[2..])
    } else if args
        .first()
        .is_some_and(|arg| arg.starts_with("--plan-dir="))
    {
        let path = args[0].trim_start_matches("--plan-dir=");
        (PathBuf::from(path), &args[1..])
    } else {
        (
            PathBuf::from(args.first().cloned().unwrap_or_default()),
            &args[1..],
        )
    };
    if values.len() != 2 {
        usage(64);
    }
    let goal_name = &values[0];
    let requested_status = &values[1];
    let status = status_label(requested_status).unwrap_or_else(|| {
        eprintln!("Unknown status: {requested_status}");
        eprintln!("Use: incomplete, in-progress, or completed");
        std::process::exit(64)
    });
    let progress_file = plan_dir.join("progress.md");
    if !progress_file.is_file() {
        die(
            format!("Progress file not found: {}", progress_file.display()),
            66,
        );
    }

    git_snapshot(&plan_dir);
    let content =
        fs::read_to_string(&progress_file).unwrap_or_else(|error| die(error.to_string(), 66));
    let mut found = 0usize;
    let mut updated = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        let (body, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        let mut output = body.to_string();
        if body.starts_with('|') {
            let cells: Vec<&str> = body.split('|').collect();
            let goal = cells.get(1).map(|cell| cell.trim()).unwrap_or_default();
            if goal == goal_name {
                found += 1;
                if cells.len() >= 4 {
                    let last_pipe = body.rfind('|').unwrap();
                    let before_last = body[..last_pipe].rfind('|').unwrap();
                    output = format!(
                        "{} {} {}",
                        &body[..before_last + 1],
                        status,
                        &body[last_pipe..]
                    );
                }
            }
        }
        updated.push_str(&output);
        updated.push_str(newline);
    }
    if found != 1 {
        die(format!("Goal row not found exactly once: {goal_name}"), 1);
    }

    let (completed, total) = count_progress_rows_from_content(&updated);
    let percent = progress_percent(completed, total);
    let bar = progress_bar(completed, total, 20);
    let icon = progress_icon(completed, percent);
    let mut rendered = String::new();
    for line in updated.split_inclusive('\n') {
        let (body, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        if body.starts_with("**Overall progress:**") {
            rendered.push_str(&format!(
                "**Overall progress:** `{}%  {}  100%` {}",
                percent, bar, icon
            ));
        } else {
            rendered.push_str(body);
        }
        rendered.push_str(newline);
    }
    atomic_write(&progress_file, rendered.as_bytes()).unwrap_or_else(|error| die(error, 66));
    println!(
        "Updated {} ({}/{} goals, {}%)",
        progress_file.display(),
        completed,
        total,
        percent
    );
}

fn count_progress_rows_from_content(content: &str) -> (i64, i64) {
    let mut completed = 0;
    let mut total = 0;
    for row in content.lines().filter(|line| line.starts_with('|')) {
        let cells: Vec<&str> = row.split('|').collect();
        let goal = cells.get(1).map(|value| value.trim()).unwrap_or_default();
        let status = cells.get(3).map(|value| value.trim()).unwrap_or_default();
        if goal == "Goalname"
            || goal.chars().all(|ch| ch == '-')
            || status.chars().all(|ch| ch == '-')
        {
            continue;
        }
        total += 1;
        if status.contains("completed") {
            completed += 1;
        }
    }
    (completed, total)
}
