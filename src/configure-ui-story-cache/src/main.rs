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
        .unwrap_or_else(|| "configure-ui-story-cache".into())
}
fn usage(code: i32) -> ! {
    let n = name();
    println!("Usage: {n} [--plan-dir] <plan-directory> --id <US-NN> --starting-state <text>");
    println!("           --input <direct UI input> --target <text> --readiness <text>");
    println!("           --max-wait <text>");
    println!("       {n} [--plan-dir] <plan-directory> <US-NN> <starting-state> <direct-ui-input> <target-or-value> <readiness-signal> <maximum-wait>");
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
    if value.contains('\n') || value.contains('\r') {
        die(format!("{label} must be one line"), 64)
    }
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut plan_option = None;
    let mut id = String::new();
    let mut starting = String::new();
    let mut input = String::new();
    let mut target = String::new();
    let mut readiness = String::new();
    let mut wait = String::new();
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
                plan_option = Some(value["--plan-dir=".len()..].to_string());
            }
            "--id" | "--starting-state" | "--input" | "--target" | "--readiness" | "--max-wait" => {
                flagged = true;
                let flag = args[i].as_str();
                i += 1;
                let value = args.get(i).cloned().unwrap_or_else(|| usage(64));
                match flag {
                    "--id" => id = value,
                    "--starting-state" => starting = value,
                    "--input" => input = value,
                    "--target" => target = value,
                    "--readiness" => readiness = value,
                    _ => wait = value,
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
    if flagged {
        if positional.len() != usize::from(plan_option.is_none()) {
            usage(64)
        }
        if [
            id.as_str(),
            starting.as_str(),
            input.as_str(),
            target.as_str(),
            readiness.as_str(),
            wait.as_str(),
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
        starting = positional[2].clone();
        input = positional[3].clone();
        target = positional[4].clone();
        readiness = positional[5].clone();
        wait = positional[6].clone();
    }
    let plan = PathBuf::from(
        plan_option
            .or_else(|| positional.first().cloned())
            .unwrap_or_else(|| usage(64)),
    );
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 64)
    }
    if id.len() < 5 || !id.starts_with("US-") || !id[3..].bytes().all(|b| b.is_ascii_digit()) {
        die("Story ID must use US-01", 64)
    }
    for (label, value) in [
        ("starting_state", &starting),
        ("direct_input", &input),
        ("target", &target),
        ("readiness", &readiness),
        ("maximum_wait", &wait),
    ] {
        safe(label, value);
    }
    let lowered = input.to_ascii_lowercase();
    if ![
        "click", "tap", "type", "keyboard", "press", "swipe", "pinch", "drag", "select",
    ]
    .iter()
    .any(|word| lowered.contains(word))
    {
        die("Direct UI input must name a real user interaction", 64)
    }
    let cache = plan.join("ui-story-runs").join(format!("{id}.md"));
    if !cache.is_file() {
        eprintln!(
            "{}: Browser run cache not found: {}",
            name(),
            cache.display()
        );
        std::process::exit(66)
    }
    let output = format!("# Browser run cache: {id}\n\n## Starting state\n\n- {starting}\n\n## Buffered interaction sequence\n\n| Order | Direct UI input | Target / value | Expected readiness signal |\n|---|---|---|---|\n| 1 | {input} | {target} | {readiness} |\n\n## Waits and readiness\n\n| After order | Wait or condition | Maximum wait | Observed result |\n|---|---|---|---|\n| 1 | {readiness} | {wait} | Not run yet |\n\n## Run result\n\n- Status: `💤 untested`\n- Evidence: Pending browser run.\n- Cache validity: Created for the current planned interaction sequence.\n");
    let temporary = cache.parent().unwrap().join(format!(
        ".{}.tmp.{}",
        cache.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    fs::write(&temporary, output).unwrap_or_else(|error| die(error.to_string(), 66));
    fs::rename(&temporary, &cache).unwrap_or_else(|error| {
        let _ = fs::remove_file(&temporary);
        die(error.to_string(), 66)
    });
    println!("Configured {}", cache.display());
}
