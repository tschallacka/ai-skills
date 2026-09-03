// MODE: DEV
// PACKAGE: PROD
use planning_map::PlanningMap;
use planning_table::{table_cell, table_cells};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const COMMAND: &str = "plan-content.sh";

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code);
}

fn usage(code: i32) -> ! {
    print!(
        "Usage:\n  {COMMAND} get [--plan-dir] <plan-directory> <document-id> [markdown|text|json|path]\n  {COMMAND} summary [--plan-dir] <plan-directory> [markdown|text|json]\n  {COMMAND} blast-radius [--plan-dir] <plan-directory> <WNN|goal-name|goal-name/step-name> [markdown|text|json]\n  {COMMAND} find [--plan-dir] <plan-directory> <pattern> [--in plan|goals|steps|units|review|testing|coverage|stories|all] [--document <docid>] [--full] [--format text|json]\n                                    literal search; prints docid<TAB>section<TAB>excerpt per match,\n                                    exits 1 on zero or multiple matches; --document scopes to one document\n                                    (plan, review, coverage, stories, planning-bugs, goal:<g>, step:<g>/<s>, unit:<WNN>, or a\n                                    step:-testing id); --full disables excerpt truncation\n  {COMMAND} diff [--plan-dir] <plan-directory> <git-ref> [--format text|json]\n                                    lists documents changed since git-ref and the\n                                    paragraph labels touched in each\n"
    );
    std::process::exit(code);
}

fn require_dir(path: &Path) {
    if !path.is_dir() {
        die(format!("Plan directory not found: {}", path.display()), 66);
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| die(error.to_string(), 66))
}

fn json_string(value: &str) -> String {
    let mut result = String::from("\"");
    let mut first = true;
    for line in value.lines() {
        if !first {
            result.push_str("\\n");
        }
        first = false;
        for character in line.chars() {
            match character {
                '\\' => result.push_str("\\\\"),
                '"' => result.push_str("\\\""),
                character => result.push(character),
            }
        }
    }
    result.push('"');
    result
}

fn document_path(plan: &Path, id: &str) -> PathBuf {
    match id {
        "plan" => plan.join("plan-description.md"),
        "adversarial-review" => plan.join("adversarial-review.md"),
        "coverage" | "inventory" => plan.join("work-unit-inventory.md"),
        "progress" => plan.join("progress.md"),
        "stories" => plan.join("ui-user-stories.md"),
        "bugs" => plan.join("bugs.md"),
        "planning-bugs" => plan.join("planning-bugs.json"),
        "fixes" => plan.join("fixes.md"),
        "fix-keys" | "fixkeys" => plan.join("fix-keys.json"),
        "approval" => plan.join("approval.json"),
        value if value.starts_with("goal-progress:") => {
            let goal = value.strip_prefix("goal-progress:").unwrap();
            if goal.is_empty() {
                die("Goal progress IDs use goal-progress:<goal>", 64);
            }
            plan.join(goal).join("progress.md")
        }
        value if value.starts_with("goal:") => plan.join(value.strip_prefix("goal:").unwrap()).join("goal.md"),
        value if value.starts_with("step:") => {
            let rest = value.strip_prefix("step:").unwrap();
            let Some((goal, step)) = rest.split_once('/') else { die("Step document IDs use step:<goal>/<step>", 64); };
            if goal.is_empty() || step.is_empty() { die("Step document IDs use step:<goal>/<step>", 64); }
            plan.join(goal).join("steps").join(format!("{step}.md"))
        }
        value if value.starts_with("unit:W") => {
            let unit = value.strip_prefix("unit:").unwrap();
            let row = inventory_rows(&plan.join("work-unit-inventory.md")).into_iter().find(|row| row.id == unit)
                .unwrap_or_else(|| die(format!("Work unit not found: {unit}"), 64));
            plan.join(row.goal).join("steps").join(format!("{}.md", row.step))
        }
        value => die(format!("Unknown document ID: {value} (use plan, adversarial-review, goal:<goal>, goal-progress:<goal>, step:<goal>/<step>, unit:<WNN>, coverage, inventory, progress, stories, bugs, planning-bugs, fixes, fix-keys, or approval)"), 64),
    }
}

#[derive(Clone)]
struct Row {
    id: String,
    kind: String,
    file: String,
    scope: String,
    depends: String,
    goal: String,
    step: String,
}

