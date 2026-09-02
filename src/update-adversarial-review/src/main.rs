// MODE: DEV
// PACKAGE: PROD
use planning_core::{atomic_write, git_snapshot};
use planning_table::{csv_to_markdown, review_finding_ids};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

const COMMAND: &str = "update-adversarial-review.sh";
const HEADER: &str = "ID,Missing or over-broad item,Required plan change,Status,Work unit";

fn usage(code: i32) -> ! {
    println!("Usage: {COMMAND} [--plan-dir] <plan-directory> [--file CSV] [--cycle N]\n       {COMMAND} --help\n\nRewrites the adversarial-review \"## Findings\" table from CSV rows whose columns\nare: ID, Missing or over-broad item, Required plan change, Status, Work unit.\nRows are read from adversarial-review-incoming.md if present, else from --file\nCSV, else from stdin.\n\n  --file CSV   read the rows from CSV instead of stdin\n  --cycle N    number the archived history entry N instead of the next one up\n  --check      validate the rows (shape and mint) and report; writes nothing\n\nThis does not set the Verdict to approved. Author the Verdict and run\n`update-plan-content.sh --review-status <plan> approved` separately.");
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

fn read_source(plan: &Path, file: Option<&Path>) -> (String, bool) {
    if let Some(file) = file {
        let text = fs::read_to_string(file)
            .unwrap_or_else(|_| die(format!("CSV file not found: {}", file.display()), 66));
        return (text, false);
    }
    let incoming = plan.join("adversarial-review-incoming.md");
    if incoming.is_file() {
        return (
            fs::read_to_string(&incoming).unwrap_or_else(|error| die(error.to_string(), 65)),
            true,
        );
    }
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|error| die(error.to_string(), 65));
    if input.is_empty() {
        die("no CSV provided. Pipe rows or a heredoc to stdin, pass --file PATH, or let reviewers write adversarial-review-incoming.md (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)", 64)
    }
    (input, false)
}

fn filtered_csv(input: &str) -> String {
    let mut header_removed = false;
    input
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return false;
            }
            if !header_removed && trimmed == HEADER {
                header_removed = true;
                return false;
            }
            true
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render(input: &str) -> Result<(String, usize), String> {
    let filtered = filtered_csv(input);
    if filtered.trim().is_empty() {
        return Err("no finding rows beneath the title/comment lines: write one row per line with 5 comma-separated columns (ID, Missing or over-broad item, Required plan change, Status, Work unit)".into());
    }
    let rows = filtered.lines().count();
    let full = format!("{HEADER}\n{filtered}\n");
    csv_to_markdown(5, &full)
        .map(|table| (table, rows))
        .map_err(|error| format!("invalid findings CSV: {error:?}"))
}

fn replace_findings(review: &str, table: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut in_findings = false;
    let mut found = false;
    for line in review.lines() {
        if line == "## Findings" {
            output.push_str(line);
            output.push_str("\n\n");
            output.push_str(table);
            in_findings = true;
            found = true;
            continue;
        }
        if in_findings && line == "## Verdict" {
            output.push('\n');
            output.push_str(line);
            output.push('\n');
            in_findings = false;
            continue;
        }
        if !in_findings {
            output.push_str(line);
            output.push('\n');
        }
    }
    found
        .then_some(output)
        .ok_or_else(|| "adversarial-review.md has no ## Findings section".into())
}

