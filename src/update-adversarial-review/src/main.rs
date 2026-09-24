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
    println!("Usage: {COMMAND} [--plan-dir] <plan-directory> [--file CSV] [--cycle N]\n       {COMMAND} [--plan-dir] <plan-directory> --set-rationale <text>\n       {COMMAND} --help\n\nRewrites the adversarial-review \"## Findings\" table from CSV rows whose columns\nare: ID, Missing or over-broad item, Required plan change, Status, Work unit.\nRows are read from adversarial-review-incoming.md if present, else from --file\nCSV, else from stdin.\n\n  --file CSV          read the rows from CSV instead of stdin\n  --cycle N           number the archived history entry N instead of the next one up\n  --check             validate the rows (shape and mint) and report; writes nothing\n  --set-rationale T   set the Verdict's own \"- Rationale:\" line to T, stamped with\n                      the review cycle it describes (\"- Rationale cycle: N\", the\n                      same number the CURRENT findings table would get if archived\n                      right now). A later findings-table update that archives past\n                      that cycle leaves the stamp behind, so validate-plan can flag\n                      the rationale as describing a superseded cycle (T56) instead\n                      of silently reading as current. A standalone action: does not\n                      touch the Findings table, and takes no CSV/stdin input.\n\nThis does not set the Verdict to approved. Author the Verdict (--set-rationale,\nthen edit the Status line directly) and run\n`update-plan-content.sh --review-status <plan> approved` separately.");
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

