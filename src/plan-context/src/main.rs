// MODE: DEV
// PACKAGE: PROD
use plan_context_core::{
    entry_hash, inventory_row_text, inventory_rows, resolve_document, view_text,
    CONTEXT_GENERATOR_VERSION, CONTEXT_RESULT_SCHEMA_VERSION, CONTEXT_SCHEMA_VERSION,
};
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const COMMAND: &str = "plan-context.sh";

fn usage(code: u8) -> ! {
    print!(
        "Usage:\n  {COMMAND} init --plan-dir DIR\n  {COMMAND} read --plan-dir DIR (--document ID | --unit WNN) [--view VIEW] [--token TOKEN] [--format text|json] [--max-bytes N] [--max-records N] [--read-only]\n  {COMMAND} check --plan-dir DIR (--entry ID | --changed | --all) [--format text|json]\n  {COMMAND} refresh --plan-dir DIR (--entry ID | --stale) [--format text|json]\n  {COMMAND} checkpoint --plan-dir DIR --phase PHASE --state STATE --findings-file FILE --changed-files FILE --source-hash HASH --plan-hash HASH\n\nValid --document IDs:\n  plan                 plan-description.md\n  inventory            work-unit-inventory.md\n  progress             progress.md\n  adversarial-review   adversarial-review.md\n  coverage             work-unit-inventory.md     (the coverage table)\n  stories              ui-user-stories.md\n  bugs                 bugs.md\n  planning-bugs        planning-bugs.json\n  fixes                fixes.md\n  fix-keys             fix-keys.json\n  approval             approval.json\n  goal-progress:<goal> <goal>/progress.md\n  goal:<goal id>       <goal>/goal.md\n  step:<goal>/<step>   <goal>/steps/<step>.md\n  --unit WNN           the step a work unit maps to in work-unit-inventory.md\n\nViews: full, summary, metadata, ownership, instructions, acceptance, handoff,\ntesting, dependencies, execution-summary, changed-documents, inventory-row,\nvalidator. Default is full for inventory and review, summary otherwise.\n\nPaging: a page that withholds records reports next_token; pass it back as --token.\n"
    );
    std::process::exit(code.into());
}

fn die(message: impl AsRef<str>, code: u8) -> ! {
    eprintln!("{}", message.as_ref());
    std::process::exit(code.into());
}

#[derive(Default)]
struct Args {
    command: String,
    plan_dir: Option<PathBuf>,
    document: Option<String>,
    entry: Option<String>,
    check: Option<String>,
    refresh: Option<String>,
    view: Option<String>,
    token: Option<String>,
    format: String,
    max_bytes: usize,
    max_records: usize,
    read_only: bool,
    phase: Option<String>,
    state: Option<String>,
    findings_file: Option<PathBuf>,
    changed_files: Option<PathBuf>,
    source_hash: Option<String>,
    plan_hash: Option<String>,
    document_count: usize,
    check_count: usize,
    refresh_count: usize,
}

