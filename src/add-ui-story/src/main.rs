// MODE: DEV
// PACKAGE: PROD
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
        .unwrap_or_else(|| "add-ui-story".into())
}
fn usage(code: i32) -> ! {
    let n = name();
    println!(
        "Usage: {n} [--plan-dir] <plan-directory> --id <US-NN> --persona <text> --actions <text>"
    );
    println!("           --interaction <text> --expected <text> --work-units <WNN[,WNN...]>");
    println!("       {n} [--plan-dir] <plan-directory> <US-NN> <persona-or-precondition> <browser-actions> <interaction-evidence> <expected-result> <WNN[,WNN...]>");
    println!("       {n} --help");
    println!();
    println!("The positional form is deprecated; it is kept working for existing callers.");
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
fn valid_id(value: &str) -> bool {
    value.len() >= 5 && value.starts_with("US-") && value[3..].bytes().all(|b| b.is_ascii_digit())
}
fn valid_units(value: &str) -> bool {
    let parts: Vec<_> = value.split(',').collect();
    !parts.is_empty()
        && parts.iter().all(|part| {
            let part = part.trim();
            part.len() >= 3
                && part.starts_with('W')
                && part[1..].bytes().all(|b| b.is_ascii_digit())
        })
}
fn cache_text(id: &str) -> String {
    format!("# Browser run cache: {id}\n\n## Starting state\n\n- URL, persona, viewport/device, and visible initial condition: not yet configured\n\n## Buffered interaction sequence\n\n| Order | Direct UI input | Target / value | Expected readiness signal |\n|---|---|---|---|\n| 1 | one direct user interaction (click, tap, type, keyboard, press, swipe, pinch, drag, select) | the target element and value | the observable readiness signal |\n\n## Waits and readiness\n\n| After order | Wait or condition | Maximum wait | Observed result |\n|---|---|---|---|\n| 1 | the readiness condition | the maximum wait | the actual elapsed wait |\n\n## Run result\n\n- Status: `💤 untested`\n- Evidence: not yet configured (run configure-ui-story-cache.sh to set the sequence, then execute)\n- Cache validity: not yet configured\n")
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut plan_option = None;
    let mut id = String::new();
    let mut persona = String::new();
    let mut actions = String::new();
    let mut interaction = String::new();
    let mut expected = String::new();
    let mut units = String::new();
    let mut positional = Vec::new();
    let mut flagged = false;
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
            "--id" | "--persona" | "--actions" | "--interaction" | "--expected"
            | "--work-units" => {
                flagged = true;
                let flag = args[i].as_str();
                i += 1;
                let value = args.get(i).cloned().unwrap_or_else(|| usage(64));
                match flag {
                    "--id" => id = value,
                    "--persona" => persona = value,
                    "--actions" => actions = value,
                    "--interaction" => interaction = value,
                    "--expected" => expected = value,
                    _ => units = value,
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
    let plan = plan_option
        .clone()
        .or_else(|| positional.first().cloned())
        .unwrap_or_else(|| usage(64));
    if flagged {
        if positional.len() != usize::from(plan_option.is_none()) {
            usage(64)
        }
        if [
            id.as_str(),
            persona.as_str(),
            actions.as_str(),
            interaction.as_str(),
            expected.as_str(),
            units.as_str(),
        ]
        .iter()
        .any(|v| v.is_empty())
        {
            usage(64)
        }
    } else {
        if positional.len() != 7 {
            usage(64)
        }
        id = positional[1].clone();
        persona = positional[2].clone();
        actions = positional[3].clone();
        interaction = positional[4].clone();
        expected = positional[5].clone();
        units = positional[6].clone();
    }
    let plan = PathBuf::from(plan);
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 64)
    }
    if !valid_id(&id) {
        die("Story ID must use US-01", 64)
    }
    if !valid_units(&units) {
        die("Work units must be comma-separated IDs such as W01,W02", 64)
    }
    for (label, value) in [
        ("persona", &persona),
        ("actions", &actions),
        ("interaction", &interaction),
        ("expected", &expected),
    ] {
        safe(label, value);
    }
    let lowered = format!("{actions} {interaction}").to_ascii_lowercase();
    if ![
        "click", "tap", "type", "keyboard", "press", "swipe", "pinch", "drag", "select",
    ]
    .iter()
    .any(|w| lowered.contains(w))
    {
        die("A UI story must name a direct user interaction (accepted verbs: click, tap, type, keyboard, press, swipe, pinch, drag, select)",64)
    }
    let stories = plan.join("ui-user-stories.md");
    if !stories.is_file() {
        eprintln!(
            "{}: UI story artifact not found; run create-ui-validation.sh first",
            name()
        );
        std::process::exit(66)
    }
    let text = fs::read_to_string(&stories).unwrap_or_else(|e| die(e.to_string(), 64));
    if text.lines().any(|line| {
        line.starts_with('|') && line.split('|').nth(1).map(str::trim) == Some(id.as_str())
    }) {
        eprintln!("{}: Story ID already exists: {id}", name());
        std::process::exit(73)
    }
    let row = format!("| {id} | {persona} | {actions} | {interaction} | {expected} | 💤 untested | — | {units} | `ui-story-runs/{id}.md` |\n");
    let mut output = String::new();
    let mut inserted = false;
    for line in text.lines() {
        output.push_str(line);
        output.push('\n');
        if line.starts_with("|---") && !inserted {
            output.push_str(&row);
            inserted = true;
        }
    }
    if !inserted {
        die("UI story table header not found", 64)
    }
    let temporary = stories.with_extension(format!("md.tmp.{}", std::process::id()));
    fs::write(&temporary, output).unwrap_or_else(|e| die(e.to_string(), 64));
    fs::rename(&temporary, &stories).unwrap_or_else(|e| {
        let _ = fs::remove_file(&temporary);
        die(e.to_string(), 64)
    });
    let cache = plan.join("ui-story-runs").join(format!("{id}.md"));
    fs::create_dir_all(cache.parent().unwrap()).unwrap_or_else(|e| die(e.to_string(), 64));
    fs::write(&cache, cache_text(&id)).unwrap_or_else(|e| die(e.to_string(), 64));
    println!("Added {id}");
}
