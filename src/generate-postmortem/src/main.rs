// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const COMMAND: &str = "generate-postmortem.sh";
const FIELD_PREFIXES: [(&str, usize); 4] = [
    ("- Reviewer session:", 0),
    ("- Elapsed:", 1),
    ("- Cost signal:", 2),
    ("- Tokens:", 3),
];

fn usage(code: i32) -> ! {
    println!("Usage: {COMMAND} [--plan-dir] <plan-directory> [--output PATH]\n       {COMMAND} --help\n\nRenders a per-cycle cost/findings postmortem from <plan-directory>/adversarial-review-history.md.\n\n  --output PATH   write the postmortem to PATH instead of the default\n                  <plan-directory-parent>/postmortems/<plan-directory-basename>.md");
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

struct Cycle {
    number: String,
    fields: [Option<String>; 4],
    findings: usize,
}

fn is_findings_row(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('|') {
        return false;
    }
    let core = trimmed.trim_matches('|');
    if core
        .chars()
        .all(|ch| ch == '-' || ch == '|' || ch.is_whitespace())
    {
        return false;
    }
    let first_cell = core.split('|').next().unwrap_or("").trim();
    first_cell != "ID"
}

fn parse_history(text: &str) -> Vec<Cycle> {
    let mut cycles = Vec::new();
    let mut current: Option<Cycle> = None;
    for line in text.lines() {
        if let Some(number) = line.strip_prefix("## Cycle ") {
            if let Some(cycle) = current.take() {
                cycles.push(cycle);
            }
            current = Some(Cycle {
                number: number.trim().to_string(),
                fields: [None, None, None, None],
                findings: 0,
            });
            continue;
        }
        let Some(cycle) = current.as_mut() else {
            continue;
        };
        let mut matched = false;
        for (prefix, index) in FIELD_PREFIXES {
            if let Some(value) = line.strip_prefix(prefix) {
                if cycle.fields[index].is_none() {
                    cycle.fields[index] = Some(value.trim().to_string());
                }
                matched = true;
                break;
            }
        }
        if !matched && is_findings_row(line) {
            cycle.findings += 1;
        }
    }
    if let Some(cycle) = current.take() {
        cycles.push(cycle);
    }
    cycles
}

fn parse_nonneg_int(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    (!trimmed.is_empty() && trimmed.chars().all(|ch| ch.is_ascii_digit()))
        .then(|| trimmed.parse::<u64>().ok())
        .flatten()
}

fn render(cycles: &[Cycle]) -> String {
    let mut out = String::from("# Postmortem\n\n");
    if cycles.is_empty() {
        out.push_str(
            "No adversarial-review cycles have been archived for this plan yet (zero cycles recorded).\n\n",
        );
    } else {
        out.push_str("| Cycle | Reviewer session | Elapsed | Cost signal | Tokens | Findings |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for cycle in cycles {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                cycle.number,
                cycle.fields[0].as_deref().unwrap_or("not reported"),
                cycle.fields[1].as_deref().unwrap_or("not reported"),
                cycle.fields[2].as_deref().unwrap_or("not reported"),
                cycle.fields[3].as_deref().unwrap_or("not reported"),
                cycle.findings,
            ));
        }
        out.push('\n');
    }

    let total_cycles = cycles.len();
    let total_findings: usize = cycles.iter().map(|cycle| cycle.findings).sum();
    out.push_str(&format!("Total cycles: {total_cycles}\n"));
    out.push_str(&format!("Total findings: {total_findings}\n"));

    let numeric_tokens: Vec<u64> = cycles
        .iter()
        .filter_map(|cycle| cycle.fields[3].as_deref().and_then(parse_nonneg_int))
        .collect();
    let reported = numeric_tokens.len();
    if reported == 0 {
        out.push_str(&format!(
            "Total tokens: not reported (0 of {total_cycles} cycles reported)\n"
        ));
    } else {
        let sum: u64 = numeric_tokens.iter().sum();
        if reported == total_cycles {
            out.push_str(&format!("Total tokens: {sum}\n"));
        } else {
            out.push_str(&format!(
                "Total tokens: {sum} (partial total, {reported} of {total_cycles} cycles reported)\n"
            ));
        }
    }
    out
}