fn parse_args() -> Args {
    let mut result = Args {
        format: "text".into(),
        max_bytes: 32768,
        max_records: 128,
        ..Args::default()
    };
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => usage(0),
            "init" | "read" | "check" | "refresh" | "checkpoint" if result.command.is_empty() => {
                result.command = arg
            }
            "--plan-dir" => result.plan_dir = Some(PathBuf::from(next_arg(&mut args))),
            "--document" => {
                result.document = Some(next_arg(&mut args));
                result.document_count += 1;
            }
            "--unit" => {
                result.document = Some(format!("unit:{}", next_arg(&mut args)));
                result.document_count += 1;
            }
            "--entry" => {
                result.entry = Some(next_arg(&mut args));
                result.check_count += 1;
                result.refresh_count += 1;
            }
            "--changed" => {
                result.check = Some("changed".into());
                result.check_count += 1;
            }
            "--all" => {
                result.check = Some("all".into());
                result.check_count += 1;
            }
            "--stale" => {
                result.refresh = Some("stale".into());
                result.refresh_count += 1;
            }
            "--view" => result.view = Some(next_arg(&mut args)),
            "--token" => result.token = Some(next_arg(&mut args)),
            "--format" => result.format = next_arg(&mut args),
            "--max-bytes" => result.max_bytes = parse_limit(&next_arg(&mut args)),
            "--max-records" => result.max_records = parse_limit(&next_arg(&mut args)),
            "--read-only" => result.read_only = true,
            "--phase" => result.phase = Some(next_arg(&mut args)),
            "--state" => result.state = Some(next_arg(&mut args)),
            "--findings-file" => result.findings_file = Some(PathBuf::from(next_arg(&mut args))),
            "--changed-files" => result.changed_files = Some(PathBuf::from(next_arg(&mut args))),
            "--source-hash" => result.source_hash = Some(next_arg(&mut args)),
            "--plan-hash" => result.plan_hash = Some(next_arg(&mut args)),
            _ => usage(2),
        }
    }
    if result.command.is_empty() || result.plan_dir.is_none() {
        usage(2);
    }
    if result.format != "text" && result.format != "json" {
        die("usage: unsupported format", 2);
    }
    if result
        .token
        .as_deref()
        .is_some_and(|token| !valid_token(token))
    {
        die("usage: malformed --token", 2);
    }
    match result.command.as_str() {
        "read"
            if result.document_count != 1
                || result.check_count != 0
                || result.refresh_count != 0 =>
        {
            usage(2)
        }
        "check" if result.document_count != 0 || result.check_count != 1 => usage(2),
        "refresh" if result.document_count != 0 || result.refresh_count != 1 => usage(2),
        "checkpoint"
            if result.document_count != 0
                || result.check_count != 0
                || result.refresh_count != 0 =>
        {
            usage(2)
        }
        "init"
            if result.document_count != 0
                || result.check_count != 0
                || result.refresh_count != 0 =>
        {
            usage(2)
        }
        _ => {}
    }
    result
}

fn valid_token(token: &str) -> bool {
    let Some(rest) = token.strip_prefix("continue:") else {
        return false;
    };
    let mut parts = rest.split(':');
    let Some(hash) = parts.next() else {
        return false;
    };
    let Some(view) = parts.next() else {
        return false;
    };
    let Some(offset) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && hash.len() == 64
        && hash.chars().all(|character| character.is_ascii_hexdigit())
        && !view.is_empty()
        && view
            .chars()
            .all(|character| character.is_ascii_lowercase() || character == '-')
        && offset.parse::<usize>().is_ok()
}

fn next_arg<I>(args: &mut I) -> String
where
    I: Iterator<Item = String>,
{
    args.next().unwrap_or_else(|| usage(2))
}

fn parse_limit(raw: &str) -> usize {
    raw.parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or_else(|| die("usage: limits must be positive integers", 2))
}

fn default_view(id: &str) -> &'static str {
    match id {
        "inventory" | "adversarial-review" | "coverage" | "stories" | "bugs" | "planning-bugs"
        | "fixes" | "fix-keys" | "approval" => "full",
        _ => "summary",
    }
}

fn require_plan(plan: &Path) {
    if !plan.is_dir() {
        die(format!("not-found: plan directory {}", plan.display()), 66);
    }
}

fn context_root(plan: &Path) -> PathBuf {
    plan.join("context")
}

fn generation_root(plan: &Path) -> PathBuf {
    context_root(plan).join("snapshots")
}

fn next_generation(plan: &Path) -> u64 {
    let mut highest = 0;
    if let Ok(entries) = fs::read_dir(generation_root(plan)) {
        for entry in entries.flatten() {
            if let Ok(number) = entry.file_name().to_string_lossy().parse::<u64>() {
                highest = highest.max(number);
            }
        }
    }
    highest + 1
}

fn file_hash(path: &Path) -> String {
    if !path.is_file() {
        return "absent".into();
    }
    plan_context_core::hash_file(path).unwrap_or_else(|error| die(error, 66))
}

