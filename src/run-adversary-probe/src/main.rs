// MODE: DEV
// PACKAGE: PROD
//! Rust port of `run-adversary-probe.sh`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        usage(0);
    }
    if args.first().is_some_and(|arg| arg.starts_with('-')) {
        eprintln!("run-adversary-probe: unknown option: {}", args[0]);
        usage(64);
    }
    if args.len() > 1 {
        usage(64);
    }
    let script_dir = locate_script_dir();
    let fixture = script_dir.join("../tests/fixtures/adversary-probe");
    let working = args
        .first()
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("PLANNING_AGENT_TMPDIR")
                .map(PathBuf::from)
                .map(|path| path.join("adversary-probe"))
        })
        .unwrap_or_else(|| env::temp_dir().join("planning-agent/adversary-probe"));
    if !fixture.join("FIXTURE-VERSION").is_file() {
        eprintln!("probe fixture missing: {}", fixture.display());
        std::process::exit(66);
    }
    if working.exists() {
        fs::remove_dir_all(&working).unwrap_or_else(|error| fail_io(&working, error));
    }
    copy_tree(&fixture, &working);
    let _ = fs::remove_file(working.join("FIXTURE-VERSION"));
    let _ = fs::remove_file(working.join("README.md"));
    let reader = sibling("plan-context");
    run_reader(&reader, &working, &["init", "--plan-dir"], true);
    let mut failed = false;
    println!("=== gated-reader sanity check on materialized probe ===");
    for document in ["plan", "inventory", "progress", "adversarial-review"] {
        failed |= !read_document(&reader, &working, document);
    }
    for entry in fs::read_dir(&working)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
    {
        let id = entry.file_name().to_string_lossy().into_owned();
        if id != "context" {
            failed |= !read_document(&reader, &working, &format!("goal:{id}"));
        }
    }
    let inventory_path = working.join("work-unit-inventory.md");
    for unit in inventory_unit_ids(&inventory_path) {
        failed |= !read_unit(&reader, &working, &unit);
    }
    if !fs::read_to_string(working.join("adversarial-review.md"))
        .unwrap_or_default()
        .lines()
        .any(|line| line == "- Status: pending")
    {
        eprintln!("  FAIL: fixture is not a reusable pending stub");
        failed = true;
    }
    if failed {
        eprintln!("probe fixture is not usable with the current reader (update it; no backwards compatibility)");
        std::process::exit(1);
    }
    println!();
    println!("=== spawn a fresh adversarial reviewer with this starting prompt ===");
    println!("You are a fresh, independent adversarial reviewer for a benchmark plan. You are NOT the author of the plan.\n\nYOUR PERSONA: ROLE_ID=chris (oriented scout). Adopt this identity: load your scoped role docs and voice via\n  ROLE_ID=chris bash {}/role-context.sh chris\n(which injects your stance preamble). State your persona id in your self-report. A fresh adversary forms its own findings.\n\nTHE REQUEST the plan must satisfy:\n\"Add a GET /health endpoint to the example service that returns {{\\\"status\\\":\\\"ok\\\"}} with HTTP 200.\"\n\nTHE PLAN to review is at exactly:\n  {}\n\nMANDATORY READING RULES:\n1. SKILL-LOCK: Do not load any skill on your own. Use only what is named in this prompt. Do not infer skills from paths like .plans/.\n2. BOUNDED-READ: Read plan files and artifacts ONLY through the gated reader:\n     bash {}/plan-context.sh read --plan-dir {} --document ID\n     bash {}/plan-context.sh read --plan-dir {} --unit WNN\n   Valid --document IDs: plan, inventory, progress, adversarial-review, goal:<goal id>, step:<goal>/<step>. Use the default summary view; raise --max-records/--max-bytes for a large view if needed. Never load a whole plan file, the whole plan directory, or the .plans/ tree wholesale (no Read/cat/find on plan artifacts). If the gate cannot give you something, report it as a limitation — do not bypass it.\n\nYOUR TASK:\n1. Read the plan through the gate: --document plan, inventory, progress, goal:<each goal>, and --unit <each WNN>.\n2. Identify unplanned files, symbols, behaviors, tests, dependencies, and verification gaps for the request.\n3. Write findings + a verdict (✅ approved / rejected) to {}/adversarial-review-incoming.md.\n   That is the ONLY file you may write. Never edit adversarial-review.md — the\n   maintainer's update-adversarial-review.sh consumes your incoming file, mints\n   the fix keys and archives the result.\n\nBEHAVIOR SELF-REPORT (the most important output — be precise):\n- For every read: the exact command used and what it returned (or an error).\n- Which entry ids the gate served successfully; any it refused.\n- Did you read any plan artifact wholesale (Read/cat)? List exactly which.\n- Any friction or missing capability, and what you did instead.", script_dir.display(), working.display(), script_dir.display(), working.display(), script_dir.display(), working.display(), working.display());
    println!();
    println!(
        "Working probe: {} (reviewer writes its verdict there; committed fixture untouched)",
        working.display()
    );
}

