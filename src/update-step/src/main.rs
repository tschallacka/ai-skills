// MODE: DEV
// PACKAGE: PROD
use planning_core::{atomic_write, git_snapshot};
use planning_progress::{status_label, table_cell};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BOXES: [&str; 3] = [
    "This step owns exactly one inventory work unit.",
    "No other file, symbol, test target, or verification flow changes here.",
    "Any follow-on target has a separately named work unit and step.",
];

fn command_name() -> String {
    env::args()
        .next()
        .and_then(|value| {
            Path::new(&value)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "update-step".into())
        .trim_end_matches(".exe")
        .to_string()
}

fn usage(code: i32) -> ! {
    println!(
        "Usage: {} <goal-directory> <step-name> <incomplete|in-progress|completed>",
        command_name()
    );
    println!("                   [--repo-root DIR --unit WNN [--since GIT-REF]]");
    println!("       {} --help", command_name());
    println!();
    println!("With --unit and --repo-root, completion runs a mechanical atomicity check:");
    println!("the unit's declared target (inventory row) is compared against the changed");
    println!("files visible to git; matching evidence ticks the step file's boxes and extra");
    println!("paths are recorded as a VIOLATION annotation on the third box.");
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}: {}", command_name(), message.as_ref());
    std::process::exit(code)
}

fn rewrite_status(content: &str, step_name: &str, status: &str) -> Result<String, ()> {
    let mut found = 0usize;
    let mut output = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        let (body, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        let mut replacement = body.to_string();
        if body.starts_with('|') && table_cell(body, 3) == step_name {
            found += 1;
            let last_pipe = body.rfind('|').ok_or(())?;
            let previous_pipe = body[..last_pipe].rfind('|').ok_or(())?;
            replacement = format!(
                "{} {} {}",
                &body[..previous_pipe + 1],
                status,
                &body[last_pipe..]
            );
        }
        output.push_str(&replacement);
        output.push_str(newline);
    }
    (found == 1).then_some(output).ok_or(())
}

fn reset_boxes(content: &str) -> String {
    content
        .lines()
        .map(|line| {
            let mut line = line.to_string();
            for wanted in BOXES {
                if line.starts_with(&format!("- [x] {wanted}"))
                    || line.starts_with(&format!("- [X] {wanted}"))
                {
                    line = format!("- [ ] {wanted}");
                }
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if content.ends_with('\n') { "\n" } else { "" }
}

fn child_update_progress(goal_dir: &Path) -> Result<(), (i32, String)> {
    let executable = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("update-progress")))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("update-progress"));
    let output = Command::new(executable)
        .arg(goal_dir)
        .output()
        .map_err(|error| (66, error.to_string()))?;
    if !output.stdout.is_empty() {
        use std::io::Write;
        let _ = std::io::stderr().write_all(&output.stdout);
    }
    if !output.stderr.is_empty() {
        use std::io::Write;
        let _ = std::io::stderr().write_all(&output.stderr);
    }
    if output.status.success() {
        Ok(())
    } else {
        Err((output.status.code().unwrap_or(1), String::new()))
    }
}

