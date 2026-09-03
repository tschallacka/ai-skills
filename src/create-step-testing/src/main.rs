// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn usage(code: i32) -> ! {
    let name = env::args()
        .next()
        .unwrap_or_else(|| "create-step-testing".into());
    let name = Path::new(&name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("create-step-testing");
    println!("Usage: {name} <goal-directory> <step-name> <verification-instructions>");
    println!("              [--browser TEXT] [--backend TEXT] [--manual TEXT] [--overwrite]");
    println!("       {name} --help");
    println!();
    println!("  <verification-instructions>  automated tests, section 2");
    println!("  --browser TEXT               browser verification, section 3");
    println!("  --backend TEXT               backend verification, section 4");
    println!("  --manual TEXT                manual verification, section 5");
    println!();
    println!(r#"Paragraphs split on "\\n" escapes or real newlines; each gets its own label."#);
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("create-step-testing: {}", message.as_ref());
    std::process::exit(code)
}

fn check_text(label: &str, text: &str) {
    if text.trim().is_empty() {
        die(format!("{label} instructions must not be empty"), 1)
    }
    if text.contains('|') {
        die(
            format!("{label} instructions must not contain a Markdown table separator (|)"),
            1,
        )
    }
    if text.contains('§') {
        die(
            format!("{label} instructions must not contain the reserved paragraph marker §"),
            1,
        )
    }
}

fn paragraphs(text: &str, section: u32, label: &str) -> String {
    let expanded = text.replace(r"\n", "\n");
    let mut result = Vec::new();
    for paragraph in expanded.split('\n') {
        let paragraph = paragraph.trim();
        if !paragraph.is_empty() {
            result.push(paragraph.to_string());
        }
    }
    if result.is_empty() {
        die(format!("{label} instructions are empty after splitting"), 1)
    }
    result
        .into_iter()
        .enumerate()
        .map(|(i, text)| format!("§ {section}.{}\n{text}", i + 1))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut overwrite = false;
    let mut browser = String::new();
    let mut backend = String::new();
    let mut manual = String::new();
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => usage(0),
            "--overwrite" => {}
            "--browser" | "--backend" | "--manual" => {
                let flag = args[i].as_str();
                i += 1;
                let value = args.get(i).unwrap_or_else(|| usage(64)).clone();
                match flag {
                    "--browser" => browser = value,
                    "--backend" => backend = value,
                    _ => manual = value,
                }
            }
            "--" => positional.extend(args.iter().skip(i + 1).cloned()),
            value if value.starts_with('-') => {
                eprintln!("create-step-testing: unknown option: {value}");
                usage(64)
            }
            value => positional.push(value.to_string()),
        }
        if args.get(i).is_some_and(|arg| arg == "--overwrite") {
            overwrite = true;
        }
        if args.get(i).is_some_and(|arg| arg == "--") {
            break;
        }
        i += 1;
    }
    if positional.len() != 3 {
        usage(64)
    }
    let goal = PathBuf::from(&positional[0]);
    let step = &positional[1];
    let instructions = &positional[2];
    if !goal.is_dir() {
        die(format!("Plan directory not found: {}", goal.display()), 66)
    }
    let bytes = step.as_bytes();
    let valid_step = bytes.len() >= 9
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b'-'
        && &bytes[3..8] == b"step-"
        && bytes[8..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-');
    if !valid_step {
        die("Step name must use 01-step-kebab-case", 1)
    }
    let step_file = goal.join("steps").join(format!("{step}.md"));
    if !step_file.is_file() {
        eprintln!(
            "create-step-testing: Implementation step not found: {}",
            step_file.display()
        );
        std::process::exit(66)
    }
    let testing = goal.join("steps").join(format!("{step}-testing.md"));
    if testing.exists() && !overwrite {
        eprintln!("create-step-testing: Testing companion already exists: {} (pass --overwrite to replace it)", testing.display());
        std::process::exit(73)
    }
    check_text("Verification", instructions);
    if !browser.is_empty() {
        check_text("Browser verification", &browser);
    }
    if !backend.is_empty() {
        check_text("Backend verification", &backend);
    }
    if !manual.is_empty() {
        check_text("Manual verification", &manual);
    }
    let mut output = format!(
        "# Verification: {step}\n\n## Automated tests\n\n{}",
        paragraphs(instructions, 2, "Verification")
    );
    for (text, section, heading, label) in [
        (&browser, 3, "Browser verification", "Browser verification"),
        (&backend, 4, "Backend verification", "Backend verification"),
        (&manual, 5, "Manual verification", "Manual verification"),
    ] {
        if !text.is_empty() {
            output.push_str(&format!(
                "\n\n## {heading}\n\n{}",
                paragraphs(text, section, label)
            ));
        }
    }
    fs::create_dir_all(testing.parent().unwrap())
        .unwrap_or_else(|error| die(error.to_string(), 66));
    let temporary = testing.with_extension(format!("md.tmp.{}", std::process::id()));
    fs::write(&temporary, output).unwrap_or_else(|error| die(error.to_string(), 66));
    fs::rename(&temporary, &testing).unwrap_or_else(|error| {
        let _ = fs::remove_file(&temporary);
        die(error.to_string(), 66)
    });
    println!("Created {}", testing.display());
}