fn usage(code: i32) -> ! {
    println!("Usage: run-adversary-probe [<working-dir>]\n       run-adversary-probe --help");
    std::process::exit(code);
}
fn locate_script_dir() -> PathBuf {
    env::var_os("PLANNING_SKILL_ROOT")
        .map(|path| PathBuf::from(path).join("scripts"))
        .unwrap_or_else(|| {
            env::current_dir()
                .unwrap_or_default()
                .join("planning/scripts")
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from("planning/scripts"))
        })
}
fn sibling(name: &str) -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}
fn run_reader(reader: &Path, working: &Path, args: &[&str], quiet: bool) {
    let mut command = Command::new(reader);
    command.args(args).arg(working);
    if quiet {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }
    if !command.status().is_ok_and(|status| status.success()) {
        eprintln!("gated reader failed");
        std::process::exit(1);
    }
}
fn read_document(reader: &Path, working: &Path, id: &str) -> bool {
    let ok = Command::new(reader)
        .args(["read", "--plan-dir"])
        .arg(working)
        .args(["--document", id])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if ok {
        println!("  gate serves --document {id}");
    } else {
        eprintln!("  FAIL: gate does not serve --document {id}");
    }
    ok
}
fn read_unit(reader: &Path, working: &Path, id: &str) -> bool {
    let ok = Command::new(reader)
        .args(["read", "--plan-dir"])
        .arg(working)
        .args(["--unit", id])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if ok {
        println!("  gate serves --unit {id}");
    } else {
        eprintln!("  FAIL: gate does not serve --unit {id}");
    }
    ok
}
fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap_or_else(|error| fail_io(target, error));
    for entry in fs::read_dir(source)
        .unwrap_or_else(|error| panic!("{}: {error}", source.display()))
        .flatten()
    {
        let from = entry.path();
        let to = target.join(entry.file_name());
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(&from, &to).unwrap_or_else(|error| fail_io(&to, error));
        }
    }
}
fn fail_io(path: &Path, error: std::io::Error) -> ! {
    eprintln!("{}: {error}", path.display());
    std::process::exit(1);
}
fn inventory_unit_ids(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let cell = line.split('|').nth(1)?.trim();
            (cell.starts_with('W') && cell[1..].bytes().all(|byte| byte.is_ascii_digit()))
                .then(|| cell.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::copy_tree;
    use std::fs;

    #[test]
    fn copies_dotfiles_and_nested_fixture_content() {
        let root = std::env::temp_dir().join(format!("adversary-probe-{}", std::process::id()));
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join(".hidden"), "hidden\n").unwrap();
        fs::write(source.join("nested/file"), "content\n").unwrap();
        copy_tree(&source, &target);
        assert_eq!(
            fs::read_to_string(target.join(".hidden")).unwrap(),
            "hidden\n"
        );
        assert_eq!(
            fs::read_to_string(target.join("nested/file")).unwrap(),
            "content\n"
        );
        let _ = fs::remove_dir_all(root);
    }
}
