// MODE: DEV
// PACKAGE: PROD
use planning_register::{findings, next_id, now, read, require_rjq, sort, text, write};
use serde_json::{Map, Value};
use std::env;
use std::path::PathBuf;

fn usage(code: i32) -> ! {
    println!("Usage: todo-add.sh --id T45 --title \"text\" [--parent T44] [--priority high]");
    println!("               [--status open] [--blocked-on X] [--detail \"text\"] [--ref path]...");
    println!("       todo-add.sh --help");
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
    let path = env::var("TODO_JSON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("TODO.json"));
    if !path.is_file() {
        die(
            format!("todo-add.sh: register not found: {}", path.display()),
            66,
        );
    }
    require_rjq().unwrap_or_else(|e| die(format!("todo-add.sh: {e}"), 69));
    let mut id = "".to_string();
    let mut title = "".to_string();
    let mut parent: Value = Value::Null;
    let mut priority = "normal".to_string();
    let mut status = "open".to_string();
    let mut blocked = Value::Null;
    let mut detail = Value::Null;
    let mut refs = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--id" => {
                i += 1;
                id = args
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| std::process::exit(64));
            }
            "--title" => {
                i += 1;
                title = args
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| std::process::exit(64));
            }
            "--parent" => {
                i += 1;
                parent = Value::String(args.get(i).cloned().unwrap_or_default());
            }
            "--priority" => {
                i += 1;
                priority = args.get(i).cloned().unwrap_or_default();
            }
            "--status" => {
                i += 1;
                status = args.get(i).cloned().unwrap_or_default();
            }
            "--blocked-on" => {
                i += 1;
                blocked = Value::String(args.get(i).cloned().unwrap_or_default());
            }
            "--detail" => {
                i += 1;
                detail = Value::String(args.get(i).cloned().unwrap_or_default());
            }
            "--ref" => {
                i += 1;
                refs.push(Value::String(args.get(i).cloned().unwrap_or_default()));
            }
            _ => die(format!("todo-add.sh: unknown argument: {}", args[i]), 64),
        }
        i += 1;
    }
    if id.is_empty() || title.is_empty() {
        die("todo-add.sh: --id and --title are required", 64);
    }
    let stamp = now();
    let mut root = read(&path).unwrap_or_else(|e| die(e, 66));
    if root
        .get("tasks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|v| text(v.as_object().unwrap(), "id") == id)
    {
        die("todo-add.sh: duplicate ids", 65);
    }
    let mut entry = Map::new();
    for (k, v) in [
        ("id", Value::String(id.clone())),
        ("title", Value::String(title.clone())),
        ("status", Value::String(status)),
        ("priority", Value::String(priority)),
        ("parent", parent),
        ("detail", detail),
        ("blocked_on", blocked),
        ("refs", Value::Array(refs)),
        ("note", Value::Null),
        ("created_at", Value::String(stamp.clone())),
        ("updated_at", Value::String(stamp)),
    ] {
        entry.insert(k.into(), v);
    }
    root.get_mut("tasks")
        .and_then(Value::as_array_mut)
        .unwrap_or_else(|| die("todo-add.sh: register has no .tasks array", 66))
        .push(Value::Object(entry));
    sort(&mut root, "tasks", false);
    if !findings(&root, "tasks", false).is_empty() {
        die("todo-add.sh: entry refused; nothing was written", 65);
    }
    write(&path, &root).unwrap_or_else(|e| die(e, 66));
    let next = next_id(&root, "tasks", 'T');
    println!("Added {id} (next free: T{})", next - 1);
}