fn inventory_rows(path: &Path) -> Vec<Row> {
    if !path.is_file() {
        die(
            format!("Work-unit inventory not found: {}", path.display()),
            64,
        );
    }
    read(path)
        .lines()
        .filter(|line| line.starts_with("| W"))
        .filter_map(|line| {
            let cells = table_cells(line);
            (cells.len() >= 9).then(|| Row {
                id: table_cell(line, 2),
                kind: table_cell(line, 3),
                file: table_cell(line, 4),
                scope: table_cell(line, 5),
                depends: table_cell(line, 8),
                goal: table_cell(line, 9),
                step: table_cell(line, 10),
            })
        })
        .collect()
}

fn hoist(args: &[String]) -> (PathBuf, Vec<String>) {
    let mut remaining = Vec::new();
    let mut plan = None;
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--plan-dir" {
            plan = args.get(index + 1).map(PathBuf::from);
            index += 2;
        } else if let Some(value) = args[index].strip_prefix("--plan-dir=") {
            plan = Some(PathBuf::from(value));
            index += 1;
        } else {
            remaining.push(args[index].clone());
            index += 1;
        }
    }
    if let Some(plan) = plan {
        return (plan, remaining);
    }
    let Some(value) = remaining.first().cloned() else {
        usage(64);
    };
    (PathBuf::from(value), remaining[1..].to_vec())
}

fn format_document(plan: &Path, id: &str, format: &str) {
    let path = document_path(plan, id);
    if !path.is_file() {
        die(format!("Document not found: {}", path.display()), 66);
    }
    let content = read(&path);
    match format {
        "markdown" => print!("{content}"),
        "text" => print!("Document: {id}\nPath: {}\n\n{content}", path.display()),
        "path" => println!("{}", path.display()),
        "json" => println!(
            "{{\"id\":\"{id}\",\"path\":\"{}\",\"content\":{}}}",
            path.display(),
            json_string(&content)
        ),
        value => die(
            format!("Unknown format: {value} (use markdown, text, json, or path)"),
            64,
        ),
    }
}

fn summary(plan: &Path, format: &str) {
    let rows = inventory_rows(&plan.join("work-unit-inventory.md"));
    match format {
        "markdown" => {
            println!(
                "# Plan summary: {}\n",
                plan.file_name().unwrap().to_string_lossy()
            );
            println!("| ID | Type | File | Scope | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|");
            for row in rows {
                println!(
                    "| {} | {} | {} | {} | {} | {} | {} |",
                    row.id, row.kind, row.file, row.scope, row.depends, row.goal, row.step
                );
            }
        }
        "text" => {
            for row in rows {
                println!(
                    "{}  {}  {} :: {}  <- {}  [{}/{}]",
                    row.id, row.kind, row.file, row.scope, row.depends, row.goal, row.step
                );
            }
        }
        "json" => {
            print!(
                "{{\"plan\":\"{}\",\"work_units\":[",
                plan.file_name().unwrap().to_string_lossy()
            );
            for (index, row) in rows.iter().enumerate() {
                if index > 0 {
                    print!(",");
                }
                print!("{{\"id\":\"{}\",\"type\":\"{}\",\"file\":\"{}\",\"scope\":\"{}\",\"depends_on\":\"{}\",\"goal\":\"{}\",\"step\":\"{}\"}}", row.id, row.kind, row.file, row.scope, row.depends, row.goal, row.step);
            }
            println!("]}}");
        }
        value => die(
            format!("Unknown format: {value} (use markdown, text, or json)"),
            64,
        ),
    }
}

