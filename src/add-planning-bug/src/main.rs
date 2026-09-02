// MODE: DEV
// PACKAGE: PROD
use planning_core::{atomic_write, git_snapshot};
use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const COMMAND: &str = "add-planning-bug.sh";

fn usage(code: i32) -> ! {
    println!(
        "Usage: {COMMAND} [--plan-dir] <plan-directory> --id <PB-NN> --title <text> \\
           --reproduce <text> --observed <text> --expected <text> \\
           [--severity blocking|major|minor|cosmetic] \\
           [--priority urgent|high|normal|low|someday] \\
           [--status reported|confirmed] [--found-by <text>]\n+       {COMMAND} --help\n\n  --id           plan-local id, PB-01 upward\n  --reproduce    the command or steps, runnable rather than described\n  --observed     what happened, quoted from the output\n  --expected     what should have happened\n  --severity     how bad it is when it happens; defaults to major\n  --priority     when it gets fixed, a separate judgement; defaults to normal\n  --status       reported (not yet reproduced) or confirmed; defaults to reported\n  --found-by     who or what found it\n\nAppends to <plan-directory>/planning-bugs.json, creating it on first use. Read it\nback with plan-content.sh get <plan-directory> planning-bugs."
    );
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

fn required(args: &[String], index: &mut usize) -> String {
    *index += 1;
    args.get(*index).cloned().unwrap_or_else(|| usage(64))
}

fn now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = seconds / 86_400;
    let day_seconds = seconds % 86_400;
    let (year, month, day) = civil_date(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        day_seconds / 3_600,
        (day_seconds / 60) % 60,
        day_seconds % 60
    )
}

// Howard Hinnant's Gregorian civil-date conversion, using only the standard library.
fn civil_date(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month, day)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut plan = None;
    let mut positional = Vec::new();
    let mut id = String::new();
    let mut title = String::new();
    let mut reproduce = String::new();
    let mut observed = String::new();
    let mut expected = String::new();
    let mut severity = "major".to_string();
    let mut priority = "normal".to_string();
    let mut status = "reported".to_string();
    let mut found_by = String::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => usage(0),
            "--plan-dir" => plan = Some(required(&args, &mut index)),
            "--id" => id = required(&args, &mut index),
            "--title" => title = required(&args, &mut index),
            "--reproduce" => reproduce = required(&args, &mut index),
            "--observed" => observed = required(&args, &mut index),
            "--expected" => expected = required(&args, &mut index),
            "--severity" => severity = required(&args, &mut index),
            "--priority" => priority = required(&args, &mut index),
            "--status" => status = required(&args, &mut index),
            "--found-by" => found_by = required(&args, &mut index),
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
    let plan = PathBuf::from(&positional[0]);
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 66)
    }
    if id.is_empty() {
        die("--id is required (a plan-local id such as PB-01)", 64)
    }
    if !id
        .strip_prefix("PB-")
        .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
    {
        die(format!("Bug id must match PB-NN: {id}"), 64)
    }
    for (name, value) in [
        ("title", &title),
        ("reproduce", &reproduce),
        ("observed", &observed),
        ("expected", &expected),
    ] {
        if value.is_empty() {
            die(
                format!("--{name} is required; an entry without it cannot be acted on"),
                64,
            )
        }
    }
    if !["blocking", "major", "minor", "cosmetic"].contains(&severity.as_str()) {
        die(
            format!("Severity must be blocking, major, minor or cosmetic: {severity}"),
            64,
        )
    }
    if !["urgent", "high", "normal", "low", "someday"].contains(&priority.as_str()) {
        die(
            format!("Priority must be urgent, high, normal or someday: {priority}"),
            64,
        )
    }
    if !["reported", "confirmed"].contains(&status.as_str()) {
        die(format!("Status must be reported or confirmed when recording a defect; close it later by editing the register with the bug-report skill: {status}"), 64)
    }

    let register = plan.join("planning-bugs.json");
    let document = if register.is_file() {
        let text = fs::read_to_string(&register).unwrap_or_else(|_| {
            die(
                format!(
                    "{} is not valid JSON; repair it before appending",
                    register.display()
                ),
                65,
            )
        });
        serde_json::from_str::<Value>(&text).unwrap_or_else(|_| {
            die(
                format!(
                    "{} is not valid JSON; repair it before appending",
                    register.display()
                ),
                65,
            )
        })
    } else {
        Value::Object(Map::new())
    };
    if let Some(bugs) = document.get("bugs").and_then(Value::as_array) {
        if bugs
            .iter()
            .any(|bug| bug.get("id").and_then(Value::as_str) == Some(id.as_str()))
        {
            die(format!("{id} is already recorded in planning-bugs.json; use a new id, or edit that entry"), 73)
        }
    }
    git_snapshot(&plan);
    let mut root = match document {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    root.insert("skill".into(), Value::String("bug-report".into()));
    root.entry("comment")
        .or_insert_with(|| Value::String("Defects found while carrying out this plan.".into()));
    let timestamp = now();
    let bug = serde_json::json!({
        "id": id, "title": title, "status": status, "severity": severity, "priority": priority,
        "parent": null, "reproduce": reproduce, "observed": observed, "expected": expected,
        "mechanism": null, "surfaces": [], "fix": null, "verification": null,
        "found_by": if found_by.is_empty() { Value::Null } else { Value::String(found_by) },
        "notes": null, "created_at": timestamp, "updated_at": timestamp
    });
    match root.get_mut("bugs") {
        Some(Value::Array(bugs)) => bugs.push(bug),
        Some(_) => die(
            "planning-bugs.json has a non-array .bugs value; repair it before appending",
            65,
        ),
        None => {
            root.insert("bugs".into(), Value::Array(vec![bug]));
        }
    }
    let mut output = serde_json::to_string_pretty(&Value::Object(root))
        .unwrap_or_else(|error| die(error.to_string(), 70));
    output.push('\n');
    atomic_write(&register, output.as_bytes()).unwrap_or_else(|error| die(error, 70));
    let written = fs::read_to_string(&register)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    if written
        .as_ref()
        .and_then(|value| value.get("bugs"))
        .and_then(Value::as_array)
        .is_none_or(|bugs| {
            !bugs
                .iter()
                .any(|bug| bug.get("id").and_then(Value::as_str) == Some(id.as_str()))
        })
    {
        die(
            format!(
                "wrote {} but {id} is not in it; the register may be damaged",
                register.display()
            ),
            70,
        )
    }
    println!("Recorded {id} in planning-bugs.json");
}
