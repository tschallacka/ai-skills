// MODE: DEV
// PACKAGE: PROD

//! Reproduces planning/tests/test-mermaid-accuracy.sh's exact observable
//! behavior (T145 goal 26): four mechanical checks over the tracked mermaid
//! documents, plus the mmdc render check. No arguments: the bash original
//! defines no usage()/argument parsing at all, so this binary takes none
//! either and silently ignores any argv it is given, matching that.

mod dirty;
mod discovery;
mod identifiers;
mod mermaid;
mod references;
mod render;

use mermaid::Severity;
use std::path::{Path, PathBuf};

const TRACKED_DOCS: [&str; 4] = [
    "planning/ARCHITECTURE.md",
    "benchmark/planning/ARCHITECTURE.md",
    "brainstorm/SKILL.md",
    "post-implementation-review/SKILL.md",
];

/// PLANNING_SKILL_ROOT first, current_exe()-anchored ancestor search as
/// fallback -- mirrors verify-both-shells' own discover_repo_root exactly.
fn discover_repo_root() -> Result<PathBuf, String> {
    if let Ok(root) = std::env::var("PLANNING_SKILL_ROOT") {
        if !root.is_empty() {
            return Ok(PathBuf::from(root));
        }
    }
    let self_path =
        std::env::current_exe().unwrap_or_else(|_| PathBuf::from("test-mermaid-accuracy"));
    let mut dir = self_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        if dir.join("planning/scripts").is_dir() {
            return Ok(dir);
        }
        let Some(parent) = dir.parent() else {
            return Err(format!(
                "could not locate the repository root from {}",
                self_path.display()
            ));
        };
        dir = parent.to_path_buf();
    }
}

fn count_fences(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("```mermaid"))
        .count()
}

fn is_fence_open(line: &str) -> bool {
    match line.strip_prefix("```mermaid") {
        Some(rest) => rest.chars().all(|c| c == ' ' || c == '\t'),
        None => false,
    }
}

fn extract_diagrams(repo_root: &Path, work: &Path) -> Vec<render::Diagram> {
    let mmd_dir = work.join("mmd");
    let _ = std::fs::create_dir_all(&mmd_dir);
    let mut diagrams = Vec::new();
    for doc in TRACKED_DOCS {
        let tag = doc.replace('/', "-");
        let text = std::fs::read_to_string(repo_root.join(doc)).unwrap_or_default();
        let mut inb = false;
        let mut current: Option<(usize, String)> = None;
        for (idx, line) in text.lines().enumerate() {
            let lineno = idx + 1;
            if !inb && is_fence_open(line) {
                inb = true;
                current = Some((lineno, String::new()));
                continue;
            }
            if inb && line.starts_with("```") {
                inb = false;
                if let Some((start, content)) = current.take() {
                    let filename = mmd_dir.join(format!("{tag}@{start}.mmd"));
                    let _ = std::fs::write(&filename, &content);
                    let is_sequence = content
                        .lines()
                        .next()
                        .map(|l| l.trim() == "sequenceDiagram")
                        .unwrap_or(false);
                    diagrams.push(render::Diagram {
                        where_: format!("{tag}:{start}"),
                        path: filename,
                        is_sequence,
                    });
                }
                continue;
            }
            if inb {
                if let Some((_, content)) = current.as_mut() {
                    content.push_str(line);
                    content.push('\n');
                }
            }
        }
    }
    // Matches `for f in "$work"/mmd/*.mmd`'s own alphabetical glob order.
    diagrams.sort_by(|a, b| a.path.cmp(&b.path));
    diagrams
}

fn group_done(baseline: u32, current: u32, label: &str) {
    if current == baseline {
        println!("test-mermaid-accuracy: {label}: PASS");
    } else {
        println!(
            "test-mermaid-accuracy: {label}: FAIL ({} finding(s))",
            current - baseline
        );
    }
}

fn emit(sev: Severity, message: &str, failures: &mut u32) {
    match sev {
        Severity::Fail => {
            eprintln!("mermaid-accuracy: FAIL: {message}");
            *failures += 1;
        }
        Severity::Warn => {
            eprintln!("mermaid-accuracy: WARN: {message}");
        }
    }
}

