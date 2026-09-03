// MODE: DEV
// PACKAGE: PROD
use plan_crypt::sha256::{hex, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;

fn usage(code: i32) -> ! {
    println!("Usage: generate-reviewer.sh [<skill-directory>] [<output-file>]");
    println!("       generate-reviewer.sh --help");
    std::process::exit(code);
}
fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}", message.as_ref());
    std::process::exit(code);
}
fn digest(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex(&h.finish())
}

fn section(source: &str, name: &str) -> Option<String> {
    let start = format!("<!-- REVIEWER_SECTION:START {name} -->");
    let end = format!("<!-- REVIEWER_SECTION:END {name} -->");
    let mut inside = false;
    let mut found = false;
    let mut lines = Vec::new();
    for line in source.lines() {
        if line == start {
            if inside || found {
                return None;
            }
            inside = true;
            found = true;
            continue;
        }
        if line == end {
            if !inside {
                return None;
            }
            inside = false;
            continue;
        }
        if inside {
            lines.push(line)
        }
    }
    if !found || inside {
        return None;
    }
    Some(lines.join("\n"))
}
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--help" || a == "-h") {
        usage(0)
    }
    if args.len() > 2 || args.iter().any(|a| a.starts_with('-')) {
        die(
            format!(
                "generate-reviewer.sh: unknown option: {}",
                args.iter()
                    .find(|a| a.starts_with('-'))
                    .map(String::as_str)
                    .unwrap_or("")
            ),
            64,
        )
    }
    let skill = PathBuf::from(args.first().cloned().unwrap_or_else(|| "planning".into()));
    let source = skill.join("SKILL.md");
    let output = PathBuf::from(
        args.get(1)
            .cloned()
            .unwrap_or_else(|| skill.join("REVIEWER.md").display().to_string()),
    );
    if !source.is_file() {
        die(format!("source skill not found: {}", source.display()), 66)
    }
    let bytes = fs::read(&source).unwrap_or_else(|e| die(e.to_string(), 66));
    let hash = digest(&bytes);
    let mut result = String::new();
    result.push_str("<!-- MODE: PROD -->\n# Reviewer contract\n\n");
    result.push_str(&format!("> Generated from `SKILL.md` by `scripts/generate-reviewer.sh`.\n> Reviewer profile contract: `1.4.2`\n> Source SHA-256: `{hash}`\n\nThis file is a review-scoped projection of the tagged `SKILL.md`; the tagged skill remains authoritative.\n\n## Generated sections\n\n- `mandatory-review`\n- `bounded-context`\n\n"));
    let source_text =
        String::from_utf8(bytes).unwrap_or_else(|_| die("source skill is not UTF-8", 66));
    for name in ["mandatory-review", "bounded-context"] {
        let body = section(&source_text, name)
            .unwrap_or_else(|| die(format!("invalid or missing reviewer section: {name}"), 65));
        if body.trim().is_empty() {
            die(format!("empty reviewer section: {name}"), 65)
        }
        result.push_str(&body);
        result.push_str("\n\n");
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| die(e.to_string(), 70));
    }
    fs::write(&output, result).unwrap_or_else(|e| die(e.to_string(), 70));
    println!(
        "generated {} from {} sha256={}",
        output.display(),
        source.display(),
        hash
    );
}