fn blast_radius(plan: &Path, target: &str, format: &str) {
    let rows = inventory_rows(&plan.join("work-unit-inventory.md"));
    let program = env::args().next().unwrap_or_else(|| COMMAND.into());
    eprintln!("{COMMAND}: blast-radius walks dependency edges. To sweep every document mentioning this unit, run: {program} find --plan-dir <plan-directory> \"{target}\" --in all");
    let by_id: HashMap<_, _> = rows.iter().map(|row| (row.id.clone(), row)).collect();
    let mut selected = Vec::new();
    if target.starts_with('W') {
        if !by_id.contains_key(target) {
            die(format!("Work unit not found: {target}"), 64);
        }
        selected.push(target.to_string());
    } else if let Some((goal, step)) = target.split_once('/') {
        selected.extend(
            rows.iter()
                .filter(|row| row.goal == goal && row.step == step)
                .map(|row| row.id.clone()),
        );
        if selected.is_empty() {
            die(format!("Step not found: {target}"), 64);
        }
    } else {
        selected.extend(
            rows.iter()
                .filter(|row| row.goal == target)
                .map(|row| row.id.clone()),
        );
        if selected.is_empty() {
            die(format!("Goal not found: {target}"), 64);
        }
    }
    let mut selected_set = PlanningMap::default();
    for id in &selected {
        selected_set.set(id, "1");
    }
    let mut impacted = PlanningMap::default();
    loop {
        let mut changed = false;
        for row in &rows {
            if selected_set.has(&row.id) || impacted.has(&row.id) {
                continue;
            }
            if row
                .depends
                .split(',')
                .map(str::trim)
                .any(|dep| selected_set.has(dep) || impacted.has(dep))
                && !impacted.has(&row.id)
            {
                impacted.set(row.id.clone(), "1");
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    selected.sort();
    let mut impacted: Vec<_> = impacted.keys().map(str::to_owned).collect();
    impacted.sort();
    match format {
        "markdown" => {
            println!("# Blast radius: {target}\n\n## Changed\n");
            for id in &selected {
                let row = by_id[id];
                println!("- \\x60{id}\\x60 → \\x60{}/{}\\x60", row.goal, row.step);
            }
            println!("\n## Downstream work units\n");
            if impacted.is_empty() {
                println!("- None");
            } else {
                for id in &impacted {
                    let row = by_id[id];
                    println!("- \\x60{id}\\x60 → \\x60{}/{}\\x60", row.goal, row.step);
                }
            }
        }
        "text" => {
            for id in &selected {
                let row = by_id[id];
                println!("changed {id} -> {}/{}", row.goal, row.step);
            }
            for id in &impacted {
                let row = by_id[id];
                println!("downstream {id} -> {}/{}", row.goal, row.step);
            }
        }
        "json" => {
            print!("{{\"target\":\"{target}\",\"changed\":[");
            for (index, id) in selected.iter().enumerate() {
                if index > 0 {
                    print!(",");
                }
                print!("\"{id}\"");
            }
            print!("],\"downstream\":[");
            for (index, id) in impacted.iter().enumerate() {
                if index > 0 {
                    print!(",");
                }
                print!("\"{id}\"");
            }
            println!("]}}");
        }
        value => die(
            format!("Unknown format: {value} (use markdown, text, or json)"),
            70,
        ),
    }
}

struct Match {
    document: String,
    section: String,
    excerpt: String,
}

fn excerpt(line: &str, full: bool) -> String {
    let line = line.trim_start().to_string();
    if !full && line.len() > 120 {
        let mut value = line.as_bytes()[..120].to_vec();
        value.extend_from_slice(b"...");
        return String::from_utf8_lossy(&value).into_owned();
    }
    line
}

fn scan_file(document: &str, path: &Path, pattern: &str, full: bool) -> Vec<Match> {
    if !path.is_file() {
        return Vec::new();
    }
    let mut section = String::from("-");
    read(path)
        .lines()
        .filter_map(|line| {
            if let Some(label) = line.strip_prefix("§ ") {
                if label.split_once('.').is_some_and(|(a, b)| {
                    !a.is_empty()
                        && !b.is_empty()
                        && a.chars().all(|c| c.is_ascii_digit())
                        && b.chars().all(|c| c.is_ascii_digit())
                }) {
                    section = label.to_string();
                }
            }
            line.contains(pattern).then(|| Match {
                document: document.to_string(),
                section: section.clone(),
                excerpt: excerpt(line, full),
            })
        })
        .collect()
}

fn scan_inventory(path: &Path, pattern: &str, full: bool) -> Vec<Match> {
    if !path.is_file() {
        return Vec::new();
    }
    read(path)
        .lines()
        .filter(|line| line.starts_with("| W") && (pattern.is_empty() || line.contains(pattern)))
        .map(|line| {
            let id = table_cell(line, 2);
            let raw = line
                .trim_start_matches(|c: char| c.is_ascii_whitespace() || c == '|')
                .trim_end_matches(|c: char| c.is_ascii_whitespace() || c == '|')
                .to_string();
            let row = excerpt(&raw, full);
            Match {
                document: format!("unit:{id}"),
                section: row.clone(),
                excerpt: row,
            }
        })
        .collect()
}

fn scan_coverage(path: &Path, pattern: &str, full: bool) -> Vec<Match> {
    if !path.is_file() {
        return Vec::new();
    }
    let mut inside = false;
    let mut matches = Vec::new();
    for line in read(path).lines() {
        if line.starts_with("## Definition-of-done coverage") {
            inside = true;
            continue;
        }
        if inside && line.starts_with("## ") {
            break;
        }
        if inside && line.starts_with('|') && (pattern.is_empty() || line.contains(pattern)) {
            let raw = line
                .trim_start_matches(|c: char| c.is_ascii_whitespace() || c == '|')
                .trim_end_matches(|c: char| c.is_ascii_whitespace() || c == '|')
                .to_string();
            let row = excerpt(&raw, full);
            matches.push(Match {
                document: "coverage".into(),
                section: row.clone(),
                excerpt: row,
            });
        }
    }
    matches
}

fn scan_stories(path: &Path, pattern: &str, full: bool) -> Vec<Match> {
    if !path.is_file() {
        return Vec::new();
    }
    let mut section = String::new();
    read(path)
        .lines()
        .filter_map(|line| {
            if line.starts_with("## ") {
                section = line.to_string();
            }
            (line.contains(pattern) && !line.starts_with("# ")).then(|| Match {
                document: "stories".into(),
                section: if section.is_empty() {
                    "-".into()
                } else {
                    section.clone()
                },
                excerpt: excerpt(line, full),
            })
        })
        .collect()
}

fn all_documents(plan: &Path) -> Vec<(String, PathBuf)> {
    let mut documents = vec![
        ("plan".into(), plan.join("plan-description.md")),
        ("review".into(), plan.join("adversarial-review.md")),
    ];
    let mut goals = Vec::new();
    if let Ok(entries) = fs::read_dir(plan) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("goal.md").is_file() {
                goals.push(path);
            }
        }
    }
    goals.sort();
    for goal in goals {
        let name = goal.file_name().unwrap().to_string_lossy().into_owned();
        documents.push((format!("goal:{name}"), goal.join("goal.md")));
        let steps = goal.join("steps");
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&steps) {
            files.extend(entries.flatten().map(|entry| entry.path()));
        }
        files.sort();
        for path in files {
            if path.extension().is_some_and(|extension| extension == "md") {
                let step = path.file_stem().unwrap().to_string_lossy();
                documents.push((format!("step:{name}/{step}"), path));
            }
        }
    }
    documents.push(("stories".into(), plan.join("ui-user-stories.md")));
    documents
}

