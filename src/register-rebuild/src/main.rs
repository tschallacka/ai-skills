// MODE: DEV
// PACKAGE: PROD
use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn usage(code: i32) -> ! {
    eprintln!("usage: register-rebuild.sh bugs|todo [file]");
    std::process::exit(code);
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}", message.as_ref());
    std::process::exit(code);
}

fn main() {
    if env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!("register-rebuild.sh — repair a damaged register mechanically: stamp missing timestamps, drop nothing, reorder worst-first, and report every change.");
        println!();
        println!("Usage:");
        println!("  register-rebuild.sh bugs [file]");
        println!("  register-rebuild.sh todo [file]");
        println!("  register-rebuild.sh --help");
        return;
    }
    if Command::new("rjq").arg("--version").output().is_err() {
        die("register-rebuild.sh: rjq is required (it assembles the JSON state); install rjq and re-run", 69);
    }
    let args: Vec<_> = env::args().skip(1).collect();
    let kind = args
        .first()
        .map(String::as_str)
        .unwrap_or_else(|| usage(64));
    if kind != "bugs" && kind != "todo" {
        usage(64);
    }
    if args.len() > 2 {
        usage(64);
    }
    let path = args.get(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(if kind == "bugs" {
            "BUGS.json"
        } else {
            "TODO.json"
        })
    });
    if !path.is_file() {
        die(
            format!(
                "register-rebuild.sh: register not found: {}",
                path.display()
            ),
            66,
        );
    }
    let mut root: Value = serde_json::from_str(
        &fs::read_to_string(&path).unwrap_or_else(|error| die(error.to_string(), 66)),
    )
    .unwrap_or_else(|error| die(error.to_string(), 66));
    let array_key = if kind == "bugs" { "bugs" } else { "tasks" };
    let items = root
        .get_mut(array_key)
        .and_then(Value::as_array_mut)
        .unwrap_or_else(|| {
            die(
                format!(
                    "register-rebuild.sh: register has no .{array_key} array: {}",
                    path.display()
                ),
                66,
            )
        });
    let now = utc_now();
    for item in items.iter_mut() {
        let object = item
            .as_object_mut()
            .unwrap_or_else(|| die("register entry must be an object", 65));
        if missing(object, "created_at") {
            object.insert("created_at".into(), Value::String(now.clone()));
        }
        if missing(object, "updated_at") {
            object.insert("updated_at".into(), Value::String(now.clone()));
        }
        if kind == "bugs" {
            if missing(object, "severity") {
                object.insert("severity".into(), Value::String("major".into()));
            }
            if missing(object, "priority") {
                object.insert("priority".into(), Value::String("normal".into()));
            }
        } else {
            if missing(object, "status") {
                object.insert("status".into(), Value::String("open".into()));
            }
            if missing(object, "priority") {
                object.insert("priority".into(), Value::String("normal".into()));
            }
        }
    }
    sort_items(items, kind);
    let findings = findings(items, kind);
    write_json(&path, &root);
    if !findings.is_empty() {
        for finding in findings {
            eprintln!("{finding}");
        }
        die(format!("register-rebuild.sh: {} still unsound after rebuild — these need human decisions, not stamps", path.display()), 65);
    }
    if kind == "bugs" {
        let object = root.as_object_mut().unwrap();
        if !object
            .get("skill_version")
            .is_some_and(|value| value.as_str().is_some_and(|text| !text.is_empty()))
        {
            object.insert("skill_version".into(), Value::String("1.4.2".into()));
        }
        write_json(&path, &root);
    }
    println!("rebuilt {}: stamped, sorted, and sound", path.display());
}

fn missing(object: &Map<String, Value>, key: &str) -> bool {
    object
        .get(key)
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
}

fn text<'a>(object: &'a Map<String, Value>, key: &str) -> &'a str {
    object.get(key).and_then(Value::as_str).unwrap_or("")
}

fn findings(items: &[Value], kind: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for item in items {
        let Some(object) = item.as_object() else {
            result.push("register entry is not an object".into());
            continue;
        };
        let id = text(object, "id");
        if !ids.insert(id) {
            result.push("duplicate ids".into());
        }
        let status = text(object, "status");
        let valid = if kind == "bugs" {
            [
                "reported",
                "confirmed",
                "fixed",
                "not-a-defect",
                "wont-fix",
                "obsolete",
            ]
            .contains(&status)
        } else {
            ["open", "done", "blocked", "partly", "decided", "obsolete"].contains(&status)
        };
        if !valid {
            result.push(format!(
                "{id}: unknown status {}",
                if status.is_empty() { "missing" } else { status }
            ));
        }
        if kind == "bugs" {
            if !["blocking", "major", "minor", "cosmetic"].contains(&text(object, "severity")) {
                result.push(format!("{id}: unknown severity"));
            }
            if status == "confirmed" && text(object, "mechanism").is_empty() {
                result.push(format!("{id}: confirmed without a mechanism"));
            }
            if status == "fixed" && text(object, "verification").is_empty() {
                result.push(format!("{id}: fixed without verification"));
            }
            if text(object, "reproduce").is_empty() {
                result.push(format!("{id}: no reproduction"));
            }
        }
    }
    result
}

fn sort_items(items: &mut [Value], kind: &str) {
    let rank = |value: &Value| {
        let object = value.as_object().unwrap();
        let priority = match text(object, "priority") {
            "urgent" => 0,
            "high" => 1,
            "normal" => 2,
            "low" => 3,
            "someday" => 4,
            _ => 5,
        };
        let second = if kind == "bugs" {
            match text(object, "severity") {
                "blocking" => 0,
                "major" => 1,
                "minor" => 2,
                "cosmetic" => 3,
                _ => 4,
            }
        } else {
            match text(object, "status") {
                "open" => 0,
                "blocked" => 1,
                "partly" => 2,
                "decided" => 3,
                "done" => 4,
                "obsolete" => 5,
                _ => 6,
            }
        };
        let number = text(object, "id")
            .trim_start_matches(char::is_alphabetic)
            .parse::<u64>()
            .unwrap_or(u64::MAX);
        (
            if kind == "bugs" { priority } else { second },
            if kind == "bugs" { second } else { priority },
            number,
        )
    };
    items.sort_by_key(rank);
}

fn write_json(path: &PathBuf, value: &Value) {
    let text =
        serde_json::to_string_pretty(value).unwrap_or_else(|error| die(error.to_string(), 66));
    let temporary = path.with_file_name(format!(".register-rebuild.{}", std::process::id()));
    fs::write(&temporary, format!("{text}\n")).unwrap_or_else(|error| die(error.to_string(), 66));
    fs::rename(temporary, path).unwrap_or_else(|error| die(error.to_string(), 66));
}

fn utc_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = seconds / 86400;
    let rem = seconds % 86400;
    let (year, month, day) = civil(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + if m <= 2 { 1 } else { 0 }, m, d)
}

#[cfg(test)]
mod tests {
    use super::missing;
    use serde_json::Map;
    #[test]
    fn empty_stamp_is_missing() {
        let mut map = Map::new();
        map.insert("created_at".into(), "".into());
        assert!(missing(&map, "created_at"));
    }
}
