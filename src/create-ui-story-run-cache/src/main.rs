// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn usage(code: i32) -> ! {
    let name = env::args()
        .next()
        .unwrap_or_else(|| "create-ui-story-run-cache".into());
    let name = Path::new(&name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("create-ui-story-run-cache");
    println!("Usage: {name} [--plan-dir] <plan-directory> <US-NN>");
    println!("       {name} --help");
    std::process::exit(code)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut plan: Option<String> = None;
    let mut story: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => usage(0),
            "--plan-dir" => {
                index += 1;
                plan = args.get(index).cloned().or_else(|| usage(64));
            }
            value if value.starts_with("--plan-dir=") => {
                plan = Some(value["--plan-dir=".len()..].to_string());
            }
            "--" => {
                let remaining = &args[index + 1..];
                if remaining.len() != 2 || plan.is_some() {
                    usage(64)
                }
                plan = Some(remaining[0].clone());
                story = Some(remaining[1].clone());
                break;
            }
            value if value.starts_with('-') => usage(64),
            value => {
                if plan.is_none() {
                    plan = Some(value.to_string());
                } else if story.is_none() {
                    story = Some(value.to_string());
                } else {
                    usage(64)
                }
            }
        }
        index += 1;
    }
    let plan = plan.unwrap_or_else(|| usage(64));
    let story = story.unwrap_or_else(|| usage(64));
    if !Path::new(&plan).is_dir() {
        eprintln!("Plan directory not found: {plan}");
        std::process::exit(66)
    }
    let valid_story = story.len() >= 5
        && story.starts_with("US-")
        && story[3..].bytes().all(|byte| byte.is_ascii_digit());
    if !valid_story {
        eprintln!("Invalid story ID: {story} (use US-01)");
        std::process::exit(64)
    }
    let cache = PathBuf::from(&plan)
        .join("ui-story-runs")
        .join(format!("{story}.md"));
    if cache.exists() {
        eprintln!("Browser run cache already exists: {}", cache.display());
        std::process::exit(73)
    }
    let output = format!(
        "# Browser run cache: {story}\n\n\
## Starting state\n\n\
- URL, persona, viewport/device, and visible initial condition: not yet configured\n\n\
## Buffered interaction sequence\n\n\
| Order | Direct UI input | Target / value | Expected readiness signal |\n\
|---|---|---|---|\n\
| 1 | one direct user interaction (click, tap, type, keyboard, press, swipe, pinch, drag, select) | the target element and value | the observable readiness signal |\n\n\
## Waits and readiness\n\n\
| After order | Wait or condition | Maximum wait | Observed result |\n\
|---|---|---|---|\n\
| 1 | the readiness condition | the maximum wait | the actual elapsed wait |\n\n\
## Run result\n\n\
- Status: `💤 untested`\n\
- Evidence: not yet configured (run configure-ui-story-cache.sh to set the sequence, then execute)\n\
- Cache validity: not yet configured\n"
    );
    let directory = cache.parent().unwrap();
    fs::create_dir_all(directory).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(66)
    });
    let temporary = directory.join(format!(
        ".{}.tmp.{}",
        cache.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    fs::write(&temporary, output).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(66)
    });
    fs::rename(&temporary, &cache).unwrap_or_else(|error| {
        let _ = fs::remove_file(&temporary);
        eprintln!("{error}");
        std::process::exit(66)
    });
    println!("Created {}", cache.display());
}
