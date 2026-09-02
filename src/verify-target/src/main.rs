// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn usage(code: i32) -> ! {
    println!(
        "Usage: verify-target.sh [--plan-dir] <plan-directory> <WNN> [--repo <repository-root>]"
    );
    println!("       verify-target.sh --help");
    std::process::exit(code);
}
fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("verify-target: {}", message.as_ref());
    std::process::exit(code);
}
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk(&p, out)
        } else {
            out.push(p)
        }
    }
}
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--help" || a == "-h") {
        usage(0)
    }
    let mut plan = None;
    let mut unit = None;
    let mut repo = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--plan-dir" => {
                i += 1;
                plan = Some(args.get(i).cloned().unwrap_or_else(|| usage(64)))
            }
            "--repo" => {
                i += 1;
                repo = Some(args.get(i).cloned().unwrap_or_else(|| usage(64)))
            }
            "--" => {
                break;
            }
            v if v.starts_with('-') => usage(64),
            v => {
                if plan.is_none() {
                    plan = Some(v.to_string())
                } else if unit.is_none() {
                    unit = Some(v.to_string())
                } else {
                    usage(64)
                }
            }
        }
        i += 1
    }
    let plan = plan.unwrap_or_else(|| usage(64));
    let unit = unit.unwrap_or_else(|| usage(64));
    if !unit.starts_with('W')
        || unit[1..].len() < 2
        || !unit[1..].chars().all(|c| c.is_ascii_digit())
    {
        die("Work-unit ID must use W01", 64)
    }
    let plan = PathBuf::from(plan);
    if !plan.is_dir() {
        die(format!("directory not found: {}", plan.display()), 64)
    }
    let repo = repo
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap());
    if !repo.is_dir() {
        die(format!("repository root not found: {}", repo.display()), 64)
    }
    let inventory = plan.join("work-unit-inventory.md");
    if !inventory.is_file() {
        die(
            format!("work-unit inventory not found: {}", inventory.display()),
            64,
        )
    }
    let mut row = None;
    for line in fs::read_to_string(&inventory).unwrap().lines() {
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<_> = line.split('|').map(str::trim).collect();
        if cells.len() >= 7 && cells[1] == unit {
            row = Some((
                cells[2].trim_matches('`').to_string(),
                cells[3].trim_matches('`').to_string(),
                cells[4].trim_matches('`').to_string(),
                cells[5].trim_matches('`').to_string(),
            ));
            break;
        }
    }
    let (kind, file, scope, subscope) = row.unwrap_or_else(|| {
        die(
            format!("work unit {unit} not found in {}", inventory.display()),
            64,
        )
    });
    eprintln!("verify-target: {unit} ({kind}) file={file} scope={scope} subscope={subscope}");
    eprintln!("verify-target: repository={}", repo.display());
    if file.is_empty() || file == "N/A" {
        eprintln!("verify-target: {unit} has no target file recorded; no reachability check can run — record the target file, or do not claim reachability evidence for {unit}");
        eprintln!(
            "verify-target: FAIL {unit} — no target file recorded; no reachability check can run"
        );
        std::process::exit(1)
    }
    let mut issues = 0;
    let mut warnings = 0;
    let target = repo.join(&file);
    if target.is_file() {
        eprintln!("verify-target: OK target file exists: {}", target.display());
        let owner = if file.starts_with("vendor/") {
            "module (vendor)"
        } else if file.starts_with("app/code/") {
            "module (app/code)"
        } else if file.starts_with("app/design/") {
            "theme (app/design)"
        } else {
            "unclassified path (human check)"
        };
        eprintln!("verify-target: OK owner: {owner}")
    } else {
        eprintln!(
            "verify-target: FAIL target file does not exist: {}",
            target.display()
        );
        issues += 1
    }
    let surface = matches!(kind.as_str(), "markup" | "style")
        || [
            ".phtml", ".html", ".htm", ".twig", ".tpl", ".vue", ".svelte", ".jsx", ".tsx", ".xml",
            ".css", ".less", ".scss", ".sass", ".styl",
        ]
        .iter()
        .any(|x| file.ends_with(x));
    let mut files = Vec::new();
    walk(&repo, &mut files);
    let layouts: Vec<_> = files
        .iter()
        .filter(|p| {
            let s = p.to_string_lossy();
            p.file_name().and_then(|n| n.to_str()) == Some("view.xml")
                || (s.contains("/view/")
                    && s.contains("/layout/")
                    && p.extension().and_then(|x| x.to_str()) == Some("xml"))
        })
        .collect();
    let block = scope.trim_start_matches('#').trim_start_matches('.');
    if !surface {
        eprintln!("verify-target: OK {file} is not a render surface; checks 2-3 do not apply to it")
    } else if block.is_empty() || block == "N/A" {
        eprintln!("verify-target: FAIL render surface {file} has no block name in the unit's Scope column, so the remove/re-point checks cannot run — record the block the target renders through");
        issues += 1
    } else if layouts.is_empty() {
        eprintln!("verify-target: WARN no layout XML files found under {}; the remove/re-point checks could not run for block '{block}'",repo.display());
        warnings += 1
    } else {
        let mut removed = None;
        let mut repointed = None;
        for p in &layouts {
            let text = fs::read_to_string(p).unwrap_or_default();
            for line in text.lines() {
                if line.contains(&format!("name=\"{block}\"")) && line.contains("remove=\"true\"") {
                    removed = Some(line.to_string())
                }
                if line.contains(block)
                    && (line.contains("setTemplate") || line.contains("template=\""))
                {
                    repointed = Some(format!("{}: {}", p.display(), line.trim()))
                }
            }
        }
        if let Some(v) = removed {
            eprintln!("verify-target: FAIL a layout removes block '{block}' that renders this target: {v}");
            issues += 1
        } else {
            eprintln!("verify-target: OK no layout removes block '{block}'")
        }
        if let Some(v) = repointed {
            eprintln!("verify-target: WARN a layout re-points block '{block}': {v}");
            warnings += 1
        } else {
            eprintln!("verify-target: OK no layout re-points block '{block}'")
        }
    }
    if file.starts_with("app/code/") || file.starts_with("vendor/") {
        let base = Path::new(&file).file_name().unwrap().to_string_lossy();
        let mut overrides = Vec::new();
        for p in &files {
            if p.to_string_lossy().contains("/app/design/")
                && p.file_name()
                    .map(|n| n.to_string_lossy() == base)
                    .unwrap_or(false)
            {
                overrides.push(p)
            }
        }
        if overrides.is_empty() {
            eprintln!("verify-target: OK no theme override of {base} under app/design")
        } else {
            eprintln!("verify-target: WARN theme override of {base} exists (human check — the theme copy renders instead):");
            for p in overrides {
                eprintln!("verify-target:   {}", p.display())
            }
            warnings += 1
        }
    }
    if issues > 0 {
        eprintln!("verify-target: {issues} issue(s) found for {unit} — record this before planning against the target");
        std::process::exit(1)
    }
    let checks = if surface {
        "existence, layout remove/re-point, theme override"
    } else {
        "existence, theme override"
    };
    println!("verify-target: PASS ({warnings} warning(s)) — no static counter-evidence for {unit} target {file}; checks run: {checks}");
}