fn build_index(plan: &Path) -> String {
    let mut lines = vec!["entry_id\tpath\tkind\thash".into()];
    let plan_file = resolve_document(plan, "plan").unwrap_or_else(|error| die(error, 64));
    lines.push(format!(
        "plan\t{}\tplan\t{}",
        plan_file.display(),
        file_hash(&plan_file)
    ));
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
    for goal in &goals {
        let name = goal.file_name().unwrap().to_string_lossy();
        let file = goal.join("goal.md");
        lines.push(format!(
            "goal:{name}\t{}\tgoal\t{}",
            file.display(),
            file_hash(&file)
        ));
        let progress = goal.join("progress.md");
        if progress.is_file() {
            lines.push(format!(
                "goal-progress:{name}\t{}\tgoal-progress\t{}",
                progress.display(),
                file_hash(&progress)
            ));
        }
        let mut steps = Vec::new();
        if let Ok(entries) = fs::read_dir(goal.join("steps")) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "md")
                    && !path
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .ends_with("-testing")
                {
                    steps.push(path);
                }
            }
        }
        steps.sort();
        for step in steps {
            let name = step.file_stem().unwrap().to_string_lossy();
            lines.push(format!(
                "step:{}/{}\t{}\tstep\t{}",
                goal.file_name().unwrap().to_string_lossy(),
                name,
                step.display(),
                file_hash(&step)
            ));
        }
    }
    let inventory = plan.join("work-unit-inventory.md");
    if inventory.is_file() {
        if let Ok(rows) = inventory_rows(&inventory) {
            for row in rows {
                let file = plan
                    .join(&row.goal)
                    .join("steps")
                    .join(format!("{}.md", row.step));
                lines.push(format!(
                    "unit:{}\t{}\tunit\t{}",
                    row.id,
                    file.display(),
                    entry_hash(plan, &format!("unit:{}", row.id))
                        .unwrap_or_else(|_| "absent".into())
                ));
            }
        }
        lines.push(format!(
            "inventory\t{}\tinventory\t{}",
            inventory.display(),
            file_hash(&inventory)
        ));
        lines.push(format!(
            "coverage\t{}\tcoverage\t{}",
            inventory.display(),
            file_hash(&inventory)
        ));
    }
    for (id, name, kind) in [
        ("progress", "progress.md", "progress"),
        (
            "adversarial-review",
            "adversarial-review.md",
            "adversarial-review",
        ),
        ("stories", "ui-user-stories.md", "stories"),
        ("bugs", "bugs.md", "bugs"),
        ("planning-bugs", "planning-bugs.json", "planning-bugs"),
        ("fixes", "fixes.md", "fixes"),
        ("fix-keys", "fix-keys.json", "fix-keys"),
        ("approval", "approval.json", "approval"),
    ] {
        let file = plan.join(name);
        if file.is_file() {
            lines.push(format!(
                "{id}\t{}\t{kind}\t{}",
                file.display(),
                file_hash(&file)
            ));
        }
    }
    if let Ok(source_root) = env::var("CONTEXT_SOURCE_ROOT") {
        let skill = PathBuf::from(&source_root).join("planning/SKILL.md");
        if skill.is_file() {
            lines.push(format!(
                "source:SKILL.md\t{}\tsource\t{}",
                skill.display(),
                file_hash(&skill)
            ));
        }
        let reviewer = PathBuf::from(&source_root).join("planning/REVIEWER.md");
        if reviewer.is_file() {
            lines.push(format!(
                "source:REVIEWER.md\t{}\tsource\t{}",
                reviewer.display(),
                file_hash(&reviewer)
            ));
        } else {
            lines.push("source:REVIEWER.md\t-\tsource\tabsent - generate with planning/scripts/generate-reviewer.sh".into());
        }
    }
    lines.join("\n") + "\n"
}

fn publish_snapshot(plan: &Path, generation: u64, index: &str) {
    let target = generation_root(plan).join(generation.to_string());
    fs::create_dir_all(target.join("entries")).unwrap_or_else(|error| die(error.to_string(), 66));
    write_file(&target.join("index.tsv"), index.as_bytes());
    write_file(
        &target.join("manifest.tsv"),
        format!(
            "schema_version\tgenerator_version\tresult_schema_version\n{CONTEXT_SCHEMA_VERSION}\t{CONTEXT_GENERATOR_VERSION}\t{CONTEXT_RESULT_SCHEMA_VERSION}\n"
        )
        .as_bytes(),
    );
    write_file(&target.join("READY"), b"1\n");
    write_file(
        &context_root(plan).join("current"),
        format!("{generation}\n").as_bytes(),
    );
}

fn write_file(path: &Path, bytes: &[u8]) {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).unwrap_or_else(|error| die(error.to_string(), 66));
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    let mut file = File::create(&temporary).unwrap_or_else(|error| die(error.to_string(), 66));
    file.write_all(bytes)
        .unwrap_or_else(|error| die(error.to_string(), 66));
    drop(file);
    fs::rename(temporary, path).unwrap_or_else(|error| die(error.to_string(), 66));
}

