// MODE: DEV
// PACKAGE: PROD
use serde_json::Value;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

fn usage(code: i32) -> ! {
    println!("Usage: register-read.sh <bug|todo> show <ID>");
    println!("       register-read.sh <bug|todo> list [--status S] [--priority P] [--surface TEXT] [--parent ID]");
    println!("       register-read.sh <bug|todo> report [--since ISO8601]");
    println!("       register-read.sh <bug|todo> count [--status S]");
    println!("       register-read.sh <bug|todo> next-id");
    println!();
    println!("  --file PATH   read this register instead of BUGS_JSON / TODO_JSON");
    std::process::exit(code);
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}", message.as_ref());
    std::process::exit(code);
}

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        usage(0);
    }
    if args.len() < 2 {
        usage(64);
    }
    let kind = args.remove(0);
    let command = args.remove(0);
    if kind != "bug" && kind != "todo" {
        usage(64);
    }
    let mut id = None;
    let mut status = String::new();
    let mut priority = String::new();
    let mut surface = String::new();
    let mut parent = String::new();
    let mut since = String::new();
    let mut file = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--status" => {
                index += 1;
                status = args.get(index).cloned().unwrap_or_else(|| usage(64));
            }
            "--priority" => {
                index += 1;
                priority = args.get(index).cloned().unwrap_or_else(|| usage(64));
            }
            "--surface" => {
                index += 1;
                surface = args.get(index).cloned().unwrap_or_else(|| usage(64));
            }
            "--parent" => {
                index += 1;
                parent = args.get(index).cloned().unwrap_or_else(|| usage(64));
            }
            "--since" => {
                index += 1;
                since = args.get(index).cloned().unwrap_or_else(|| usage(64));
            }
            "--file" => {
                index += 1;
                file = args.get(index).map(PathBuf::from);
                if file.is_none() {
                    usage(64);
                }
            }
            "-h" | "--help" => usage(0),
            value if value.starts_with('-') => usage(64),
            value => {
                if id.is_some() {
                    usage(64);
                }
                id = Some(value.to_string());
            }
        }
        index += 1;
    }
    if command == "show" && id.is_none() {
        usage(64);
    }
    let path = file.unwrap_or_else(|| {
        let variable = if kind == "bug" {
            "BUGS_JSON"
        } else {
            "TODO_JSON"
        };
        env::var(variable).map(PathBuf::from).unwrap_or_else(|_| {
            PathBuf::from(if kind == "bug" {
                "BUGS.json"
            } else {
                "TODO.json"
            })
        })
    });
    if !path.is_file() {
        die(
            format!("register-read.sh: register not found: {}", path.display()),
            66,
        );
    }
    if Command::new("rjq").arg("--version").output().is_err() {
        die(
            "register: rjq is required (it reads and writes the JSON registers); install rjq and re-run",
            69,
        );
    }
    let root: Value = serde_json::from_str(
        &fs::read_to_string(&path).unwrap_or_else(|error| die(error.to_string(), 66)),
    )
    .unwrap_or_else(|error| die(error.to_string(), 66));
    let key = if kind == "bug" { "bugs" } else { "tasks" };
    let items = root.get(key).and_then(Value::as_array).unwrap_or_else(|| {
        die(
            format!(
                "register-read.sh: register has no .{key} array: {}",
                path.display()
            ),
            66,
        )
    });
    match command.as_str() {
        "show" => {
            let wanted = id.as_deref().unwrap();
            let item = items
                .iter()
                .find(|item| item.get("id").and_then(Value::as_str) == Some(wanted))
                .unwrap_or_else(|| {
                    eprintln!(
                        "register-read.sh: no {kind} entry with id {wanted} in {}",
                        path.display()
                    );
                    std::process::exit(1);
                });
            println!("{}", serde_json::to_string_pretty(item).unwrap());
        }
        "list" => {
            let expression = r#"
            .[env.__k] as $items | $items .[]
  | select($status == "" or (.status // "") == $status)
  | select($priority == "" or (.priority // "") == $priority)
  | select($parent == "" or (.parent // "") == $parent)
  | select($surface == "" or ((.surfaces // []) | join(",") | contains($surface)))
            | [.id, (.status // "-"), (.priority // "-"), (.severity // "-"), .title]
            | @tsv"#;
            let output = Command::new("rjq")
                .arg("-r")
                .args(["--arg", "status", &status, "--arg", "priority", &priority])
                .args(["--arg", "surface", &surface, "--arg", "parent", &parent])
                .args(["--arg", "since", &since, "--arg", "key", key])
                .env("__k", key)
                .arg(expression)
                .arg(&path)
                .output()
                .unwrap_or_else(|error| die(error.to_string(), 69));
            io::stdout().write_all(&output.stdout).unwrap();
            io::stderr().write_all(&output.stderr).unwrap();
            std::process::exit(output.status.code().unwrap_or(1));
        }
        "count" => println!(
            "{}",
            items
                .iter()
                .filter(|item| matches_filter(item, &status, &priority, &surface, &parent))
                .count()
        ),
        "next-id" => {
            let prefix = if kind == "bug" { 'B' } else { 'T' };
            let max = items
                .iter()
                .filter_map(|item| item.get("id").and_then(Value::as_str))
                .filter_map(|value| {
                    value
                        .strip_prefix(prefix)
                        .filter(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
                        .and_then(|rest| rest.parse::<u64>().ok())
                })
                .max()
                .unwrap_or(0);
            println!("{}", max + 1);
        }
        "report" => {
            let live = items.iter().filter(|item| {
                matches!(
                    scalar(item, "status", "").as_str(),
                    "open" | "reported" | "confirmed" | "blocked" | "partly"
                )
            });
            let live: Vec<_> = live
                .filter(|item| {
                    since.is_empty()
                        || scalar(item, "updated_at", &scalar(item, "created_at", "")) >= since
                })
                .collect();
            println!("{} open of {} total\n", live.len(), items.len());
            for item in live {
                let status = scalar(item, "severity", &scalar(item, "status", "-"));
                println!(
                    "{}  {}/{}  {}",
                    scalar(item, "id", ""),
                    scalar(item, "priority", "-"),
                    status,
                    scalar(item, "title", "")
                );
                if let Some(values) = item.get("surfaces").and_then(Value::as_array) {
                    if !values.is_empty() {
                        println!(
                            "      surfaces: {}",
                            values
                                .iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                }
            }
        }
        _ => usage(64),
    }
}

fn scalar(item: &Value, key: &str, fallback: &str) -> String {
    item.get(key)
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn matches_filter(item: &Value, status: &str, priority: &str, surface: &str, parent: &str) -> bool {
    (status.is_empty() || scalar(item, "status", "") == status)
        && (priority.is_empty() || scalar(item, "priority", "") == priority)
        && (parent.is_empty() || scalar(item, "parent", "") == parent)
        && (surface.is_empty()
            || item
                .get("surfaces")
                .and_then(Value::as_array)
                .is_some_and(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|value| value.contains(surface))
                }))
}

#[cfg(test)]
mod tests {
    use super::matches_filter;
    use serde_json::json;

    #[test]
    fn absent_filters_are_unfiltered_and_surface_is_array_aware() {
        let item = json!({"status":"open", "surfaces":["planning/scripts/thing.sh"]});
        assert!(matches_filter(&item, "", "", "", ""));
        assert!(matches_filter(&item, "open", "", "thing.sh", ""));
        assert!(!matches_filter(&item, "open", "", "missing", ""));
    }
}
