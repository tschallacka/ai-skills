// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn command_name() -> String {
    env::args()
        .next()
        .and_then(|p| {
            PathBuf::from(p)
                .file_name()
                .map(|v| v.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "create-ui-validation".into())
}
fn usage(code: i32) -> ! {
    let n = command_name();
    println!("Usage: {n} [--plan-dir] <plan-directory> <browser-target-or-discovery-method>");
    println!("       {n} --help");
    std::process::exit(code)
}
fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}: {}", command_name(), message.as_ref());
    std::process::exit(code)
}
fn replace_ui_field(path: &Path) {
    let text = fs::read_to_string(path).unwrap_or_else(|error| die(error.to_string(), 64));
    let mut count = 0;
    let mut output = String::new();
    for line in text.lines() {
        if line.starts_with("- UI affected:") {
            count += 1;
            output.push_str("- UI affected: yes\n");
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    if count != 1 {
        die("Field was not found exactly once: UI affected", 64)
    }
    fs::write(path, output).unwrap_or_else(|error| die(error.to_string(), 64));
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut positional = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => usage(0),
            "--plan-dir" => {
                index += 1;
                positional.push(args.get(index).cloned().unwrap_or_else(|| usage(64)));
            }
            value if value.starts_with("--plan-dir=") => {
                positional.push(value["--plan-dir=".len()..].to_string())
            }
            "--" => {
                positional.extend(args.iter().skip(index + 1).cloned());
                break;
            }
            value if value.starts_with('-') => {
                eprintln!("{}: unknown option: {value}", command_name());
                usage(64)
            }
            value => positional.push(value.to_string()),
        }
        index += 1;
    }
    if positional.len() != 2 {
        usage(64)
    }
    let plan = PathBuf::from(&positional[0]);
    let target = &positional[1];
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 64)
    }
    if target.is_empty() {
        die("Browser target must not be empty", 64)
    }
    if target.contains('|') {
        die(
            "Browser target must not contain a Markdown table separator (|)",
            64,
        )
    }
    if target.contains('\n') || target.contains('\r') {
        die("Browser target must be one line", 64)
    }
    let description = plan.join("plan-description.md");
    let stories = plan.join("ui-user-stories.md");
    let bugs = plan.join("bugs.md");
    if !description.is_file() {
        eprintln!(
            "{}: Plan description not found: {}",
            command_name(),
            description.display()
        );
        std::process::exit(66)
    }
    if stories.exists() || bugs.exists() {
        eprintln!("{}: UI validation artifacts already exist", command_name());
        std::process::exit(73)
    }
    let description_text =
        fs::read_to_string(&description).unwrap_or_else(|error| die(error.to_string(), 64));
    if description_text
        .lines()
        .any(|line| line == "## UI validation")
    {
        eprintln!(
            "{}: Plan description already has a UI validation section",
            command_name()
        );
        std::process::exit(73)
    }
    let mut inserted = false;
    let mut modified = String::new();
    for line in description_text.lines() {
        if line == "## Adversarial review" && !inserted {
            modified.push_str("## UI validation\n\n- Required: yes\n- Browser target: ");
            modified.push_str(target);
            modified.push_str("\n- Story artifact: `ui-user-stories.md`\n\n");
            inserted = true;
        }
        modified.push_str(line);
        modified.push('\n');
    }
    if !inserted {
        die("Plan description has no Adversarial review section", 64)
    }
    fs::write(&description, modified).unwrap_or_else(|error| die(error.to_string(), 64));
    replace_ui_field(&description);
    let plan_name = plan.file_name().unwrap().to_string_lossy();
    let story_text = format!("# UI user stories: {plan_name}\n\n| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |\n|---|---|---|---|---|---|---|---|---|\n");
    let bug_text = format!("# UI bugs: {plan_name}\n\n| ID | Story | Severity | Reproduction/evidence | Investigation goal | Fix goal | Retest story | Status |\n|---|---|---|---|---|---|---|---|\n");
    fs::write(&stories, story_text).unwrap_or_else(|error| die(error.to_string(), 64));
    fs::write(&bugs, bug_text).unwrap_or_else(|error| die(error.to_string(), 64));
    println!("Created {}", stories.display());
}