fn init(plan: &Path) {
    let generation = next_generation(plan);
    let index = build_index(plan);
    publish_snapshot(plan, generation, &index);
    println!("command=init\nstatus=fresh\nsnapshot_generation={generation}\nentry_id=-\nchanged_ids=-\naffected_ids=-\nnext_token=-\nerror_code=-");
}

fn json_string(value: &str) -> String {
    let mut result = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            character if character.is_control() => {
                result.push_str(&format!("\\u{:04x}", character as u32))
            }
            character => result.push(character),
        }
    }
    result.push('"');
    result
}

fn role_cap(max_bytes: usize) -> usize {
    if let Ok(role) = env::var("ROLE_ID") {
        let original = role.clone();
        let mut role = role.to_ascii_lowercase();
        if role.starts_with("benny") && role[5..].chars().all(|c| c.is_ascii_digit() || c == '-') {
            role = "benny".into();
        }
        role = match role.as_str() {
            "alex" | "benny" | "chris" | "christian" | "christoph" | "dana" | "frank"
            | "maintainer" | "installer" | "oracle" | "eve" => role,
            "willie" => "maintainer".into(),
            "felix" => "installer".into(),
            "pythia" => "oracle".into(),
            _ => role,
        };
        if matches!(role.as_str(), "installer" | "oracle" | "eve") {
            die(
                format!(
                    "usage: role {role} does not allow the plan-context gate (reader allow-list)"
                ),
                64,
            );
        }
        if role.is_empty()
            || !matches!(
                role.as_str(),
                "alex"
                    | "benny"
                    | "chris"
                    | "christian"
                    | "christoph"
                    | "dana"
                    | "frank"
                    | "maintainer"
            )
        {
            die(format!("usage: unknown ROLE_ID \"{original}\""), 64);
        }
        max_bytes.min(32768)
    } else {
        max_bytes
    }
}

fn page(
    content: &str,
    start: usize,
    max_records: usize,
    max_bytes: usize,
) -> (String, usize, bool, usize) {
    let records: Vec<&str> = content.lines().collect();
    let mut output = String::new();
    let mut used = 0;
    let mut emitted = 0;
    let mut more = false;
    for record in records.iter().skip(start) {
        if emitted >= max_records {
            more = true;
            break;
        }
        let size = record.len() + 1;
        if used + size > max_bytes {
            if emitted == 0 {
                let end = record
                    .char_indices()
                    .take_while(|(index, _)| *index < max_bytes)
                    .map(|(index, character)| index + character.len_utf8())
                    .last()
                    .unwrap_or(0)
                    .min(max_bytes);
                output.push_str(&record[..end]);
                emitted = 1;
            }
            more = true;
            break;
        }
        output.push_str(record);
        output.push('\n');
        used += size;
        emitted += 1;
    }
    (output, emitted, more, records.len())
}

