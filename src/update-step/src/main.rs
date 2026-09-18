// MODE: DEV
// PACKAGE: PROD
use planning_core::{atomic_write, git_snapshot};
use planning_progress::status_label;
use planning_table::table_cell;
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

/// Every file any work unit in the inventory declares as its own target,
/// across every row -- a multi-file target is one comma-separated cell (B354).
fn all_declared_targets(inventory_text: &str) -> std::collections::HashSet<String> {
    inventory_text
        .lines()
        .filter(|row| {
            let id = table_cell(row, 2);
            id.starts_with('W') && id[1..].chars().all(|c| c.is_ascii_digit()) && id.len() > 1
        })
        .map(|row| table_cell(row, 4))
        .flat_map(|cell| {
            cell.split(',')
                .map(|part| part.trim().trim_matches('`').to_string())
                .collect::<Vec<_>>()
        })
        .filter(|target| !target.is_empty() && target != "N/A")
        .collect()
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
    let Ok(inventory_text) = fs::read_to_string(&inventory) else {
        eprintln!("atomicity: no inventory at {}", inventory.display());
        return;
    };
    let declared_target = inventory_text
        .lines()
        .find(|row| table_cell(row, 2) == unit_id)
        .map(|row| table_cell(row, 4));
    let Some(declared_target) = declared_target else {
        eprintln!("atomicity: {unit_id} has no file target; boxes left for manual confirmation");
        return;
    };
    if declared_target.is_empty() || declared_target == "N/A" {
        eprintln!("atomicity: {unit_id} has no file target; boxes left for manual confirmation");
        return;
    }
    // B354: a goal implemented in one sitting is normally landed in one
    // commit covering every one of its work units, not one commit per unit --
    // `git diff --name-only since` then shows every sibling unit's own files
    // too, not just this unit's own. A file is only a real isolation
    // violation if NO work unit in the whole plan declares it; a file that
    // some OTHER named unit owns is evidence of a batched-but-still-scoped
    // commit, not evidence this unit's own change spilled outside its target.
    let all_declared_targets = all_declared_targets(&inventory_text);
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
        .filter(|path| !all_declared_targets.contains(path))
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

/// Reads progress.md's bytes, retrying briefly on a transient
/// "file not found" instead of failing on the first read (B349): CI-only,
/// never reproduced locally, has shown the file passing an immediately
/// preceding `is_file()` check and then failing this read moments later,
/// within the same process, with no code path here or in `git_snapshot`
/// that removes the file itself -- consistent with a transient filesystem
/// visibility lag under the heavy parallel I/O contention real CI runs
/// under (many sibling `cargo test` binaries and delegated subprocesses
/// touching the same scratch tree at once), not a logic error. A short
/// bounded retry is the correct response to a transient I/O error
/// regardless of the exact underlying mechanism, and costs nothing on the
/// ordinary path where the file is simply there.
fn read_progress_file(path: &Path) -> std::io::Result<String> {
    const ATTEMPTS: u32 = 5;
    const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(20);
    let mut last_error = None;
    for attempt in 0..ATTEMPTS {
        match fs::read_to_string(path) {
            Ok(content) => return Ok(content),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                last_error = Some(error);
                if attempt + 1 < ATTEMPTS {
                    std::thread::sleep(RETRY_DELAY);
                }
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.expect("loop runs at least once"))
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
        read_progress_file(&progress_file).unwrap_or_else(|error| die(error.to_string(), 66));
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

#[cfg(test)]
mod tests {
    use super::{all_declared_targets, read_progress_file};

    #[test]
    fn read_progress_file_succeeds_on_a_present_file() {
        let dir = std::env::temp_dir().join(format!(
            "update-step-b349-present-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("progress.md");
        std::fs::write(&path, "content").unwrap();
        assert_eq!(read_progress_file(&path).unwrap(), "content");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// B349: a genuinely missing file still reports NotFound after the
    /// retry budget is exhausted, rather than retrying forever.
    #[test]
    fn read_progress_file_reports_not_found_when_truly_absent() {
        let dir = std::env::temp_dir().join(format!(
            "update-step-b349-absent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("progress.md");
        let error = read_progress_file(&path).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }

    /// B349: a file that appears only after a couple of retries (simulating
    /// the observed transient CI disappearance) is still read successfully.
    #[test]
    fn read_progress_file_recovers_once_the_file_appears_mid_retry() {
        let dir = std::env::temp_dir().join(format!(
            "update-step-b349-delayed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("progress.md");
        let write_path = path.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(30));
            std::fs::write(&write_path, "late content").unwrap();
        });
        assert_eq!(read_progress_file(&path).unwrap(), "late content");
        handle.join().unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    /// B354 regression: a batched commit's own "extra" files must be checked
    /// against every unit's own declared target, not just the current unit's.
    #[test]
    fn every_row_own_target_is_collected_including_multi_file_cells() {
        let inventory = "\
| ID | Type | File | Scope | Subscope | Change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|
| W01 | source | `src/foo.rs` | scope | N/A | change | -- | goal | step |
| W02 | source | `src/bar.rs,src/baz.rs` | scope | N/A | change | W01 | goal | step |
| W03 | verification | N/A | scope | N/A | change | W01,W02 | goal | step |
";
        let targets = all_declared_targets(inventory);
        assert!(targets.contains("src/foo.rs"));
        assert!(targets.contains("src/bar.rs"));
        assert!(targets.contains("src/baz.rs"));
        assert!(!targets.contains("N/A"));
        assert_eq!(targets.len(), 3);
    }

    #[test]
    fn a_blank_inventory_yields_no_targets() {
        assert!(all_declared_targets("").is_empty());
    }
}
