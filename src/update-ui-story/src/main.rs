// MODE: DEV
// PACKAGE: PROD
use planning_core::git_snapshot;
use std::env;
use std::fs;
use std::path::PathBuf;

fn name() -> String {
    env::args()
        .next()
        .and_then(|p| {
            PathBuf::from(p)
                .file_name()
                .map(|v| v.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "update-ui-story".into())
}
fn usage(code: i32) -> ! {
    let n = name();
    println!("Usage: {n} [--plan-dir] <plan-directory> <US-NN>\n           [--persona <text>] [--actions <text>] [--interaction <text>]\n           [--expected <text>]\n       {n} --help");
    std::process::exit(code)
}
fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}: {}", name(), message.as_ref());
    std::process::exit(code)
}
fn safe(label: &str, value: &str) {
    if value.is_empty() {
        die(format!("{label} must not be empty"), 64)
    }
    if value.contains('|') {
        die(
            format!("{label} must not contain a Markdown table separator (|)"),
            64,
        )
    }
    if value.contains(['\n', '\r']) {
        die(format!("{label} must be one line"), 64)
    }
}
fn cell(line: &str, index: usize) -> String {
    line.split('|').nth(index).unwrap_or("").trim().to_string()
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut plan_option = None;
    let mut persona = None;
    let mut actions = None;
    let mut interaction = None;
    let mut expected = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => usage(0),
            "--plan-dir" => {
                i += 1;
                plan_option = args.get(i).cloned().or_else(|| usage(64));
            }
            value if value.starts_with("--plan-dir=") => {
                plan_option = Some(value["--plan-dir=".len()..].to_string())
            }
            "--persona" | "--actions" | "--interaction" | "--expected" => {
                i += 1;
                let value = args.get(i).cloned().unwrap_or_else(|| usage(64));
                match args[i - 1].as_str() {
                    "--persona" => persona = Some(value),
                    "--actions" => actions = Some(value),
                    "--interaction" => interaction = Some(value),
                    _ => expected = Some(value),
                }
            }
            "--" => {
                positional.extend(args.iter().skip(i + 1).cloned());
                break;
            }
            value if value.starts_with('-') => {
                eprintln!("{}: unknown option: {value}", name());
                usage(64)
            }
            value => positional.push(value.to_string()),
        }
        i += 1;
    }
    let (plan_value, id_value) = match (plan_option, positional.as_slice()) {
        (Some(plan), [id]) => (plan, id.clone()),
        (None, [plan, id]) => (plan.clone(), id.clone()),
        _ => usage(64),
    };
    let plan = PathBuf::from(plan_value);
    let id = &id_value;
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 64)
    }
    if id.len() < 5 || !id.starts_with("US-") || !id[3..].bytes().all(|b| b.is_ascii_digit()) {
        die("Story ID must use US-01", 64)
    }
    if [
        persona.as_ref(),
        actions.as_ref(),
        interaction.as_ref(),
        expected.as_ref(),
    ]
    .iter()
    .all(Option::is_none)
    {
        usage(64)
    }
    for (label, value) in [
        ("persona", &persona),
        ("actions", &actions),
        ("interaction", &interaction),
        ("expected", &expected),
    ] {
        if let Some(value) = value {
            safe(label, value);
        }
    }
    let stories = plan.join("ui-user-stories.md");
    if !stories.is_file() {
        die(
            "UI story artifact not found; run create-ui-validation.sh first",
            66,
        )
    }
    let text = fs::read_to_string(&stories).unwrap_or_else(|e| die(e.to_string(), 64));
    let row = text
        .lines()
        .find(|line| line.starts_with('|') && cell(line, 1) == *id)
        .unwrap_or("");
    if row.is_empty() {
        die(format!("Story ID not found: {id}"), 64)
    }
    let persona = persona.unwrap_or_else(|| cell(row, 2));
    let actions = actions.unwrap_or_else(|| cell(row, 3));
    let interaction = interaction.unwrap_or_else(|| cell(row, 4));
    let expected = expected.unwrap_or_else(|| cell(row, 5));
    let lowered = format!("{actions} {interaction}").to_ascii_lowercase();
    if ![
        "click", "tap", "type", "keyboard", "press", "swipe", "pinch", "drag", "select",
    ]
    .iter()
    .any(|w| lowered.contains(w))
    {
        die("A UI story must name a direct user interaction (accepted verbs: click, tap, type, keyboard, press, swipe, pinch, drag, select)",64)
    }
    git_snapshot(&plan);
    let mut output = String::new();
    let mut touched = false;
    for line in text.lines() {
        if line.starts_with('|') && cell(line, 1) == *id {
            let mut fields: Vec<String> = line.split('|').map(str::to_string).collect();
            if fields.len() < 7 {
                continue;
            }
            fields[2] = format!(" {persona} ");
            fields[3] = format!(" {actions} ");
            fields[4] = format!(" {interaction} ");
            fields[5] = format!(" {expected} ");
            output.push_str(&fields.join("|"));
            output.push('\n');
            touched = true
        } else {
            output.push_str(line);
            output.push('\n')
        }
    }
    if !touched {
        die("UI story row not found", 64)
    }
    let temporary = stories.with_extension(format!("md.tmp.{}", std::process::id()));
    fs::write(&temporary, output).unwrap_or_else(|e| die(e.to_string(), 66));
    fs::rename(&temporary, &stories).unwrap_or_else(|e| {
        let _ = fs::remove_file(&temporary);
        die(e.to_string(), 66)
    });
    println!("Updated {id}");
}
