// MODE: DEV
// PACKAGE: PROD
use planning_register::{findings, next_id, now, read, require_rjq, write};
use serde_json::{Map, Value};
use std::env;
use std::path::PathBuf;

fn usage(code: i32) -> ! {
    println!("Usage: bug-add.sh --title \"text\" --reproduce \"cmd\" --observed \"text\"");
    println!("               --expected \"text\" [--severity major] [--priority normal]");
    println!("               [--status reported|confirmed] [--mechanism \"text\"] [--parent B37]");
    println!("               [--found-by \"who\"] [--surfaces f1,f2]");
    println!("       bug-add.sh --help");
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
    let path = env::var("BUGS_JSON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("BUGS.json"));
    if !path.is_file() {
        die(
            format!("bug-add.sh: register not found: {}", path.display()),
            66,
        );
    }
    require_rjq().unwrap_or_else(|e| die(format!("bug-add.sh: {e}"), 69));
    let mut title = "".into();
    let mut reproduce = "".into();
    let mut observed = "".into();
    let mut expected = "".into();
    let mut severity = "major".into();
    let mut priority = "normal".into();
    let mut status = "reported".into();
    let mut mechanism = Value::Null;
    let mut parent = Value::Null;
    let mut found_by = "register writer".into();
    let mut surfaces = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let key = args[i].as_str();
        i += 1;
        let value = || args.get(i).cloned().unwrap_or_default();
        match key {
            "--title" => {
                title = value();
                i += 1
            }
            "--reproduce" => {
                reproduce = value();
                i += 1
            }
            "--observed" => {
                observed = value();
                i += 1
            }
            "--expected" => {
                expected = value();
                i += 1
            }
            "--severity" => {
                severity = value();
                i += 1
            }
            "--priority" => {
                priority = value();
                i += 1
            }
            "--status" => {
                status = value();
                i += 1
            }
            "--mechanism" => {
                mechanism = Value::String(value());
                i += 1
            }
            "--parent" => {
                parent = Value::String(value());
                i += 1
            }
            "--found-by" => {
                found_by = value();
                i += 1
            }
            "--surfaces" => {
                surfaces = value()
                    .split(',')
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                i += 1
            }
            _ => die(format!("bug-add.sh: unknown argument: {key}"), 64),
        }
    }
    if title.is_empty() || reproduce.is_empty() || observed.is_empty() || expected.is_empty() {
        die(
            "bug-add.sh: --title --reproduce --observed --expected are required",
            64,
        );
    }
    let mut root = read(&path).unwrap_or_else(|e| die(e, 66));
    let id = format!("B{}", next_id(&root, "bugs", 'B'));
    let stamp = now();
    let mut entry = Map::new();
    for (k, v) in [
        ("id", Value::String(id.clone())),
        ("title", Value::String(title.clone())),
        ("status", Value::String(status)),
        ("severity", Value::String(severity)),
        ("priority", Value::String(priority)),
        ("parent", parent),
        ("reproduce", Value::String(reproduce)),
        ("observed", Value::String(observed)),
        ("expected", Value::String(expected)),
        ("mechanism", mechanism),
        ("surfaces", Value::Array(surfaces)),
        ("fix", Value::Null),
        ("verification", Value::Null),
        ("found_by", Value::String(found_by)),
        ("notes", Value::Null),
        ("created_at", Value::String(stamp.clone())),
        ("updated_at", Value::String(stamp)),
    ] {
        entry.insert(k.into(), v);
    }
    root.get_mut("bugs")
        .and_then(Value::as_array_mut)
        .unwrap_or_else(|| die("bug-add.sh: register has no .bugs array", 66))
        .push(Value::Object(entry));
    let issues = findings(&root, "bugs", true);
    if !issues.is_empty() {
        for issue in issues {
            eprintln!("{issue}");
        }
        die("bug-add.sh: entry refused; nothing was written", 65);
    }
    write(&path, &root).unwrap_or_else(|e| die(e, 66));
    println!("Filed {id}: {title}");
}
