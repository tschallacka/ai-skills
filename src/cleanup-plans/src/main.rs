// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

fn usage(code: i32) -> ! {
    println!("Usage: cleanup-plans.sh [-l|--list] [<plan-name> ...] [-y|--yes]");
    println!("       cleanup-plans.sh --help");
    println!("  -l, --list       list plans under the root, marking completed");
    println!("  -y, --yes        skip the confirmation prompt");
    std::process::exit(code);
}
fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("cleanup-plans: {}", message.as_ref());
    std::process::exit(code);
}
fn root() -> PathBuf {
    if let Ok(v) = env::var("PLANS_ROOT") {
        return PathBuf::from(v.trim_end_matches('/'));
    }
    let base = env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| env::var("HOME").ok().map(|v| format!("{v}/.config")))
        .or_else(|| env::var("USERPROFILE").ok().map(|v| format!("{v}/.config")))
        .unwrap_or_else(|| {
            die(
                "Unable to resolve the user home directory; set PLANS_ROOT",
                64,
            )
        });
    PathBuf::from(format!(
        "{}/tsch-ai-skills/plans",
        base.trim_end_matches('/')
    ))
}
fn plans(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(level) = fs::read_dir(root) else {
        return out;
    };
    for plan in level.flatten() {
        if plan.path().join("plan-description.md").is_file() {
            out.push(plan.path())
        }
    }
    out.sort();
    out.dedup();
    out
}
fn complete(plan: &Path) -> bool {
    let Ok(text) = fs::read_to_string(plan.join("progress.md")) else {
        return false;
    };
    let mut seen = 0;
    for row in text.lines() {
        if !row.starts_with('|') {
            continue;
        }
        if row.starts_with("|---") || row.starts_with("| Goalname") {
            continue;
        }
        let cells: Vec<_> = row.split('|').map(str::trim).collect();
        if cells.len() < 5 || cells[4] != "✅ completed" {
            return false;
        }
        seen += 1
    }
    seen > 0
}
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--help" || a == "-h") {
        usage(0)
    }
    let mut list = false;
    let mut yes = false;
    let mut wanted = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--list" | "-l" => list = true,
            "--yes" | "-y" => yes = true,
            "--" => {
                wanted.extend(args.iter().skip(i + 1).cloned());
                break;
            }
            v if v.starts_with('-') => usage(64),
            v => wanted.push(v.to_string()),
        }
        i += 1
    }
    let root = root();
    if !root.is_dir() {
        die(format!("plans root not found: {}", root.display()), 66)
    }
    let all = plans(&root);
    if all.is_empty() {
        println!("cleanup-plans: no plans under {}", root.display());
        return;
    }
    let targets = if wanted.is_empty() {
        all
    } else {
        let mut t = Vec::new();
        for name in wanted {
            let p = all
                .iter()
                .find(|p| p.file_name().and_then(|n| n.to_str()) == Some(&name))
                .cloned()
                .unwrap_or_else(|| {
                    die(format!("no plan named {name} under {}", root.display()), 66)
                });
            t.push(p)
        }
        t
    };
    if list {
        println!("Plans under {}:", root.display());
        for p in targets {
            if complete(&p) {
                println!(
                    "  ✅ {}  (completed)",
                    p.file_name().unwrap().to_string_lossy()
                )
            } else {
                println!("  💤 {}", p.file_name().unwrap().to_string_lossy())
            }
        }
        return;
    }
    println!("The following plans would be removed:");
    for p in &targets {
        let state = if complete(p) {
            "✅ completed"
        } else {
            "💤 incomplete"
        };
        println!("  {}  {}", state, p.file_name().unwrap().to_string_lossy())
    }
    if !yes {
        eprint!("Proceed? [y/N] ");
        let mut answer = String::new();
        if io::stdin().is_terminal() {
            io::stdin().read_line(&mut answer).ok();
        } else {
            io::stdin().read_to_string(&mut answer).ok();
        }
        if !matches!(answer.trim(), "y" | "Y" | "yes") {
            eprintln!("cleanup-plans: cancelled");
            std::process::exit(1)
        }
    }
    for p in &targets {
        let status = Command::new("remove-plan")
            .arg(p)
            .status()
            .unwrap_or_else(|e| die(e.to_string(), 73));
        if !status.success() {
            std::process::exit(status.code().unwrap_or(73))
        }
    }
    println!("cleanup-plans: removed {} plan(s)", targets.len());
    let _ = io::stdout().flush();
}