fn atomicity_check(
    goal_dir: &Path,
    repo_root: &Path,
    unit_id: &str,
    since: &str,
    step_file: &Path,
) {
    let plan_root = goal_dir.parent().unwrap_or_else(|| Path::new("."));
    let inventory = plan_root.join("work-unit-inventory.md");
    if !inventory.is_file() {
        eprintln!("atomicity: no inventory at {}", inventory.display());
        return;
    }
    let declared_target = fs::read_to_string(&inventory).ok().and_then(|content| {
        content
            .lines()
            .find(|row| table_cell(row, 2) == unit_id)
            .map(|row| table_cell(row, 4))
    });
    let Some(declared_target) = declared_target else {
        eprintln!("atomicity: {unit_id} has no file target; boxes left for manual confirmation");
        return;
    };
    if declared_target.is_empty() || declared_target == "N/A" {
        eprintln!("atomicity: {unit_id} has no file target; boxes left for manual confirmation");
        return;
    }
    let output = Command::new("git")
        .args(["-C"])
        .arg(repo_root)
        .args(["diff", "--name-only", since])
        .output();
    let Ok(output) = output else {
        eprintln!("atomicity: unable to inspect git diff");
        return;
    };
    let plan_prefix = plan_root
        .strip_prefix(repo_root)
        .ok()
        .map(|path| path.to_string_lossy().into_owned());
    let changed: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|path| {
            plan_prefix
                .as_ref()
                .is_none_or(|prefix| !path.starts_with(&format!("{prefix}/")))
        })
        .map(str::to_string)
        .filter(|path| !path.is_empty())
        .collect();
    let extra: Vec<String> = changed
        .into_iter()
        .filter(|path| path != &declared_target)
        .collect();
    let violation = if extra.is_empty() {
        String::new()
    } else {
        format!(" VIOLATION: also touched {}", extra.join(","))
    };
    let Ok(content) = fs::read_to_string(step_file) else {
        eprintln!("atomicity: step file missing: {}", step_file.display());
        return;
    };
    let mut found = [false; 3];
    let updated = content
        .lines()
        .map(|line| {
            let mut value = line.to_string();
            for (index, wanted) in BOXES.iter().enumerate() {
                if value == format!("- [ ] {wanted}") {
                    value = format!("- [x] {wanted}{}", if index == 2 { &violation } else { "" });
                    found[index] = true;
                }
            }
            value
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if content.ends_with('\n') { "\n" } else { "" };
    if found.iter().all(|value| *value) {
        if let Err(error) = atomic_write(step_file, updated.as_bytes()) {
            eprintln!("atomicity: {error}");
        }
    } else {
        for (index, present) in found.iter().enumerate() {
            if !present {
                eprintln!("atomicity: box not found: {}", BOXES[index]);
            }
        }
    }
    if extra.is_empty() {
        eprintln!("atomicity: diff matches declared target {declared_target}; boxes ticked");
    } else {
        eprintln!("atomicity: VIOLATION — also touched: {} ", extra.join(" "));
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if matches!(
        args.first().map(String::as_str),
        Some("--help") | Some("-h")
    ) {
        usage(0);
    }
    if args.len() < 3 {
        usage(64);
    }
    let goal_dir = PathBuf::from(&args[0]);
    let step_name = &args[1];
    let requested_status = &args[2];
    let status = status_label(requested_status).unwrap_or_else(|| {
        eprintln!("Unknown status: {requested_status}");
        eprintln!("Use: incomplete, in-progress, or completed");
        std::process::exit(64)
    });
    let mut repo_root = None;
    let mut unit_id = None;
    let mut since = "HEAD".to_string();
    let mut index = 3;
    while index < args.len() {
        match args[index].as_str() {
            "--repo-root" if index + 1 < args.len() => {
                repo_root = Some(PathBuf::from(&args[index + 1]));
                index += 2;
            }
            "--unit" if index + 1 < args.len() => {
                unit_id = Some(args[index + 1].clone());
                index += 2;
            }
            "--since" if index + 1 < args.len() => {
                since = args[index + 1].clone();
                index += 2;
            }
            option => die(format!("unknown option: {option}"), 64),
        }
    }
    let progress_file = goal_dir.join("progress.md");
    if !progress_file.is_file() {
        die(
            format!("Progress file not found: {}", progress_file.display()),
            66,
        );
    }
    git_snapshot(goal_dir.parent().unwrap_or(&goal_dir));
    let content =
        fs::read_to_string(&progress_file).unwrap_or_else(|error| die(error.to_string(), 66));
    let updated = rewrite_status(&content, step_name, status)
        .unwrap_or_else(|_| die(format!("Step row not found exactly once: {step_name}"), 1));
    atomic_write(&progress_file, updated.as_bytes()).unwrap_or_else(|error| die(error, 73));
    if requested_status == "incomplete" {
        let step_file = goal_dir.join("steps").join(format!("{step_name}.md"));
        if let Ok(content) = fs::read_to_string(&step_file) {
            let reset = reset_boxes(&content);
            atomic_write(&step_file, reset.as_bytes()).unwrap_or_else(|error| die(error, 73));
        }
    }
    if let Err((code, message)) = child_update_progress(&goal_dir) {
        if !message.is_empty() {
            eprintln!("{}", message);
        }
        std::process::exit(code);
    }
    if requested_status == "completed" {
        if let (Some(repo), Some(unit)) = (repo_root.as_deref(), unit_id.as_deref()) {
            let step_file = goal_dir.join("steps").join(format!("{step_name}.md"));
            if repo.is_dir() {
                atomicity_check(&goal_dir, repo, unit, &since, &step_file);
            }
        }
    }
    println!(
        "Updated {} ({}: {})",
        progress_file.display(),
        step_name,
        requested_status
    );
}
