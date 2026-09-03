// MODE: DEV
// PACKAGE: PROD
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn require_rjq() -> Result<(), String> {
    Command::new("rjq")
        .arg("--version")
        .output()
        .map(|_| ())
        .map_err(|_| {
            "rjq is required (it reads and writes the JSON registers); install rjq and re-run"
                .into()
        })
}

pub fn read(path: &Path) -> Result<Value, String> {
    serde_json::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

pub fn write(path: &Path, value: &Value) -> Result<(), String> {
    let temporary = path.with_file_name(format!(
        ".{}.tmp.{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    fs::write(
        &temporary,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
        ),
    )
    .map_err(|e| e.to_string())?;
    fs::rename(temporary, path).map_err(|e| e.to_string())
}

pub fn next_id(root: &Value, key: &str, prefix: char) -> u64 {
    root.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .filter_map(|id| {
            id.strip_prefix(prefix)
                .filter(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
                .and_then(|rest| rest.parse().ok())
        })
        .max()
        .unwrap_or(0)
        + 1
}

pub fn sort(root: &mut Value, key: &str, bugs: bool) {
    let Some(items) = root.get_mut(key).and_then(Value::as_array_mut) else {
        return;
    };
    let rank = |item: &Value| {
        let object = item.as_object().unwrap();
        let priority = match text(object, "priority") {
            "urgent" => 0,
            "high" => 1,
            "normal" => 2,
            "low" => 3,
            "someday" => 4,
            _ => 5,
        };
        let secondary = if bugs {
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
            if bugs { priority } else { secondary },
            if bugs { secondary } else { priority },
            number,
        )
    };
    items.sort_by_key(rank);
}

pub fn findings(root: &Value, key: &str, bugs: bool) -> Vec<String> {
    let Some(items) = root.get(key).and_then(Value::as_array) else {
        return vec![format!("register has no .{key} array")];
    };
    let mut result = Vec::new();
    let mut ids = std::collections::HashSet::new();
    let all_ids: std::collections::HashSet<_> = items
        .iter()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .collect();
    if bugs
        && root
            .get("skill")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
    {
        result.push("register does not name its schema skill".into());
    }
    for item in items {
        let Some(object) = item.as_object() else {
            result.push("register entry is not an object".into());
            continue;
        };
        let id = text(object, "id");
        if !ids.insert(id) {
            result.push("duplicate ids".into());
        }
        if text(object, "created_at").is_empty() {
            result.push(format!("{id}: missing created_at"));
        }
        if text(object, "updated_at").is_empty() {
            result.push(format!("{id}: missing updated_at"));
        }
        if !text(object, "parent").is_empty() && !all_ids.contains(text(object, "parent")) {
            result.push(format!(
                "{id}: parent {} does not exist",
                text(object, "parent")
            ));
        }
        let status = text(object, "status");
        let allowed = if bugs {
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
        if !allowed {
            result.push(format!(
                "{id}: unknown status {}",
                if status.is_empty() { "missing" } else { status }
            ));
        }
        if bugs {
            if !["blocking", "major", "minor", "cosmetic"].contains(&text(object, "severity")) {
                result.push(format!("{id}: unknown severity"));
            }
            if text(object, "reproduce").is_empty() {
                result.push(format!("{id}: no reproduction"));
            }
            if status == "confirmed" && text(object, "mechanism").is_empty() {
                result.push(format!("{id}: confirmed without a mechanism"));
            }
            if status == "fixed" && text(object, "verification").is_empty() {
                result.push(format!("{id}: fixed without verification"));
            }
        } else if !["urgent", "high", "normal", "low", "someday"]
            .contains(&text(object, "priority"))
        {
            result.push(format!(
                "{id}: unknown priority {}",
                text(object, "priority")
            ));
        }
    }
    result
}

pub fn text<'a>(object: &'a Map<String, Value>, key: &str) -> &'a str {
    object.get(key).and_then(Value::as_str).unwrap_or("")
}
pub fn now() -> String {
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
    use super::next_id;
    use serde_json::json;
    #[test]
    fn nonnumeric_ids_are_ignored() {
        assert_eq!(
            next_id(&json!({"tasks":[{"id":"T1e"},{"id":"T9"}]}), "tasks", 'T'),
            10
        );
    }
}