fn output_matches(matches: &[Match], format: &str) {
    if format == "json" {
        print!("{{\"matches\":[");
        for (index, item) in matches.iter().enumerate() {
            if index > 0 {
                print!(",");
            }
            let document = json_string(&item.document);
            let section = json_string(&item.section);
            let excerpt = json_string(&item.excerpt);
            print!(
                "{{\"document\":{},\"section\":{},\"excerpt\":{}}}",
                document, section, excerpt
            );
        }
        println!("]}}");
    } else {
        for item in matches {
            println!("{}\t{}\t{}", item.document, item.section, item.excerpt);
        }
    }
}

fn finish_find(count: usize, pattern: &str, label: &str) -> ! {
    if count == 0 {
        eprintln!("{COMMAND}: find: no matches for {pattern} ({label})");
        std::process::exit(1);
    }
    if count > 1 {
        let suffix = if label.starts_with("document:") {
            " to get a single hit"
        } else {
            " or scope to get a single hit"
        };
        eprintln!(
            "{COMMAND}: find: {count} matches for {pattern} ({label}); narrow the pattern{suffix}"
        );
        std::process::exit(1);
    }
    std::process::exit(0);
}

fn find_command(plan: &Path, args: &[String]) {
    if args.is_empty() {
        usage(64);
    }
    let pattern = &args[0];
    let mut scope = "all";
    let mut format = "text";
    let mut document = None;
    let mut full = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--in" if index + 1 < args.len() => {
                scope = &args[index + 1];
                index += 2;
            }
            "--format" if index + 1 < args.len() => {
                format = &args[index + 1];
                index += 2;
            }
            "--document" if index + 1 < args.len() => {
                document = Some(args[index + 1].clone());
                index += 2;
            }
            "--full" => {
                full = true;
                index += 1;
            }
            _ => usage(64),
        }
    }
    if !matches!(
        scope,
        "plan"
            | "goals"
            | "steps"
            | "units"
            | "inventory"
            | "review"
            | "testing"
            | "coverage"
            | "stories"
            | "all"
    ) {
        die(
            format!(
                "Unknown scope: {scope} (use plan, goals, steps, units, review, testing, coverage, stories, inventory, or all)"
            ),
            64,
        );
    }
    if !matches!(format, "text" | "json") {
        die(format!("Unknown format: {format} (use text or json)"), 64);
    }
    let mut results = Vec::new();
    if let Some(id) = document {
        if scope != "all" {
            die("--document and --in are mutually exclusive", 64);
        }
        let path = document_path(plan, &id);
        if !path.is_file() {
            die(format!("document not found: {}", path.display()), 64);
        }
        if id == "coverage" {
            results = scan_coverage(&path, pattern, full);
        } else {
            results = scan_file(&id, &path, pattern, full);
        }
        output_matches(&results, format);
        finish_find(results.len(), pattern, &format!("document: {id}"));
    }
    if matches!(scope, "plan" | "all") {
        results.extend(scan_file(
            "plan",
            &plan.join("plan-description.md"),
            pattern,
            full,
        ));
    }
    if matches!(scope, "review" | "all") {
        results.extend(scan_file(
            "review",
            &plan.join("adversarial-review.md"),
            pattern,
            full,
        ));
    }
    if matches!(scope, "goals" | "all") {
        for (id, path) in all_documents(plan)
            .into_iter()
            .filter(|(id, _)| id.starts_with("goal:"))
        {
            results.extend(scan_file(&id, &path, pattern, full));
        }
    }
    if matches!(scope, "steps" | "all") {
        for (id, path) in all_documents(plan)
            .into_iter()
            .filter(|(id, _)| id.starts_with("step:") && !id.ends_with("-testing"))
        {
            results.extend(scan_file(&id, &path, pattern, full));
        }
    }
    if matches!(scope, "testing" | "all") {
        for (id, path) in all_documents(plan)
            .into_iter()
            .filter(|(id, _)| id.starts_with("step:") && id.ends_with("-testing"))
        {
            results.extend(scan_file(&id, &path, pattern, full));
        }
    }
    if matches!(scope, "units" | "inventory" | "all") {
        results.extend(scan_inventory(
            &plan.join("work-unit-inventory.md"),
            pattern,
            full,
        ));
    }
    if matches!(scope, "coverage" | "all") {
        results.extend(scan_coverage(
            &plan.join("work-unit-inventory.md"),
            pattern,
            full,
        ));
    }
    if matches!(scope, "stories" | "all") {
        results.extend(scan_stories(
            &plan.join("ui-user-stories.md"),
            pattern,
            full,
        ));
    }
    output_matches(&results, format);
    finish_find(results.len(), pattern, &format!("scope: {scope}"));
}

