// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs;
use std::path::PathBuf;

fn usage(code: i32) -> ! {
    let n = env::args()
        .next()
        .and_then(|p| {
            PathBuf::from(p)
                .file_name()
                .map(|v| v.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "create-adversarial-review".into());
    println!("Usage: {n} [--plan-dir] <plan-directory>\n       {n} --help");
    std::process::exit(code)
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => usage(0),
            "--plan-dir" => {
                i += 1;
                positional.push(args.get(i).cloned().unwrap_or_else(|| usage(64)));
            }
            value if value.starts_with("--plan-dir=") => {
                positional.push(value["--plan-dir=".len()..].to_string())
            }
            "--" => {
                positional.extend(args.iter().skip(i + 1).cloned());
                break;
            }
            value if value.starts_with('-') => {
                eprintln!("create-adversarial-review: unknown option: {value}");
                usage(64)
            }
            value => positional.push(value.to_string()),
        }
        i += 1;
    }
    if positional.len() != 1 {
        usage(64)
    }
    let plan = PathBuf::from(&positional[0]);
    let review = plan.join("adversarial-review.md");
    if !plan.is_dir() {
        eprintln!("Plan directory not found: {}", plan.display());
        std::process::exit(66)
    }
    if review.exists() {
        eprintln!("Adversarial review already exists: {}", review.display());
        std::process::exit(73)
    }
    let n = plan.file_name().unwrap().to_string_lossy();
    let output=format!("# Adversarial review: {n}\n\n## Review scope\n\n§ 1.1\n- Request: <verbatim or precise summary>\n- Repository/context inspected: <what was checked>\n- Reviewer session: <session id, so a claim can be traced to the run that made it>\n- Elapsed: <wall time for the cycle>\n- Cost signal: <findings this cycle; falling counts mean converging, flat means the plan is not the problem>\n\n## Findings\n\n| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n|---|---|---|---|---|\n| AR-01 | No finding recorded yet. | N/A | ✅ resolved | N/A |\n\n## Verdict\n\n- Status: `💤 pending`\n- Rationale: <why no unresolved work remains>\n");
    let temp = review.with_extension(format!("md.tmp.{}", std::process::id()));
    fs::write(&temp, output).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(66)
    });
    fs::rename(&temp, &review).unwrap_or_else(|e| {
        let _ = fs::remove_file(&temp);
        eprintln!("{e}");
        std::process::exit(66)
    });
    println!("Created {}", review.display());
}