fn main() {
    let repo_root = discover_repo_root().unwrap_or_else(|e| {
        eprintln!("test-mermaid-accuracy: {e}");
        std::process::exit(70);
    });

    if let Some(reason) = dirty::dirty_reason(&repo_root) {
        eprintln!("test-mermaid-accuracy.sh: {reason}");
        std::process::exit(70);
    }

    let mut failures = 0u32;

    let self_path = repo_root.join("planning/tests/test-mermaid-accuracy.sh");
    let tracked_paths: Vec<PathBuf> = TRACKED_DOCS.iter().map(|d| repo_root.join(d)).collect();
    let tracked_refs: Vec<&Path> = tracked_paths.iter().map(|p| p.as_path()).collect();
    let corpora = discovery::build_corpora(&repo_root, &self_path, &tracked_refs);

    // ---- Check 1: structure ----------------------------------------------
    let baseline = failures;
    let mut per_doc: Vec<(String, mermaid::ParseResult, String)> = Vec::new();
    for doc in TRACKED_DOCS {
        let text = std::fs::read_to_string(repo_root.join(doc)).unwrap_or_default();
        let result = mermaid::parse_document(&text);
        per_doc.push((doc.to_string(), result, text));
    }
    for (relpath, result, _) in &per_doc {
        for f in &result.findings {
            let message = format!("{relpath}:{}: {}", f.line, f.message);
            emit(f.severity, &message, &mut failures);
        }
    }
    let mut parsed_total = 0usize;
    let mut fences_total = 0usize;
    for (relpath, result, _) in &per_doc {
        let fences = count_fences(&repo_root.join(relpath));
        fences_total += fences;
        if fences == 0 {
            emit(
                Severity::Fail,
                &format!("{relpath} has no mermaid diagram"),
                &mut failures,
            );
        }
        parsed_total += result.blocks;
    }
    if parsed_total != fences_total {
        emit(
            Severity::Fail,
            &format!(
                "parsed {parsed_total} mermaid blocks but the documents open {fences_total} fences"
            ),
            &mut failures,
        );
    }
    group_done(
        baseline,
        failures,
        &format!("structure of {parsed_total} mermaid blocks"),
    );

    // ---- Checks 2 and 3: named scripts and artifacts ----------------------
    let baseline = failures;
    let tokens_by_doc: Vec<(String, Vec<mermaid::Token>)> = per_doc
        .iter()
        .map(|(d, r, _)| (d.clone(), r.tokens.clone()))
        .collect();
    let refs = references::check_references(&tokens_by_doc, &corpora);
    for (sev, msg) in &refs.items {
        emit(*sev, msg, &mut failures);
    }
    group_done(
        baseline,
        failures,
        &format!(
            "{} scripts and {} artifacts named in diagrams",
            refs.scripts_seen, refs.artifacts_seen
        ),
    );

    // ---- Check 4: functions and identifiers --------------------------------
    let baseline = failures;
    let docs_text: Vec<(String, String)> = per_doc
        .iter()
        .map(|(d, _, t)| (d.clone(), t.clone()))
        .collect();
    let ids = identifiers::check_identifiers(&docs_text, &corpora.script_text);
    for (sev, msg) in &ids.items {
        emit(*sev, msg, &mut failures);
    }
    group_done(
        baseline,
        failures,
        &format!("{} function names in the diagram documents", ids.funcs_seen),
    );

    // ---- Render check -------------------------------------------------------
    let baseline = failures;
    let work = std::env::temp_dir().join(format!("mermaid-accuracy-render.{}", std::process::id()));
    let _ = std::fs::create_dir_all(&work);
    let diagrams = extract_diagrams(&repo_root, &work);
    match render::run(&work, &diagrams) {
        render::Outcome::Unconfigured => {
            eprintln!(
                "test-mermaid-accuracy: UNCONFIGURED (mmdc) \u{2014} the structural checks ran, mermaid syntax itself is unverified; `nix develop` provides mmdc, and the mermaid-render CI job always runs it"
            );
        }
        render::Outcome::BrowserUnconfigured => {
            eprintln!(
                "test-mermaid-accuracy: UNCONFIGURED (browser) \u{2014} mmdc cannot launch even for a control flowchart here; the structural checks ran, mermaid syntax itself is unverified; `mermaid-render` CI always runs it"
            );
            group_done(baseline, failures, "mmdc rendered 0 diagrams");
        }
        render::Outcome::Rendered {
            rendered,
            sequence_skew_warning,
            findings,
            fail_findings,
        } => {
            if let Some(w) = sequence_skew_warning {
                eprintln!("{w}");
            }
            for f in &findings {
                eprintln!("{f}");
            }
            for f in &fail_findings {
                emit(Severity::Fail, f, &mut failures);
            }
            group_done(
                baseline,
                failures,
                &format!("mmdc rendered {rendered} diagrams"),
            );
        }
    }
    let _ = std::fs::remove_dir_all(&work);

    if failures != 0 {
        std::process::exit(1);
    }
    println!("test-mermaid-accuracy: PASS");
}
