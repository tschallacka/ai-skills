// MODE: DEV
// PACKAGE: PROD
use planning_core::{atomic_write, git_snapshot};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;

const COMMAND: &str = "resolve-finding.sh";

fn usage(code: i32) -> ! {
    println!("Usage: {COMMAND} [--plan-dir] <plan-directory> <AR-NN> [--status STATUS] [--claimed-by ID]\n       {COMMAND} --help\n\n  --status STATUS   the status to record (default: resolved)\n  --claimed-by ID   the session recording the claim; refused when it equals the\n                    session that minted the keys");
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

fn cell(line: &str, column: usize) -> String {
    line.split('|')
        .nth(column.saturating_sub(1))
        .unwrap_or_default()
        .trim()
        .trim_matches('`')
        .to_string()
}

fn set_cell(line: &str, column: usize, value: &str) -> String {
    let mut parts: Vec<_> = line.split('|').map(str::to_string).collect();
    if let Some(part) = parts.get_mut(column.saturating_sub(1)) {
        *part = value.to_string();
    }
    parts.join("|")
}

fn valid_finding(value: &str) -> bool {
    value
        .strip_prefix("AR-")
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "-h" || arg == "--help")
    {
        usage(0)
    }
    let mut plan_option = None;
    let mut positional = Vec::new();
    let mut status = "resolved".to_string();
    let mut claimed_by = String::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--plan-dir" => {
                index += 1;
                plan_option = Some(args.get(index).cloned().unwrap_or_else(|| usage(64)));
            }
            "--status" => {
                index += 1;
                status = args.get(index).cloned().unwrap_or_else(|| usage(64));
            }
            "--claimed-by" => {
                index += 1;
                claimed_by = args.get(index).cloned().unwrap_or_else(|| usage(64));
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
    if let Some(plan) = plan_option {
        positional.insert(0, plan);
    }
    if positional.len() != 2 {
        usage(64)
    }
    let plan = PathBuf::from(&positional[0]);
    let finding = &positional[1];
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 66)
    }
    if !valid_finding(finding) {
        die("Finding id must use AR-NN", 64)
    }
    let review_file = plan.join("adversarial-review.md");
    if !review_file.is_file() {
        die(
            format!("adversarial-review.md not found: {}", review_file.display()),
            66,
        )
    }
    let keys_file = plan.join("fix-keys.json");
    let review =
        fs::read_to_string(&review_file).unwrap_or_else(|error| die(error.to_string(), 64));
    let target = review
        .lines()
        .find(|line| line.starts_with('|') && cell(line, 2) == *finding)
        .unwrap_or_else(|| die(format!("{finding} has no row in {}; add it with add-adversarial-finding.sh before resolving it", review_file.display()), 65));
    let work_unit = cell(target, 6);
    if work_unit.is_empty() {
        die(format!("{finding} has no row in {}; add it with add-adversarial-finding.sh before resolving it", review_file.display()), 65)
    }
    if !work_unit.starts_with('W') || !work_unit[1..].bytes().all(|byte| byte.is_ascii_digit()) {
        die(format!("{finding} is not gated on a work unit (its cell reads '{work_unit}'), so it has no key to claim"), 65)
    }
    if !claimed_by.is_empty() && keys_file.is_file() {
        let keys = fs::read_to_string(&keys_file).unwrap_or_default();
        let minted_by = serde_json::from_str::<Value>(&keys)
            .ok()
            .and_then(|value| {
                value
                    .get("minted_by")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_default();
        if claimed_by == minted_by {
            die(
                format!("refusing: {claimed_by} minted these keys, so it cannot also claim them"),
                70,
            )
        }
    }
    git_snapshot(&plan);
    let before = cell(target, 5);
    let mut output = String::new();
    for line in review.lines() {
        if line.starts_with('|') && cell(line, 2) == *finding {
            output.push_str(&set_cell(line, 5, &format!(" {status} ")));
        } else {
            output.push_str(line);
        }
        output.push('\n');
    }
    atomic_write(&review_file, output.as_bytes()).unwrap_or_else(|error| die(error, 70));
    println!("{finding}: status {before} -> {status} (gated on {work_unit})");
    if !keys_file.is_file() {
        eprintln!("{COMMAND}: no fix-keys.json yet; mint keys before claiming");
        return;
    }
    let keys = fs::read_to_string(&keys_file).unwrap_or_default();
    let key = serde_json::from_str::<Value>(&keys).ok().and_then(|value| {
        value
            .get("keys")?
            .get(finding)?
            .get(&work_unit)?
            .as_str()
            .map(str::to_string)
    });
    let Some(key) = key.filter(|value| !value.is_empty()) else {
        eprintln!("{COMMAND}: no key for {finding}/{work_unit}; re-mint after adding the row");
        return;
    };
    let fixes_file = plan.join("fixes.md");
    let existing = fs::read_to_string(&fixes_file).unwrap_or_default();
    let mut removed = 0;
    let mut claims = String::new();
    for line in existing.lines() {
        if line.starts_with(&format!("{finding}\t")) {
            removed += 1;
        } else {
            claims.push_str(line);
            claims.push('\n');
        }
    }
    if removed > 0 {
        atomic_write(&fixes_file, claims.as_bytes()).unwrap_or_else(|error| die(error, 73));
        eprintln!("{COMMAND}: dropped {removed} superseded claim row(s) for {finding}");
    }
    claims.push_str(&format!("{finding}\t{work_unit}\t{key}\n"));
    atomic_write(&fixes_file, claims.as_bytes()).unwrap_or_else(|error| die(error, 73));
    println!("Recorded fix claim {finding}/{work_unit} in fixes.md");
    eprintln!("add-fix-claim.sh: verify with verify-fix-keys.sh --claimed-by <this session>, from a session that did not mint the keys");
}
