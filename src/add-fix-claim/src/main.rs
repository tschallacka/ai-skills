// MODE: DEV
// PACKAGE: PROD
use planning_core::{atomic_write, git_snapshot};
use std::env;
use std::fs;
use std::path::PathBuf;

const COMMAND: &str = "add-fix-claim.sh";

fn usage(code: i32) -> ! {
    println!(
        "Usage: {COMMAND} [--plan-dir] <plan-directory> --finding <AR-NN> --work-unit <WNN> --key <hex>\n       {COMMAND} --help\n\n  --finding AR-NN    the adversarial-review finding the fix answers\n  --work-unit WNN    the work unit that carried the fix\n  --key hex          the fix key minted for that pair (64 lowercase hex chars)\n\nRecords one claim in <plan-directory>/fixes.md. Run verify-fix-keys.sh from a\nsession that did not mint the keys to check the claims."
    );
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code);
}

fn required(args: &[String], index: &mut usize) -> String {
    *index += 1;
    args.get(*index).cloned().unwrap_or_else(|| usage(64))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut plan_option = None;
    let mut finding = String::new();
    let mut work_unit = String::new();
    let mut key = String::new();
    let mut positional = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => usage(0),
            "--plan-dir" => plan_option = Some(required(&args, &mut index)),
            "--finding" => finding = required(&args, &mut index),
            "--work-unit" => work_unit = required(&args, &mut index),
            "--key" => key = required(&args, &mut index),
            "--" => {
                positional.extend(args.iter().skip(index + 1).cloned());
                break;
            }
            value if value.starts_with('-') => {
                eprintln!("{COMMAND}: unknown option: {value}");
                usage(64)
            }
            value => positional.push(value.to_string()),
        }
        index += 1;
    }
    if let Some(plan) = plan_option {
        positional.push(plan);
    }
    if positional.len() != 1 {
        usage(64)
    }
    let plan = PathBuf::from(&positional[0]);
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 66)
    }
    if finding.is_empty() {
        die(
            "--finding is required (an adversarial-review id such as AR-01)",
            64,
        )
    }
    if work_unit.is_empty() {
        die("--work-unit is required (a work-unit id such as W05)", 64)
    }
    if key.is_empty() {
        die("--key is required (the fix key minted for this pair)", 64)
    }
    if !finding
        .strip_prefix("AR-")
        .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
    {
        die(format!("Finding id must match AR-NN: {finding}"), 64)
    }
    if !work_unit
        .strip_prefix('W')
        .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
    {
        die(format!("Work unit must match WNN: {work_unit}"), 64)
    }
    if key.len() != 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        die(
            format!("Key must be 64 lowercase hex characters as minted by mint-fix-keys.sh: {key}"),
            64,
        )
    }

    let keys_file = plan.join("fix-keys.json");
    if !keys_file.is_file() {
        die(format!("no fix-keys.json in {} -- a reviewer mints the keys with mint-fix-keys.sh before a fix can be claimed", plan.display()), 66)
    }
    let review_file = plan.join("adversarial-review.md");
    if !review_file.is_file() {
        die(
            format!(
                "no adversarial-review.md in {} -- there are no findings to claim a fix for",
                plan.display()
            ),
            66,
        )
    }
    let review =
        fs::read_to_string(&review_file).unwrap_or_else(|error| die(error.to_string(), 64));
    let gated = review
        .lines()
        .skip_while(|line| *line != "## Findings")
        .skip(1)
        .take_while(|line| *line != "## Verdict")
        .filter_map(|line| {
            if !line.starts_with('|') {
                return None;
            }
            let fields: Vec<_> = line.split('|').map(str::trim).collect();
            let fid = fields.get(1).copied().unwrap_or_default();
            let wu = fields.get(5).copied().unwrap_or_default();
            (fid.strip_prefix("AR-")
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                && wu
                    .strip_prefix('W')
                    .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())))
            .then(|| format!("{fid}\t{wu}"))
        })
        .any(|pair| pair == format!("{finding}\t{work_unit}"));
    if !gated {
        die(format!("{finding}/{work_unit} is not a gated pair in the review's Findings table -- claim a pair the reviewer recorded, or add the finding and re-mint"), 65)
    }
    let keys = fs::read_to_string(&keys_file).unwrap_or_default();
    if !keys.contains(&key) {
        die(format!("that key is not in fix-keys.json -- it is forged, or stale from a previous minting; ask the reviewer for the key minted for {finding}/{work_unit}"), 65)
    }
    let claims_file = plan.join("fixes.md");
    let claim = format!("{finding}\t{work_unit}\t{key}");
    let existing = fs::read_to_string(&claims_file).unwrap_or_default();
    if existing.lines().any(|line| line == claim) {
        println!("{COMMAND}: claim already recorded for {finding}/{work_unit}; nothing to do");
        return;
    }
    if existing.lines().any(|line| {
        let mut fields = line.split('\t');
        fields.next() == Some(finding.as_str()) && fields.next() == Some(work_unit.as_str())
    }) {
        die(format!("{finding}/{work_unit} is already claimed with a different key in fixes.md -- remove that line, or re-mint the pair"), 73)
    }
    git_snapshot(&plan);
    let mut output = existing;
    output.push_str(&claim);
    output.push('\n');
    atomic_write(&claims_file, output.as_bytes()).unwrap_or_else(|error| die(error, 73));
    println!("Recorded fix claim {finding}/{work_unit} in fixes.md");
    eprintln!("{COMMAND}: verify with verify-fix-keys.sh --claimed-by <this session>, from a session that did not mint the keys");
}