/// Sets the Verdict section's own "- Rationale:" line to `rationale`,
/// stamped immediately below with "- Rationale cycle: N" -- N computed the
/// same way `cycle_number` decides what the CURRENT (not yet archived)
/// findings table would be numbered if archived right now. Written at
/// rationale-write-time rather than left to be inferred later from the
/// archive's own contents (T56): the earlier attempt at this compared two
/// DIFFERENT meanings that happen to share the name "Cycle N" (the archive
/// entry names the state BEFORE a run; a hand-written "Cycle N" in prose
/// named the cycle whose findings had just landed) and misfired in both
/// directions. Using this exact function for both the stamp and the later
/// staleness comparison (planning-validator-docs's own copy, kept in sync
/// by doc comment cross-reference since it is 8 lines and unlikely to
/// drift) means the two values are directly comparable by construction,
/// not by arithmetic on a number recovered from parsed prose.
///
/// A standalone action, independent of the findings-CSV flow this binary's
/// main mode runs: does not read or write the Findings table, the history
/// file, or run mint-fix-keys.
fn set_verdict_rationale(review_file: &Path, history: &Path, rationale: &str) {
    let cycle = cycle_number(history, None);
    let review = fs::read_to_string(review_file).unwrap_or_else(|error| die(error.to_string(), 65));
    let mut output = String::new();
    let mut in_verdict = false;
    let mut wrote_rationale = false;
    for line in review.lines() {
        if line == "## Verdict" {
            in_verdict = true;
            output.push_str(line);
            output.push('\n');
            continue;
        }
        if in_verdict && line.starts_with("## ") {
            in_verdict = false;
        }
        if in_verdict && line.starts_with("- Rationale cycle:") {
            // A stamp from a prior --set-rationale call: dropped here, a
            // fresh one is emitted right after the Rationale line below.
            continue;
        }
        if in_verdict && line.starts_with("- Rationale:") {
            output.push_str(&format!("- Rationale: {rationale}\n"));
            output.push_str(&format!("- Rationale cycle: {cycle}\n"));
            wrote_rationale = true;
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    if in_verdict && !wrote_rationale {
        // The Verdict section had no "- Rationale:" line to replace at all
        // (malformed content) -- append one rather than silently doing
        // nothing, matching create-adversarial-review's own scaffold shape.
        output.push_str(&format!("- Rationale: {rationale}\n"));
        output.push_str(&format!("- Rationale cycle: {cycle}\n"));
        wrote_rationale = true;
    }
    if !wrote_rationale {
        die("adversarial-review.md has no ## Verdict section", 65);
    }
    atomic_write(review_file, output.as_bytes()).unwrap_or_else(|error| die(error, 70));
    println!(
        "Set Verdict rationale in {} (cycle {cycle})",
        review_file.display()
    );
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

fn history_scope_preamble(history: &Path) -> String {
    let mut preamble = String::new();
    let mut in_cycle = false;
    for line in fs::read_to_string(history).unwrap_or_default().lines() {
        if line.starts_with("## Cycle ") {
            in_cycle = true;
            preamble.clear();
        } else if in_cycle && is_scope_field_line(line) {
            preamble.push_str(line);
            preamble.push('\n');
        }
    }
    preamble
}

/// Matches only the four known Review-scope fields this goal archives --
/// never Request/Repository-context-inspected, which stay live-only.
fn is_scope_field_line(line: &str) -> bool {
    const PREFIXES: [&str; 4] = [
        "- Reviewer session:",
        "- Elapsed:",
        "- Cost signal:",
        "- Tokens:",
    ];
    PREFIXES.iter().any(|prefix| line.starts_with(prefix))
}

fn archive(history: &Path, scope_preamble: &str, landed_rows: &str, explicit: Option<i64>) {
    let number = cycle_number(history, explicit);
    let existing = fs::read_to_string(history).unwrap_or_default();
    if !landed_rows.is_empty()
        && landed_rows == history_rows(history)
        && scope_preamble == history_scope_preamble(history)
    {
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
    if !scope_preamble.is_empty() {
        append.push_str(scope_preamble);
        append.push('\n');
    }
    if landed_rows.is_empty() {
        append.push_str("_No row-level findings were recorded for this cycle._\n");
    } else {
        append.push_str(landed_rows);
    }
    let mut result = existing;
    result.push_str(&append);
    atomic_write(history, result.as_bytes()).unwrap_or_else(|error| die(error, 70));
    eprintln!(
        "Archived this cycle's Findings table to {} (Cycle {number})",
        history.display()
    );
}

// A bare `Command::new("mint-fix-keys")` relies on the OS resolving the name
// through $PATH, but nothing in this repo's install or exec path ever puts a
// skill's own scripts/bin directory on $PATH (confirmed live: this failed
// with a raw ENOENT even though a working mint-fix-keys sat right next to
// this very binary). MINT_FIX_KEYS_BIN keeps working as an explicit override;
// otherwise resolve the sibling in this binary's own directory first, the
// same directory plan_exec_compiled_binary_if_present.sh execs THIS binary
// from -- mirroring run-adversary-probe's own sibling() helper -- and only
// fall back to the bare name (still $PATH-dependent) if current_exe() can't
// be read at all.
fn mint_fix_keys_binary() -> PathBuf {
    if let Some(path) = env::var_os("MINT_FIX_KEYS_BIN") {
        return PathBuf::from(path);
    }
    env::current_exe()
        .ok()
        .and_then(|path| {
            path.parent()
                .map(|parent| parent.join(planning_core::exe_name("mint-fix-keys")))
        })
        .unwrap_or_else(|| PathBuf::from("mint-fix-keys"))
}

fn mint(_plan: &Path, review: &str) -> Result<(), String> {
    let temporary = env::temp_dir().join(format!("adversarial-review-mint-{}", std::process::id()));
    fs::create_dir_all(&temporary).map_err(|error| error.to_string())?;
    let review_file = temporary.join("adversarial-review.md");
    fs::write(&review_file, review).map_err(|error| error.to_string())?;
    let result = Command::new(mint_fix_keys_binary())
        .arg(&temporary)
        .output();
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
    let mut set_rationale: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--plan-dir" | "--file" | "--cycle" | "--set-rationale" => {
                index += 1;
                let value = args.get(index).cloned().unwrap_or_else(|| usage(64));
                match args[index - 1].as_str() {
                    "--plan-dir" => plan = Some(value),
                    "--file" => file = Some(PathBuf::from(value)),
                    "--set-rationale" => set_rationale = Some(value),
                    _ => cycle = value.parse().ok(),
                }
            }
            "--check" => check = true,
            value if value.starts_with("--plan-dir=") => {
                plan = Some(value["--plan-dir=".len()..].to_string());
            }
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
    let review_file = plan.join("adversarial-review.md");
    if let Some(rationale) = set_rationale {
        if file.is_some() || cycle.is_some() || check {
            eprintln!("{COMMAND}: --set-rationale does not take --file/--cycle/--check");
            usage(64);
        }
        if !review_file.is_file() {
            die(
                format!(
                    "adversarial-review.md not found: {} (run create-adversarial-review.sh first)",
                    review_file.display()
                ),
                66,
            );
        }
        git_snapshot(&plan);
        set_verdict_rationale(
            &review_file,
            &plan.join("adversarial-review-history.md"),
            &rationale,
        );
        return;
    }
    git_snapshot(&plan);
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
    let mut scope_preamble = String::new();
    let mut in_scope = false;
    for line in review.lines() {
        if line == "## Review scope" {
            in_scope = true;
            continue;
        }
        if in_scope && line == "## Findings" {
            in_scope = false;
        }
        if in_scope && is_scope_field_line(line) {
            scope_preamble.push_str(line);
            scope_preamble.push('\n');
        }
    }
    // scope_preamble describes the findings this call is landing (table), not
    // the outgoing ones it is about to replace (B353): a reviewer fills in
    // Reviewer session/Elapsed/Cost signal/Tokens to describe their OWN
    // findings before submitting them, so archiving must pair the two by that
    // same authorship, not by which table happened to be live at call time.
    archive(&history_file, &scope_preamble, &table, cycle);
    atomic_write(&review_file, rewritten.as_bytes()).unwrap_or_else(|error| die(error, 70));
    if consumed_incoming {
        let _ = fs::remove_file(plan.join("adversarial-review-incoming.md"));
    }
    let output = Command::new(mint_fix_keys_binary())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "update-adversarial-review-test-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    const SCAFFOLD: &str = "# Adversarial review: demo\n\n## Review scope\n\n§ 1.1\n- Request: x\n\n## Findings\n\n| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n|---|---|---|---|---|\n| AR-01 | No finding recorded yet. | N/A | ✅ resolved | N/A |\n\n## Verdict\n\n- Status: `💤 pending`\n- Rationale: <why no unresolved work remains>\n";

    #[test]
    fn set_verdict_rationale_replaces_the_placeholder_and_stamps_cycle_one_on_an_empty_history() {
        let dir = scratch("first-use");
        let review = dir.join("adversarial-review.md");
        let history = dir.join("adversarial-review-history.md");
        fs::write(&review, SCAFFOLD).unwrap();
        // No history file at all yet -- cycle_number's own empty-input default.
        set_verdict_rationale(&review, &history, "No unresolved findings remain.");
        let text = fs::read_to_string(&review).unwrap();
        assert!(text.contains("- Rationale: No unresolved findings remain.\n"));
        assert!(text.contains("- Rationale cycle: 1\n"));
        assert!(!text.contains("<why no unresolved work remains>"));
        // Everything outside the Verdict section survives untouched.
        assert!(text.contains("| AR-01 | No finding recorded yet. |"));
    }

    #[test]
    fn set_verdict_rationale_replaces_a_prior_rationale_and_stamp_together() {
        let dir = scratch("replace");
        let review = dir.join("adversarial-review.md");
        let history = dir.join("adversarial-review-history.md");
        fs::write(&review, SCAFFOLD).unwrap();
        fs::write(
            &history,
            "\n## Cycle 1\n\n_No row-level findings were recorded for this cycle._\n",
        )
        .unwrap();
        set_verdict_rationale(&review, &history, "First rationale.");
        set_verdict_rationale(&review, &history, "Second, corrected rationale.");
        let text = fs::read_to_string(&review).unwrap();
        assert!(!text.contains("First rationale."));
        assert!(text.contains("- Rationale: Second, corrected rationale.\n"));
        // Exactly one stamp line survives -- the old one was dropped, not
        // left behind alongside the new one.
        assert_eq!(text.matches("- Rationale cycle:").count(), 1);
        assert!(text.contains("- Rationale cycle: 2\n"));
    }

    #[test]
    fn set_verdict_rationale_reports_the_next_free_cycle_not_the_last_archived_one() {
        let dir = scratch("next-free");
        let review = dir.join("adversarial-review.md");
        let history = dir.join("adversarial-review-history.md");
        fs::write(&review, SCAFFOLD).unwrap();
        fs::write(&history, "\n## Cycle 1\n\nrow\n\n## Cycle 2\n\nrow\n").unwrap();
        // The CURRENT (not yet archived) findings table would become Cycle 3
        // if archived right now -- the rationale describing it is stamped
        // with that same number, not the last one already archived (2).
        set_verdict_rationale(&review, &history, "Describes cycle 3's findings.");
        let text = fs::read_to_string(&review).unwrap();
        assert!(text.contains("- Rationale cycle: 3\n"));
    }

    // A malformed document with no ## Verdict heading at all calls die(),
    // which exits the process -- not safely exercisable from inside this
    // test binary. Covered instead by the shell integration test
    // (planning/tests/test-adversarial-review-cycles.sh), which can check a
    // real subprocess's exit code.
}