fn read_command(args: &Args, plan: &Path) {
    if args.document.is_none()
        || args.entry.is_some()
        || args.check.is_some()
        || args.refresh.is_some()
    {
        die("usage: read requires exactly one --document or --unit", 2);
    }
    let id = args.document.as_deref().unwrap();
    let file = resolve_document(plan, id).unwrap_or_else(|error| die(error, 64));
    if !file.is_file() {
        die(format!("not-found: {id}"), 66);
    }
    let view = args.view.as_deref().unwrap_or_else(|| default_view(id));
    let row_text = id
        .strip_prefix("unit:")
        .and_then(|unit| inventory_row_text(plan, unit).ok());
    let content =
        view_text(&file, view, row_text.as_deref()).unwrap_or_else(|error| die(error, 64));
    let max_bytes = role_cap(args.max_bytes);
    let hash = entry_hash(plan, id).unwrap_or_else(|error| die(error, 66));
    let start = if let Some(token) = &args.token {
        let prefix = format!("continue:{hash}:{view}:");
        if !token.starts_with(&prefix) {
            die(
                "stale: --token was minted against different content or view",
                65,
            );
        }
        token[prefix.len()..]
            .parse::<usize>()
            .unwrap_or_else(|_| die("usage: malformed --token", 2))
    } else {
        0
    };
    let (bounded, emitted, more, total) = page(&content, start, args.max_records, max_bytes);
    let bounded = bounded.trim_end_matches('\n').to_string();
    let summary_excerpt = view == "summary" && !more && file_line_count(&file) > total;
    if args.format == "json" {
        let next = if more {
            format!("\"continue:{hash}:{view}:{}\"", start + emitted)
        } else {
            "null".into()
        };
        let excerpt = if summary_excerpt {
            format!(
                "{{\"shown_lines\":{total},\"document_lines\":{},\"complete\":false,\"read_all_with\":\"--view full\"}}",
                file_line_count(&file)
            )
        } else {
            "null".into()
        };
        println!(
            "{{\"command\":\"read\",\"status\":\"ok\",\"entry_id\":{},\"view\":{},\"returned_records\":{emitted},\"total_records\":{total},\"truncated\":{more},\"content\":{},\"next_token\":{next},\"excerpt\":{excerpt}}}",
            json_string(id),
            json_string(view),
            json_string(&bounded)
        );
    } else {
        eprintln!("entry_id={id}\nview={view}\nreturned_records={emitted}\ntotal_records={total}\ntruncated={more}");
        println!("{bounded}");
        if more {
            println!("next_token=continue:{hash}:{view}:{}", start + emitted);
        } else if summary_excerpt {
            println!(
                "excerpt=summary shows {total} of {} line(s); this is not the whole document. Re-read with --view full (which pages, and reports next_token until nothing is withheld) before drawing a conclusion from it.",
                file_line_count(&file)
            );
        }
    }
    if !args.read_only {
        let state = context_root(plan).join("processed.tsv");
        let mut existing = fs::read_to_string(&state).unwrap_or_default();
        existing.retain(|character| character != '\r');
        let mut lines: Vec<String> = existing
            .lines()
            .filter(|line| !line.starts_with(&format!("{id}\t")))
            .map(str::to_string)
            .collect();
        lines.push(format!("{id}\t{hash}"));
        lines.sort_unstable();
        write_file(&state, format!("{}\n", lines.join("\n")).as_bytes());
    }
}

fn file_line_count(path: &Path) -> usize {
    fs::read_to_string(path)
        .map(|content| content.lines().count())
        .unwrap_or_default()
}

fn current_generation(plan: &Path) -> u64 {
    let current = context_root(plan).join("current");
    let value =
        fs::read_to_string(&current).unwrap_or_else(|_| die("stale: no current snapshot", 64));
    let generation: u64 = value
        .trim()
        .parse()
        .unwrap_or_else(|_| die("stale: invalid current snapshot", 64));
    if !generation_root(plan)
        .join(generation.to_string())
        .join("READY")
        .is_file()
    {
        die("stale: invalid current snapshot", 64);
    }
    generation
}