fn diff_json_escape(value: &str) -> String {
    let mut result = String::new();
    for character in value.chars() {
        match character {
            '§' => result.push_str("\\u00a7"),
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            character => result.push(character),
        }
    }
    result
}

fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn paragraph_labels(
    repo: &Path,
    plan: &Path,
    plan_rel: &str,
    reference: &str,
    document: &str,
) -> Vec<String> {
    let path = if plan_rel == "." {
        document.to_string()
    } else {
        format!("{plan_rel}/{document}")
    };
    let Some(diff) = git_output(repo, &["diff", "-U0", reference, "--", &path]) else {
        return Vec::new();
    };
    let mut changed_lines = Vec::new();
    let mut new_line = 0usize;
    let mut in_hunk = false;
    for line in diff.lines() {
        if line.starts_with("@@") {
            let Some(plus) = line.split_whitespace().find(|part| part.starts_with('+')) else {
                in_hunk = false;
                continue;
            };
            let number = plus
                .trim_start_matches('+')
                .split(',')
                .next()
                .unwrap_or_default();
            new_line = number.parse().unwrap_or(0);
            in_hunk = true;
            continue;
        }
        if !in_hunk {
            continue;
        }
        let lead = line.as_bytes().first().copied().unwrap_or_default();
        if lead == b'+' && !line.starts_with("+++") {
            changed_lines.push(new_line);
        }
        if lead != b'-' {
            new_line += 1;
        }
    }
    let document_path = plan.join(document);
    let Ok(content) = fs::read_to_string(document_path) else {
        return Vec::new();
    };
    let mut current = String::new();
    let mut labels = Vec::new();
    let mut seen = HashSet::new();
    for (number, line) in content.lines().enumerate() {
        if let Some(label) = line.strip_prefix("§ ") {
            if label.split_once('.').is_some_and(|(a, b)| {
                !a.is_empty()
                    && !b.is_empty()
                    && a.chars().all(|c| c.is_ascii_digit())
                    && b.chars().all(|c| c.is_ascii_digit())
            }) {
                current = line.to_string();
            }
        }
        let line_number = number + 1;
        if changed_lines.contains(&line_number)
            && !current.is_empty()
            && seen.insert(current.clone())
        {
            labels.push(current.clone());
        }
    }
    labels
}

