// MODE: DEV
// PACKAGE: PROD
use planning_register::{findings, now, read, require_rjq, write};
use serde_json::Value;
use std::env;
use std::path::PathBuf;

fn usage(code: i32) -> ! {
    eprintln!("usage: todo-update.sh <id> [--status S] [--priority P] [--note N] [--detail D] [--blocked-on X]");
    std::process::exit(code);
}
fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}", message.as_ref());
    std::process::exit(code);
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--help" || a == "-h") {
        usage(0);
    }
    let id = args.first().cloned().unwrap_or_else(|| usage(64));
    if id.starts_with('-') {
        usage(64);
    }
    let path = env::var("TODO_JSON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("TODO.json"));
    if !path.is_file() {
        die(
            format!("todo-update.sh: register not found: {}", path.display()),
            66,
        );
    }
    require_rjq().unwrap_or_else(|e| die(format!("todo-update.sh: {e}"), 69));
    let mut status = None;
    let mut priority = None;
    let mut note = None;
    let mut detail = None;
    let mut blocked = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--status" => {
                i += 1;
                status = Some(args.get(i).cloned().unwrap_or_default());
            }
            "--priority" => {
                i += 1;
                priority = Some(args.get(i).cloned().unwrap_or_default());
            }
            "--note" => {
                i += 1;
                note = Some(args.get(i).cloned().unwrap_or_default());
            }
            "--detail" => {
                i += 1;
                detail = Some(args.get(i).cloned().unwrap_or_default());
            }
            "--blocked-on" => {
                i += 1;
                blocked = Some(args.get(i).cloned().unwrap_or_default());
            }
            _ => die(format!("todo-update.sh: unknown argument: {}", args[i]), 64),
        }
        i += 1;
    }
    if status.is_none()
        && priority.is_none()
        && note.is_none()
        && detail.is_none()
        && blocked.is_none()
    {
        die("todo-update.sh: nothing to set", 64);
    }
    let mut root = read(&path).unwrap_or_else(|e| die(e, 66));
    let items = root
        .get_mut("tasks")
        .and_then(Value::as_array_mut)
        .unwrap_or_else(|| die("todo-update.sh: register has no .tasks array", 66));
    let item = items
        .iter_mut()
        .find(|v| v.get("id").and_then(Value::as_str) == Some(&id))
        .unwrap_or_else(|| die(format!("todo-update.sh: no task {id}"), 66));
    let object = item.as_object_mut().unwrap();
    object.insert("updated_at".into(), Value::String(now()));
    if let Some(v) = status {
        object.insert("status".into(), Value::String(v));
    }
    if let Some(v) = priority {
        object.insert("priority".into(), Value::String(v));
    }
    if let Some(v) = note {
        object.insert("note".into(), Value::String(v));
    }
    if let Some(v) = detail {
        object.insert("detail".into(), Value::String(v));
    }
    if let Some(v) = blocked {
        object.insert("blocked_on".into(), Value::String(v));
    }
    let issues = findings(&root, "tasks", false);
    if !issues.is_empty() {
        for issue in issues {
            eprintln!("{issue}");
        }
        die("todo-update.sh: update refused; nothing was written", 65);
    }
    write(&path, &root).unwrap_or_else(|e| die(e, 66));
    println!("Updated {id}");
    println!("Updated {id}");
}
