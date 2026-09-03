// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn usage(code: i32) -> ! {
    let n = env::args()
        .next()
        .unwrap_or_else(|| "create-work-unit-inventory".into());
    let n = Path::new(&n)
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("create-work-unit-inventory");
    println!("Usage: {n} [--plan-dir] <plan-directory>");
    println!("       {n} --help");
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
                eprintln!("create-work-unit-inventory: unknown option: {value}");
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
    if !plan.is_dir() {
        eprintln!("Plan directory not found: {}", plan.display());
        std::process::exit(66)
    }
    let inventory = plan.join("work-unit-inventory.md");
    if inventory.exists() {
        eprintln!(
            "Work-unit inventory already exists: {}",
            inventory.display()
        );
        std::process::exit(73)
    }
    let name = plan.file_name().unwrap().to_string_lossy();
    let output=format!("# Work-unit inventory: {name}\n\n## Definition-of-done coverage\n\n| Required outcome or proof | Work unit IDs | Notes |\n|---|---|---|\n| <outcome> | W01 | <why this work unit covers it> |\n\n## Work units\n\n| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n| W01 | source | `path/to/file` | `Class::method()` | `N/A` | <one concrete change> | — | 01-<goal> | 01-step-<slug> |\n\n## Decomposition review\n\n- [ ] Every definition-of-done item maps to one or more work units.\n- [ ] Every known affected file and changing symbol has its own work unit.\n- [ ] Every work unit has exactly one goal and one step.\n- [ ] Each goal has 2–10 work units, or records an allowed exception.\n- [ ] Each step has exactly one work unit and no unnamed incidental edits.\n- [ ] Dependencies form an executable order with no cycle.\n");
    let temporary = inventory.with_extension(format!("md.tmp.{}", std::process::id()));
    fs::write(&temporary, output).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(66)
    });
    fs::rename(&temporary, &inventory).unwrap_or_else(|e| {
        let _ = fs::remove_file(&temporary);
        eprintln!("{e}");
        std::process::exit(66)
    });
    println!("Created {}", inventory.display());
}