fn diff_command(plan: &Path, reference: &str, format: &str) {
    if !matches!(format, "text" | "json") {
        die(format!("Unknown format: {format} (use text or json)"), 64);
    }
    let Some(repo) = git_output(plan, &["rev-parse", "--show-toplevel"]) else {
        die(
            format!(
                "plan directory is not inside a git repository: {}",
                plan.display()
            ),
            64,
        );
    };
    let repo = PathBuf::from(repo.trim());
    let plan = fs::canonicalize(plan).unwrap_or_else(|error| die(error.to_string(), 66));
    let plan_rel = plan
        .strip_prefix(&repo)
        .unwrap_or(Path::new("."))
        .to_string_lossy()
        .into_owned();
    let plan_rel = if plan == repo { "." } else { plan_rel.as_str() };
    let Some(changed_output) =
        git_output(&repo, &["diff", "--name-only", reference, "--", plan_rel])
    else {
        if git_output(&repo, &["rev-parse", "-q", "--verify", reference]).is_none() {
            die(format!("git ref not found: {reference}"), 64);
        }
        if format == "json" {
            println!(
                "{{\"ref\": \"{}\", \"documents\": []}}",
                diff_json_escape(reference)
            );
        } else {
            println!("No plan documents changed since {reference}");
        }
        return;
    };
    let prefix = if plan_rel == "." {
        String::new()
    } else {
        format!("{plan_rel}/")
    };
    let changed: Vec<_> = changed_output
        .lines()
        .map(|line| line.strip_prefix(&prefix).unwrap_or(line))
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    if changed.is_empty() {
        if git_output(&repo, &["rev-parse", "-q", "--verify", reference]).is_none() {
            die(format!("git ref not found: {reference}"), 64);
        }
        if format == "json" {
            println!(
                "{{\"ref\": \"{}\", \"documents\": []}}",
                diff_json_escape(reference)
            );
        } else {
            println!("No plan documents changed since {reference}");
        }
        return;
    }
    if format == "json" {
        print!(
            "{{\"ref\": \"{}\", \"documents\": [",
            diff_json_escape(reference)
        );
        for (index, document) in changed.iter().enumerate() {
            if index > 0 {
                print!(", ");
            }
            print!(
                "{{\"document\": \"{}\", \"paragraphs\": [",
                diff_json_escape(document)
            );
            let labels = paragraph_labels(&repo, &plan, plan_rel, reference, document);
            for (label_index, label) in labels.iter().enumerate() {
                if label_index > 0 {
                    print!(", ");
                }
                print!("\"{}\"", diff_json_escape(label));
            }
            print!("]}}");
        }
        println!("]}}");
    } else {
        for document in changed {
            println!("## {document}");
            for label in paragraph_labels(&repo, &plan, plan_rel, reference, &document) {
                println!("{label}");
            }
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        usage(0);
    }
    let Some(command) = args.first().cloned() else {
        usage(64);
    };
    let (plan, rest) = hoist(&args[1..]);
    require_dir(&plan);
    match command.as_str() {
        "get" if (2..=3).contains(&rest.len()) => format_document(
            &plan,
            &rest[0],
            rest.get(1).map(String::as_str).unwrap_or("markdown"),
        ),
        "summary" if (0..=1).contains(&rest.len()) => summary(
            &plan,
            rest.first().map(String::as_str).unwrap_or("markdown"),
        ),
        "blast-radius" if (2..=3).contains(&rest.len()) => blast_radius(
            &plan,
            &rest[0],
            rest.get(1).map(String::as_str).unwrap_or("markdown"),
        ),
        "find" if !rest.is_empty() => find_command(&plan, &rest),
        "diff" if (1..=2).contains(&rest.len()) => diff_command(
            &plan,
            &rest[0],
            rest.get(1).map(String::as_str).unwrap_or("text"),
        ),
        _ => usage(64),
    }
}