fn cycle_number(history: &Path, explicit: Option<i64>) -> i64 {
    if let Some(number) = explicit {
        return number;
    }
    fs::read_to_string(history)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.strip_prefix("## Cycle ")?.parse::<i64>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

fn history_rows(history: &Path) -> String {
    let mut rows = String::new();
    let mut in_cycle = false;
    for line in fs::read_to_string(history).unwrap_or_default().lines() {
        if line.starts_with("## Cycle ") {
            in_cycle = true;
            rows.clear();
        } else if in_cycle && line.starts_with('|') {
            rows.push_str(line);
            rows.push('\n');
        }
    }
    rows
}

fn archive(history: &Path, prior_rows: &str, explicit: Option<i64>) {
    let number = cycle_number(history, explicit);
    let existing = fs::read_to_string(history).unwrap_or_default();
    if !prior_rows.is_empty() && prior_rows == history_rows(history) {
        eprintln!(
            "Findings table is already the last entry in {}; not archiving it twice",
            history.display()
        );
        return;
    }
    if existing
        .lines()
        .any(|line| line == format!("## Cycle {number}").as_str())
    {
        die(format!("Cycle {number} is already recorded in {} with other findings; archiving would discard them (choose a free --cycle number)", history.display()), 73)
    }
    let mut append = String::new();
    append.push_str(&format!("\n## Cycle {number}\n\n"));
    if prior_rows.is_empty() {
        append.push_str("_No row-level findings were recorded for this cycle._\n");
    } else {
        append.push_str(prior_rows);
    }
    let mut result = existing;
    result.push_str(&append);
    atomic_write(history, result.as_bytes()).unwrap_or_else(|error| die(error, 70));
    eprintln!(
        "Archived previous Findings table to {} (Cycle {number})",
        history.display()
    );
}

fn mint(_plan: &Path, review: &str) -> Result<(), String> {
    let temporary = env::temp_dir().join(format!("adversarial-review-mint-{}", std::process::id()));
    fs::create_dir_all(&temporary).map_err(|error| error.to_string())?;
    let review_file = temporary.join("adversarial-review.md");
    fs::write(&review_file, review).map_err(|error| error.to_string())?;
    let binary = env::var_os("MINT_FIX_KEYS_BIN").unwrap_or_else(|| "mint-fix-keys".into());
    let result = Command::new(binary).arg(&temporary).output();
    let _ = fs::remove_dir_all(&temporary);
    match result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).to_string()),
        Err(error) => Err(format!("could not run mint-fix-keys: {error}")),
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "-h" || arg == "--help")
    {
        usage(0);
    }
    let mut plan = None;
    let mut file: Option<PathBuf> = None;
    let mut cycle = None;
    let mut check = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--plan-dir" | "--file" | "--cycle" => {
                index += 1;
                let value = args.get(index).cloned().unwrap_or_else(|| usage(64));
                match args[index - 1].as_str() {
                    "--plan-dir" => plan = Some(value),
                    "--file" => file = Some(PathBuf::from(value)),
                    _ => cycle = value.parse().ok(),
                }
            }
            "--check" => check = true,
            "--" => usage(64),
            value if value.starts_with('-') => {
                eprintln!("{COMMAND}: unknown option: {value}");
                usage(64);
            }
            value if plan.is_none() => plan = Some(value.to_string()),
            _ => usage(64),
        }
        index += 1;
    }
    let plan = plan.map(PathBuf::from).unwrap_or_else(|| usage(64));
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 66);
    }
    git_snapshot(&plan);
    let review_file = plan.join("adversarial-review.md");
    if !review_file.is_file() {
        die(
            format!(
                "adversarial-review.md not found: {} (run create-adversarial-review.sh first)",
                review_file.display()
            ),
            66,
        );
    }
    let before = review_finding_ids(&review_file).unwrap_or_default();
    let (input, consumed_incoming) = read_source(&plan, file.as_deref());
    if consumed_incoming {
        eprintln!(
            "Consumed reviewer findings from {}",
            plan.join("adversarial-review-incoming.md").display()
        );
    }
    let (table, row_count) = render(&input).unwrap_or_else(|error| die(error, 65));
    let review =
        fs::read_to_string(&review_file).unwrap_or_else(|error| die(error.to_string(), 65));
    let rewritten = replace_findings(&review, &table).unwrap_or_else(|error| die(error, 65));
    if let Err(diagnosis) = mint(&plan, &rewritten) {
        die(format!("findings were not minted; nothing was modified — fix the finding/work-unit cells and rerun (diagnosis above)\n{diagnosis}"), 65);
    }
    if check {
        println!("CSV is valid: {row_count} finding row(s) passed the shape and mint checks; nothing was written");
        return;
    }
    let history_file = plan.join("adversarial-review-history.md");
    let mut prior_rows = String::new();
    let mut in_findings = false;
    for line in review.lines() {
        if line == "## Findings" {
            in_findings = true;
            continue;
        }
        if in_findings && line == "## Verdict" {
            break;
        }
        if in_findings && line.starts_with('|') {
            prior_rows.push_str(line);
            prior_rows.push('\n');
        }
    }
    archive(&history_file, &prior_rows, cycle);
    atomic_write(&review_file, rewritten.as_bytes()).unwrap_or_else(|error| die(error, 70));
    if consumed_incoming {
        let _ = fs::remove_file(plan.join("adversarial-review-incoming.md"));
    }
    let binary = env::var_os("MINT_FIX_KEYS_BIN").unwrap_or_else(|| "mint-fix-keys".into());
    let output = Command::new(binary)
        .arg(&plan)
        .output()
        .unwrap_or_else(|error| die(error.to_string(), 70));
    eprint!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(70));
    }
    let after = review_finding_ids(&review_file).unwrap_or_default();
    let added: Vec<_> = after
        .iter()
        .filter(|id| !before.contains(id))
        .cloned()
        .collect();
    let dropped: Vec<_> = before
        .iter()
        .filter(|id| !after.contains(id))
        .cloned()
        .collect();
    print!(
        "Updated findings table in {}: {} row(s) in, {} row(s) out",
        review_file.display(),
        before.len(),
        after.len()
    );
    if !added.is_empty() {
        print!("; added {}", added.join(" "));
    }
    if !dropped.is_empty() {
        print!("; dropped {}", dropped.join(" "));
    }
    println!();
}
