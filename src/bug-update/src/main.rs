// MODE: DEV
// PACKAGE: PROD
use planning_register::{findings, now, read, require_rjq, sort, text, write};
use serde_json::Value;
use std::env;
use std::path::PathBuf;

fn usage(code: i32) -> ! {
    println!("Usage: bug-update.sh <id> [--status S] [--fix F] [--verification V]");
    println!("       [--reason R] [--priority P] [--mechanism M] [--append-note N]");
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
        usage(64)
    };
    let path = env::var("BUGS_JSON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("BUGS.json"));
    if !path.is_file() {
        die(
            format!("bug-update.sh: register not found: {}", path.display()),
            66,
        )
    }
    require_rjq().unwrap_or_else(|e| die(format!("bug-update.sh: {e}"), 69));
    let mut status = None;
    let mut fix = None;
    let mut ver = None;
    let mut reason = None;
    let mut priority = None;
    let mut mechanism = None;
    let mut note = None;
    let mut i = 1;
    while i < args.len() {
        let key = args[i].as_str();
        i += 1;
        let v = args.get(i).cloned().unwrap_or_default();
        match key {
            "--status" => status = Some(v),
            "--fix" => fix = Some(v),
            "--verification" => ver = Some(v),
            "--reason" => reason = Some(v),
            "--priority" => priority = Some(v),
            "--mechanism" => mechanism = Some(v),
            "--append-note" => note = Some(v),
            _ => die(format!("bug-update.sh: unknown argument: {key}"), 64),
        }
        i += 1;
    }
    if status.is_none()
        && fix.is_none()
        && ver.is_none()
        && reason.is_none()
        && priority.is_none()
        && mechanism.is_none()
        && note.is_none()
    {
        die("bug-update.sh: nothing to set", 64)
    }
    let mut root = read(&path).unwrap_or_else(|e| die(e, 66));
    let item = root
        .get_mut("bugs")
        .and_then(Value::as_array_mut)
        .and_then(|xs| {
            xs.iter_mut()
                .find(|x| x.get("id").and_then(Value::as_str) == Some(&id))
        })
        .unwrap_or_else(|| die(format!("bug-update.sh: no defect {id}"), 66));
    let object = item.as_object_mut().unwrap();
    if let Some(s) = status.as_deref() {
        match s {
            "fixed" => {
                if fix.as_deref().unwrap_or("").is_empty() {
                    die("bug-update.sh: --status fixed requires --fix", 64)
                }
                if ver.as_deref().unwrap_or("").is_empty() {
                    die("bug-update.sh: --status fixed requires --verification", 64)
                }
            }
            "wont-fix" | "not-a-defect" | "obsolete"
                if reason.as_deref().unwrap_or("").is_empty() =>
            {
                die(format!("bug-update.sh: --status {s} requires --reason"), 64)
            }
            _ => {}
        }
    }
    object.insert("updated_at".into(), Value::String(now()));
    if let Some(v) = status {
        object.insert("status".into(), Value::String(v));
    }
    if let Some(v) = priority {
        object.insert("priority".into(), Value::String(v));
    }
    if let Some(v) = fix {
        object.insert("fix".into(), Value::String(v));
    }
    if let Some(v) = ver {
        object.insert("verification".into(), Value::String(v));
    }
    if let Some(v) = mechanism {
        object.insert("mechanism".into(), Value::String(v));
    }
    for v in [reason, note].into_iter().flatten() {
        let old = text(object, "notes");
        let joined = if old.is_empty() {
            v
        } else {
            format!("{old} {v}")
        };
        object.insert("notes".into(), Value::String(joined));
    }
    write(&path, &root).unwrap_or_else(|e| die(e, 66));
    let issues = findings(&root, "bugs", true);
    if !issues.is_empty() {
        for issue in issues {
            eprintln!("{issue}")
        }
        die(
            format!(
                "bug-update.sh: {} is not sound; run register-rebuild.sh bug \"{}\" first",
                path.display(),
                path.display()
            ),
            65,
        )
    }
    sort(&mut root, "bugs", true);
    write(&path, &root).unwrap_or_else(|e| die(e, 66));
    println!("Updated {id}");
}
