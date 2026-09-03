// MODE: DEV
// PACKAGE: PROD
use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::path::PathBuf;

fn usage(code: i32) -> ! {
    println!("Usage: register-command.sh [--plan-dir] <plan-directory> <key> <command> <when>");
    println!("       register-command.sh [--plan-dir] <plan-directory> --remove <key>");
    println!("       register-command.sh [--plan-dir] <plan-directory> --list");
    println!("       register-command.sh --help");
    std::process::exit(code);
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}", message.as_ref());
    std::process::exit(code);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        usage(0);
    }
    let mut plan = None;
    let mut mode = "add";
    let mut positionals = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--plan-dir" => {
                index += 1;
                plan = args.get(index).map(PathBuf::from);
            }
            value if value.starts_with("--plan-dir=") => plan = Some(PathBuf::from(&value[11..])),
            "--list" => mode = "list",
            "--remove" => mode = "remove",
            "--" => {
                positionals.extend(args[index + 1..].iter().cloned());
                break;
            }
            value if value.starts_with('-') => {
                eprintln!("register-command.sh: unknown option: {value}");
                usage(64);
            }
            value => positionals.push(value.to_string()),
        }
        index += 1;
    }
    if plan.is_none() && !positionals.is_empty() {
        plan = Some(PathBuf::from(positionals.remove(0)));
    }
    let plan = plan.unwrap_or_else(|| usage(64));
    if !plan.is_dir() {
        die(format!("directory does not exist: {}", plan.display()), 66);
    }
    if std::process::Command::new("rjq")
        .arg("--version")
        .output()
        .is_err()
    {
        die("rjq is required by register-command.sh; install rjq (macOS: brew install rjq, Debian: apt-get install rjq)", 69);
    }
    let commands_file = plan.join("commands.json");
    if !commands_file.is_file() {
        die(
            format!(
                "No {}; create the plan with create-plan.sh to seed an empty registry",
                commands_file.display()
            ),
            64,
        );
    }
    let mut root: Value = serde_json::from_str(
        &fs::read_to_string(&commands_file).unwrap_or_else(|error| die(error.to_string(), 66)),
    )
    .unwrap_or_else(|error| die(error.to_string(), 66));
    let object = root
        .as_object_mut()
        .unwrap_or_else(|| die("commands.json must contain an object", 64));
    match mode {
        "list" => {
            if !positionals.is_empty() {
                usage(64);
            }
            for (key, value) in object.iter() {
                println!(
                    "{}\t{}\t{}",
                    key,
                    value.get("cmd").and_then(Value::as_str).unwrap_or(""),
                    value.get("when").and_then(Value::as_str).unwrap_or("")
                );
            }
        }
        "remove" => {
            if positionals.len() != 1 {
                die("--remove requires exactly one <key>", 64);
            }
            let key = &positionals[0];
            object.remove(key);
            write_json(&commands_file, &root);
            println!("Removed command key {key} from {}", commands_file.display());
        }
        _ => {
            if positionals.len() != 3 {
                usage(64);
            }
            let key = &positionals[0];
            let command = &positionals[1];
            let when = &positionals[2];
            if !is_key(key) {
                die("Command key must be kebab-case (e.g. cache-flush)", 64);
            }
            if command.trim().is_empty() {
                die("Command must not be empty", 64);
            }
            if when.trim().is_empty() {
                die("A command must be registered with a 'when' explanation (when is it appropriate to run it?)", 64);
            }
            let mut entry = Map::new();
            entry.insert("cmd".into(), Value::String(command.clone()));
            entry.insert("when".into(), Value::String(when.clone()));
            object.insert(key.clone(), Value::Object(entry));
            write_json(&commands_file, &root);
            println!("Registered {key} = {command} ({when})");
        }
    }
}

fn is_key(value: &str) -> bool {
    !value.is_empty()
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn write_json(path: &PathBuf, value: &Value) {
    let text =
        serde_json::to_string_pretty(value).unwrap_or_else(|error| die(error.to_string(), 66));
    let temporary = path.with_file_name(format!(".commands.json.tmp.{}", std::process::id()));
    fs::write(&temporary, format!("{text}\n")).unwrap_or_else(|error| die(error.to_string(), 66));
    fs::rename(temporary, path).unwrap_or_else(|error| die(error.to_string(), 66));
}

#[cfg(test)]
mod tests {
    use super::is_key;
    #[test]
    fn key_contract_is_kebab_case() {
        assert!(is_key("cache-flush-2"));
        assert!(!is_key("Cache"));
        assert!(!is_key("cache_flush"));
    }
}