fn default_output_path(canonical_plan_dir: &Path) -> PathBuf {
    let parent = canonical_plan_dir
        .parent()
        .unwrap_or_else(|| die("plan directory has no parent", 66));
    let basename = canonical_plan_dir
        .file_name()
        .unwrap_or_else(|| die("plan directory has no basename", 66));
    parent
        .join("postmortems")
        .join(format!("{}.md", basename.to_string_lossy()))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "-h" || arg == "--help")
    {
        usage(0)
    }
    let mut plan_dir: Option<String> = None;
    let mut output: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => usage(0),
            "--output" => {
                if index + 1 >= args.len() {
                    usage(64)
                }
                output = Some(args[index + 1].clone());
                index += 2;
            }
            "--" => index += 1,
            value if value.starts_with('-') => {
                eprintln!("{COMMAND}: unknown option: {value}");
                usage(64)
            }
            value if plan_dir.is_none() => {
                plan_dir = Some(value.to_string());
                index += 1;
            }
            _ => usage(64),
        }
    }
    let plan_dir = plan_dir.unwrap_or_else(|| usage(64));
    let plan_path = Path::new(&plan_dir);
    // is_dir() alone catches both a nonexistent path and an existing
    // non-directory path; canonicalize() only errors on the former.
    if !plan_path.is_dir() {
        die(
            format!("Plan directory not found: {}", plan_path.display()),
            66,
        );
    }
    let canonical = plan_path
        .canonicalize()
        .unwrap_or_else(|error| die(error.to_string(), 66));

    let history_file = canonical.join("adversarial-review-history.md");
    let text = fs::read_to_string(&history_file).unwrap_or_default();
    let cycles = parse_history(&text);
    let rendered = render(&cycles);

    let output_path = output
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(&canonical));
    if let Some(dir) = output_path.parent() {
        fs::create_dir_all(dir).unwrap_or_else(|error| die(error.to_string(), 70));
    }
    fs::write(&output_path, rendered).unwrap_or_else(|error| die(error.to_string(), 70));
    println!("Wrote postmortem to {}", output_path.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cycle_with_all_four_fields_present_parses_each_correctly() {
        let text = "\n## Cycle 1\n\n- Reviewer session: sess-abc\n- Elapsed: 9min\n- Cost signal: 2 findings this cycle\n- Tokens: 1000\n\n| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n|---|---|---|---|---|\n| AR-01 | x | y | open | W01 |\n| AR-02 | x | y | open | W01 |\n";
        let cycles = parse_history(text);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].number, "1");
        assert_eq!(cycles[0].fields[0].as_deref(), Some("sess-abc"));
        assert_eq!(cycles[0].fields[1].as_deref(), Some("9min"));
        assert_eq!(
            cycles[0].fields[2].as_deref(),
            Some("2 findings this cycle")
        );
        assert_eq!(cycles[0].fields[3].as_deref(), Some("1000"));
        assert_eq!(cycles[0].findings, 2);
    }

    #[test]
    fn a_cycle_with_no_fields_renders_not_reported() {
        let text = "\n## Cycle 1\n\n| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n|---|---|---|---|---|\n| AR-01 | x | y | open | W01 |\n";
        let cycles = parse_history(text);
        assert_eq!(cycles.len(), 1);
        assert!(cycles[0].fields.iter().all(|field| field.is_none()));
        let rendered = render(&cycles);
        assert!(rendered
            .contains("| 1 | not reported | not reported | not reported | not reported | 1 |"));
        assert!(rendered.contains("Total tokens: not reported (0 of 1 cycles reported)"));
    }

    #[test]
    fn a_non_numeric_tokens_value_is_excluded_from_the_total_but_shown_verbatim() {
        let text = "\n## Cycle 1\n\n- Tokens: about a lot\n\n| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n|---|---|---|---|---|\n| AR-01 | x | y | open | W01 |\n\n## Cycle 2\n\n- Tokens: 500\n\n| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n|---|---|---|---|---|\n| AR-02 | x | y | open | W01 |\n";
        let cycles = parse_history(text);
        assert_eq!(cycles.len(), 2);
        let rendered = render(&cycles);
        assert!(rendered
            .contains("| 1 | not reported | not reported | not reported | about a lot | 1 |"));
        assert!(rendered.contains("Total tokens: 500 (partial total, 1 of 2 cycles reported)"));
    }

    #[test]
    fn a_history_with_zero_cycle_headings_renders_without_panicking() {
        let cycles = parse_history("no cycle headings here at all\n");
        assert!(cycles.is_empty());
        let rendered = render(&cycles);
        assert!(rendered.contains("zero cycles recorded"));
        assert!(rendered.contains("Total cycles: 0"));
        assert!(rendered.contains("Total findings: 0"));
        assert!(rendered.contains("Total tokens: not reported (0 of 0 cycles reported)"));
    }

    #[test]
    fn header_and_separator_rows_are_excluded_while_data_rows_are_counted() {
        let text = "\n## Cycle 1\n\n| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n|---|---|---|---|---|\n| AR-01 | x | y | open | W01 |\n| AR-02 | x | y | open | W01 |\n| AR-03 | x | y | open | W01 |\n";
        let cycles = parse_history(text);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].findings, 3);
    }

    #[test]
    fn parse_nonneg_int_rejects_anything_that_is_not_a_plain_digit_string() {
        assert_eq!(parse_nonneg_int("42"), Some(42));
        assert_eq!(parse_nonneg_int("0"), Some(0));
        assert_eq!(parse_nonneg_int(""), None);
        assert_eq!(parse_nonneg_int("about a lot"), None);
        assert_eq!(parse_nonneg_int("-5"), None);
        assert_eq!(parse_nonneg_int("1,000"), None);
    }
}
