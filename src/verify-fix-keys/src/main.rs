// MODE: DEV
// PACKAGE: PROD
use plan_crypt::sha256::{hex, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;

const COMMAND: &str = "verify-fix-keys.sh";

fn usage(code: i32) -> ! {
    println!("Usage: {COMMAND} [--plan-dir] <plan-directory> [--claimed-by <id>]");
    println!("       {COMMAND} --help");
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

fn key(secret: &str, message: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(secret.as_bytes());
    digest.update(message.as_bytes());
    hex(&digest.finish())
}

fn valid_id(value: &str, prefix: &str) -> bool {
    value
        .strip_prefix(prefix)
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

fn gated_pairs(review: &str) -> Vec<(String, String)> {
    let mut in_findings = false;
    let mut pairs = Vec::new();
    for line in review.lines() {
        if line == "## Findings" {
            in_findings = true;
            continue;
        }
        if in_findings && line == "## Verdict" {
            break;
        }
        if !in_findings || !line.starts_with('|') {
            continue;
        }
        let fields: Vec<_> = line.split('|').map(str::trim).collect();
        let finding = fields.get(1).copied().unwrap_or_default();
        let work_unit = fields.get(5).copied().unwrap_or_default();
        if valid_id(finding, "AR-") && valid_id(work_unit, "W") {
            pairs.push((finding.to_string(), work_unit.to_string()));
        }
    }
    pairs
}

fn json_string_value(content: &str, field: &str) -> Option<String> {
    let marker = format!("\"{field}\"");
    let start = content.find(&marker)? + marker.len();
    let rest = &content[start..];
    let quote = rest.find('"')? + 1;
    let end = rest[quote..].find('"')? + quote;
    Some(rest[quote..end].to_string())
}

fn scratch_dir() -> PathBuf {
    env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("planning-agent")
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if matches!(
        args.first().map(String::as_str),
        Some("--help") | Some("-h")
    ) {
        usage(0);
    }
    let mut plan_dir = None;
    let mut claimed_by = String::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--plan-dir" if index + 1 < args.len() => {
                plan_dir = Some(PathBuf::from(&args[index + 1]));
                index += 2;
            }
            value if value.starts_with("--plan-dir=") => {
                plan_dir = Some(PathBuf::from(&value[11..]));
                index += 1;
            }
            "--claimed-by" if index + 1 < args.len() => {
                claimed_by = args[index + 1].clone();
                index += 2;
            }
            value if value.starts_with("--claimed-by=") => {
                claimed_by = value[13..].to_string();
                index += 1;
            }
            "--" => {
                index += 1;
                break;
            }
            value if value.starts_with('-') => {
                eprintln!("{COMMAND}: unknown option: {value}");
                usage(64)
            }
            value if plan_dir.is_none() => {
                plan_dir = Some(PathBuf::from(value));
                index += 1;
            }
            _ => usage(64),
        }
    }
    if index < args.len() {
        usage(64);
    }
    let plan = plan_dir.unwrap_or_else(|| usage(64));
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 66);
    }
    let review_file = plan.join("adversarial-review.md");
    if !review_file.is_file() {
        die(
            format!("adversarial-review.md not found: {}", review_file.display()),
            66,
        );
    }
    let json_file = plan.join("fix-keys.json");
    if !json_file.is_file() {
        println!("ungated plan (no fix-keys.json): no fix verification required");
        return;
    }
    let review =
        fs::read_to_string(&review_file).unwrap_or_else(|error| die(error.to_string(), 66));
    let json = fs::read_to_string(&json_file).unwrap_or_else(|error| die(error.to_string(), 66));
    let session = json_string_value(&json, "session_id")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| die("fix-keys.json has no session_id", 70));
    let minted_by = json_string_value(&json, "minted_by").unwrap_or_default();
    let pairs = gated_pairs(&review);
    if pairs.is_empty() {
        println!(
            "no gated (finding, work unit) pairs in {}: no fix verification required",
            plan.display()
        );
        return;
    }
    let secret_file = scratch_dir()
        .join("review-fix-keys")
        .join(&session)
        .join("secret");
    if !secret_file.is_file() {
        die(
            format!(
                "session secret missing: {} (was the session invalidated at approval?)",
                secret_file.display()
            ),
            70,
        );
    }
    let fixes_file = plan.join("fixes.md");
    if !fixes_file.is_file() {
        die(
            "fixes.md missing; a gated plan must record one claim per (finding, work unit)",
            70,
        );
    }
    let secret =
        fs::read_to_string(&secret_file).unwrap_or_else(|error| die(error.to_string(), 70));
    let secret = secret.trim_end_matches(['\n', '\r']);
    let claims = fs::read_to_string(&fixes_file).unwrap_or_else(|error| die(error.to_string(), 70));
    let mut failures = 0usize;
    let mut warnings = 0usize;
    let mut claimed_pairs = Vec::new();
    for (line_number, line) in claims.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 {
            eprintln!(
                "malformed fixes.md claim (line {}): expected finding_id, work_unit, key",
                line_number + 1
            );
            failures += 1;
            continue;
        }
        let pair = (fields[0].to_string(), fields[1].to_string());
        if !pairs.contains(&pair) {
            eprintln!(
                "ignoring claim for pair {}/{} (not gated in fix-keys.json)",
                fields[0], fields[1]
            );
            warnings += 1;
            continue;
        }
        let expected = key(secret, &format!("{session}|{}|{}", fields[0], fields[1]));
        if expected != fields[2] {
            eprintln!(
                "fix key mismatch for {}/{} (forged or stale key)",
                fields[0], fields[1]
            );
            failures += 1;
        }
        claimed_pairs.push(pair);
    }
    for (finding, work_unit) in &pairs {
        if !claimed_pairs
            .iter()
            .any(|pair| pair == &(finding.clone(), work_unit.clone()))
        {
            eprintln!("no fix key claim recorded for gated pair {finding}/{work_unit}");
            failures += 1;
        }
    }
    if !claimed_by.is_empty() && !minted_by.is_empty() && claimed_by == minted_by {
        eprintln!("self-certification: fix claims for {} were recorded by the same session ({}) that minted the keys -- let the reviewer write adversarial-review-incoming.md so the findings arrive from its own session, have a fresh reviewer re-review and re-mint, or, in a harness whose roles share one derived session id, re-mint with MINTED_BY=<reviewer identity> so the recorded minter is the role", plan.display(), claimed_by);
        failures += 1;
    }
    if failures > 0 {
        die(
            format!(
                "fix-keys verification failed for {} ({} failure(s), {} warning(s))",
                plan.display(),
                failures,
                warnings
            ),
            70,
        );
    }
    println!(
        "fix-keys verification passed for {} ({} warning(s))",
        plan.display(),
        warnings
    );
}