fn check_command(args: &Args, plan: &Path) {
    if args.check.is_none() && args.entry.is_none() {
        die("usage: check requires --entry, --changed, or --all", 2);
    }
    let generation = current_generation(plan);
    let mut changed = Vec::new();
    if args.check.as_deref() == Some("all") {
        let snapshot = fs::read_to_string(
            generation_root(plan)
                .join(generation.to_string())
                .join("index.tsv"),
        )
        .unwrap_or_default();
        let current = build_index(plan);
        let old: std::collections::HashMap<_, _> = snapshot
            .lines()
            .skip(1)
            .filter_map(|line| {
                let fields: Vec<_> = line.split('\t').collect();
                (fields.len() >= 4).then(|| (fields[0].to_string(), fields[3].to_string()))
            })
            .collect();
        let mut seen = std::collections::HashSet::new();
        for line in current.lines().skip(1) {
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() < 4 {
                continue;
            }
            let id = fields[0];
            seen.insert(id.to_string());
            match old.get(id) {
                None => changed.push(id.to_string()),
                Some(previous) if previous != fields[3] => changed.push(id.to_string()),
                _ => {}
            }
        }
        for id in old.keys() {
            if !seen.contains(id) {
                changed.push(id.clone());
            }
        }
        changed.sort();
    } else {
        let state =
            fs::read_to_string(context_root(plan).join("processed.tsv")).unwrap_or_default();
        for line in state.lines() {
            let Some((id, old)) = line.split_once('\t') else {
                continue;
            };
            if args.entry.as_deref().is_some_and(|wanted| wanted != id) {
                continue;
            }
            let current = entry_hash(plan, id).unwrap_or_else(|_| "deleted".into());
            if current != old {
                changed.push(id.to_string());
            }
        }
    }
    let status = if changed.is_empty() {
        "fresh"
    } else {
        "suspect"
    };
    if args.format == "json" {
        let ids = changed
            .iter()
            .map(|id| json_string(id))
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{{\"command\":\"check\",\"status\":\"{status}\",\"snapshot_generation\":\"{generation}\",\"entry_id\":{},\"changed_ids\":[{ids}],\"affected_ids\":[{ids}],\"next_token\":null,\"error_code\":{}}}",
            args.entry.as_deref().map(json_string).unwrap_or_else(|| "null".into()),
            if changed.is_empty() { "null" } else { "\"external-edit\"" }
        );
    } else {
        let ids = if changed.is_empty() {
            "-".into()
        } else {
            changed.join(",")
        };
        println!("command=check\nstatus={status}\nsnapshot_generation={generation}\nentry_id={}\nchanged_ids={ids}\naffected_ids={ids}\nnext_token=-\nerror_code={}", args.entry.as_deref().unwrap_or("-"), if changed.is_empty() { "-" } else { "external-edit" });
    }
}

fn checkpoint(args: &Args, plan: &Path) {
    let phase = args
        .phase
        .as_deref()
        .unwrap_or_else(|| die("usage: invalid checkpoint phase", 2));
    let state = args
        .state
        .as_deref()
        .unwrap_or_else(|| die("usage: invalid checkpoint state", 2));
    if !matches!(phase, "drafting" | "review" | "correction" | "validation")
        || !matches!(state, "in_progress" | "blocked" | "complete")
    {
        die("usage: invalid checkpoint phase", 2);
    }
    let findings = args
        .findings_file
        .as_deref()
        .filter(|path| path.is_file())
        .unwrap_or_else(|| die("usage: checkpoint input file missing", 2));
    let changed = args
        .changed_files
        .as_deref()
        .filter(|path| path.is_file())
        .unwrap_or_else(|| die("usage: checkpoint input file missing", 2));
    let source_hash = args
        .source_hash
        .as_deref()
        .filter(|value| value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or_else(|| die("usage: checkpoint hashes must be SHA-256", 2));
    let plan_hash = args
        .plan_hash
        .as_deref()
        .filter(|value| value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or_else(|| die("usage: checkpoint hashes must be SHA-256", 2));
    let now = utc_timestamp();
    let open_findings = input_lines(findings, true);
    let changed_files = input_lines(changed, false);
    let output = format!(
        "{{\"schema_version\":\"1.4.2\",\"run_id\":{},\"revision\":{},\"phase\":\"{phase}\",\"state\":\"{state}\",\"open_findings\":[{}],\"next_action\":{},\"changed_files\":[{}],\"source_hash\":\"{source_hash}\",\"plan_hash\":\"{plan_hash}\",\"created_at\":\"{now}\",\"updated_at\":\"{now}\"}}\n",
        json_string(&env::var("RUN_ID").unwrap_or_else(|_| "local".into())),
        json_string(&env::var("REVISION").unwrap_or_else(|_| "local".into())),
        open_findings,
        json_string(&env::var("NEXT_ACTION").unwrap_or_else(|_| "continue".into())),
        changed_files
    );
    let path = context_root(plan)
        .join("checkpoints")
        .join(format!("{phase}.json"));
    let _ = (findings, changed);
    write_file(&path, output.as_bytes());
    println!("checkpoint={phase}\nstate={state}\npath={}", path.display());
}

fn input_lines(path: &Path, skip_header: bool) -> String {
    let content = fs::read_to_string(path).unwrap_or_default();
    content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            if skip_header && index == 0 {
                return None;
            }
            let value = line.replace(['"', '\\'], "_");
            Some(json_string(&value))
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn utc_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = seconds / 86_400;
    let day_seconds = seconds % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        day_seconds / 3600,
        (day_seconds % 3600) / 60,
        day_seconds % 60
    )
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + if m <= 2 { 1 } else { 0 }, m, d)
}

fn main() -> ExitCode {
    let args = parse_args();
    let plan = args.plan_dir.as_deref().unwrap();
    require_plan(plan);
    match args.command.as_str() {
        "init" => init(plan),
        "read" => read_command(&args, plan),
        "check" => check_command(&args, plan),
        "refresh" => init(plan),
        "checkpoint" => checkpoint(&args, plan),
        _ => usage(2),
    }
    ExitCode::SUCCESS
}
