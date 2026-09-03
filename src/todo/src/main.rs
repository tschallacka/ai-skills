// MODE: DEV
// PACKAGE: PROD
//! todo — the work queue's tools in one binary.
//!
//! Rust rather than shell so the queue's tools depend on no shell at all: they
//! behave the same under bash, zsh or anything else, and survive macOS retiring
//! the bash it ships. A static binary also asks the target machine for nothing,
//! so shipping one LOWERS this skill's declared requirements — reading the queue
//! no longer needs `rjq` installed.
//!
//! This is a SEPARATE binary from `bugs`, not a mode of it, because the two
//! registers do not share their rules: no severity here, a different status
//! vocabulary, a tree that renames have to follow, and a `blocked_on` field that
//! holds prose as often as an id. One shared shell library had to pretend they
//! agreed, and every place it pretended became a defect.

mod cli;
mod clock;
mod migrate;
mod mutate;
mod query;
mod register;
mod resolve;

use std::io::{Read, Write};
use std::process::ExitCode;

use register::Register;

const EX_USAGE: u8 = 64;
const EX_DATAERR: u8 = 65;
const EX_NOINPUT: u8 = 66;
const EX_IOERR: u8 = 74;

const USAGE: &str = "\
todo — the work queue's tools.

Usage:
  todo add --title T --detail D
           [--priority normal] [--status open] [--id T42] [--parent T7]
           [--blocked-on TEXT] [--refs a,b] [--note N]
  todo update <ID> [--status S] [--priority P] [--detail D]
                   [--blocked-on TEXT] [--note N] [--append-note N]
  todo show <ID>
  todo list [--status S] [--priority P] [--parent ID] [--touching TEXT]
            [--since ISO8601]
  todo report [--since ISO8601]
  todo tree
  todo count [--status S]
  todo next-id
  todo check
  todo fmt
  todo migrate
  todo prune [--older-than ISO8601]             archive closed tasks
  todo resolve [<side>:<old-id>:<new-id> ...]   resolve a merge conflict
  todo --help

Any command takes --file PATH, which wins over everything else. Failing that
the queue is TODO_JSON, else ./TODO.json.

Closing needs its evidence: done, dropped and obsolete all require --note
saying what happened. A task marked done that nobody can check is a guess.

`prune` is how the queue stays a queue: closed tasks move OUT to a dated
archive beside it, and are never deleted. A closed task is kept back while an
open one is its child or names it in --blocked-on.

A queue this version did not write is not read in place. `migrate` copies it to
a versioned .back.json, carries what fits the current shape, and reports what
did not with the commands to move it by hand. There is no compatibility layer,
deliberately.

Exit codes: 1 a rule is broken or tasks need moving by hand; 64 usage;
65 unreadable or refused; 66 no queue or no such task; 74 write failed.
";

const FLAGS: &[&str] = &[
    "file",
    "title",
    "detail",
    "status",
    "priority",
    "id",
    "parent",
    "blocked-on",
    "refs",
    "note",
    "append-note",
    "touching",
    "since",
    "older-than",
];

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match run(&argv) {
        Ok(code) => code,
        Err(failure) => {
            eprintln!("todo: {}", failure.message);
            ExitCode::from(failure.code)
        }
    }
}

struct Failure {
    message: String,
    code: u8,
}

fn fail<T>(message: impl Into<String>, code: u8) -> Result<T, Failure> {
    Err(Failure {
        message: message.into(),
        code,
    })
}

fn run(argv: &[String]) -> Result<ExitCode, Failure> {
    match argv.first().map(String::as_str) {
        None | Some("--help") | Some("-h") => {
            print!("{USAGE}");
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }

    let args = match cli::parse(argv, FLAGS) {
        Ok(args) => args,
        Err(cli::ParseError::MissingValue(flag)) => {
            return fail(format!("{flag} needs a value"), EX_USAGE)
        }
        Err(cli::ParseError::UnknownFlag(flag)) => {
            return fail(format!("unknown option: {flag}"), EX_USAGE)
        }
    };

    let path = resolve_path(args.flag("file").map(str::to_string));

    match args.command.as_str() {
        "add" => add(&path, &args),
        "update" => update(&path, &args),
        "show" => show(&path, &args),
        "list" => {
            let register = read(&path)?;
            print!("{}", query::list(&register, &filter(&args)?));
            Ok(ExitCode::SUCCESS)
        }
        "report" => {
            let register = read(&path)?;
            print!("{}", query::report(&register, &filter(&args)?));
            Ok(ExitCode::SUCCESS)
        }
        "count" => {
            let register = read(&path)?;
            println!("{}", query::select(&register, &filter(&args)?).len());
            Ok(ExitCode::SUCCESS)
        }
        "tree" => {
            let register = read(&path)?;
            print!("{}", query::tree(&register));
            Ok(ExitCode::SUCCESS)
        }
        "next-id" => {
            let register = read(&path)?;
            println!("T{}", register.next_id());
            Ok(ExitCode::SUCCESS)
        }
        "check" => check(&path),
        "fmt" => {
            let register = read(&path)?;
            write(&path, &register)?;
            println!("{path}");
            Ok(ExitCode::SUCCESS)
        }
        "prune" => prune_command(&path, &args),
        "resolve" => resolve_command(&path, &args),
        "migrate" => migrate_command(&path),
        other => fail(format!("unknown command: {other}"), EX_USAGE),
    }
}

fn add(path: &str, args: &cli::Args) -> Result<ExitCode, Failure> {
    let mut register = read(path)?;

    let required = |name: &str| -> Result<String, Failure> {
        match args.flag(name) {
            Some(value) if !value.trim().is_empty() => Ok(value.to_string()),
            _ => fail(
                format!(
                    "--{name} is required. A queued task needs a title and enough detail \
                     that somebody else could pick it up."
                ),
                EX_USAGE,
            ),
        }
    };

    let new = mutate::NewTask {
        id: args.flag("id").map(str::to_string),
        title: required("title")?,
        detail: required("detail")?,
        status: enum_or(args.flag("status"), "open", "--status", register::STATUSES)?,
        priority: enum_or(
            args.flag("priority"),
            "normal",
            "--priority",
            register::PRIORITIES,
        )?,
        parent: args.flag("parent").map(str::to_string),
        blocked_on: args.flag("blocked-on").map(str::to_string),
        refs: args.list("refs"),
        note: args.flag("note").map(str::to_string),
    };

    // Checked before anything is built, so the message is about the request.
    if let Some(missing) = mutate::missing_evidence(new.status, new.note.as_deref()) {
        return fail(missing, EX_USAGE);
    }

    match mutate::add(&mut register, new) {
        Ok(id) => {
            write(path, &register)?;
            println!("Queued {id}");
            Ok(ExitCode::SUCCESS)
        }
        Err(findings) => {
            for finding in &findings {
                eprintln!("  {finding}");
            }
            fail(format!("task refused; {path} is unchanged"), EX_DATAERR)
        }
    }
}

fn update(path: &str, args: &cli::Args) -> Result<ExitCode, Failure> {
    let Some(id) = args.positional.first().cloned() else {
        return fail("update needs the id of the task to change", EX_USAGE);
    };
    let mut register = read(path)?;

    let change = mutate::Change {
        status: opt_enum(args.flag("status"), "--status", register::STATUSES)?,
        priority: opt_enum(args.flag("priority"), "--priority", register::PRIORITIES)?,
        detail: args.flag("detail").map(str::to_string),
        blocked_on: args.flag("blocked-on").map(str::to_string),
        note: args.flag("note").map(str::to_string),
        append_note: args.flag("append-note").map(str::to_string),
    };

    if change.is_empty() {
        return fail(
            format!("nothing to set for {id}; name at least one field to change"),
            EX_USAGE,
        );
    }
    if let Some(missing) = change.missing_evidence() {
        return fail(missing, EX_USAGE);
    }

    match mutate::update(&mut register, &id, change) {
        Ok(()) => {
            write(path, &register)?;
            println!("Updated {id}");
            Ok(ExitCode::SUCCESS)
        }
        Err(mutate::UpdateError::NoSuchEntry) => {
            fail(format!("no task {id} in {path}"), EX_NOINPUT)
        }
        Err(mutate::UpdateError::Unsound(findings)) => {
            for finding in &findings {
                eprintln!("  {finding}");
            }
            fail(format!("update refused; {path} is unchanged"), EX_DATAERR)
        }
    }
}

fn show(path: &str, args: &cli::Args) -> Result<ExitCode, Failure> {
    let Some(id) = args.positional.first().cloned() else {
        return fail("show needs an id", EX_USAGE);
    };
    let register = read(path)?;
    match register.find(&id) {
        Some(task) => {
            match serde_json::to_string_pretty(task) {
                Ok(text) => println!("{text}"),
                Err(error) => return fail(format!("cannot render {id}: {error}"), EX_IOERR),
            }
            Ok(ExitCode::SUCCESS)
        }
        None => fail(format!("no task {id} in {path}"), EX_NOINPUT),
    }
}

fn check(path: &str) -> Result<ExitCode, Failure> {
    let register = read(path)?;
    let findings = register.findings();
    if findings.is_empty() {
        println!("{} tasks, sound", register.tasks.len());
        return Ok(ExitCode::SUCCESS);
    }
    for finding in &findings {
        eprintln!("  {finding}");
    }
    eprintln!("todo: {path} breaks {} of its own rules", findings.len());
    Ok(ExitCode::from(1))
}

/// Move closed tasks out of the live queue and into a dated archive.
///
/// Two files are written or neither is: the archive first, then the shortened
/// register. Doing it the other way round is how a prune loses the tasks it was
/// supposed to keep — the register would already be short with nothing holding
/// what left.
///
/// Gated by CONFIRM for the same reason `resolve` is: the caller is shown the
/// exact content and hands back the token computed over it, which proves the
/// approval was for what is about to be written and not for something older.
fn prune_command(path: &str, args: &cli::Args) -> Result<ExitCode, Failure> {
    let register = read(path)?;
    let cutoff = args.flag("older-than");
    let plan = migrate::plan(&register, cutoff);

    if !plan.held.is_empty() {
        eprintln!("todo: kept back, though closed:");
        for (id, why) in &plan.held {
            eprintln!("  {id}: {why}");
        }
    }
    if plan.removable.is_empty() {
        println!("{} tasks, nothing to prune", register.tasks.len());
        return Ok(ExitCode::SUCCESS);
    }

    let remaining = plan.remaining(&register);
    let archive = plan.archive(&register);
    let archive_path = migrate::archive_path(path, &migrate::today());

    let text = serialised(&remaining)?;
    // The live register is what stdout shows, because that is the file the
    // caller has to be willing to live with; the archive is derived from the
    // same decision and named on stderr.
    print!("{text}");

    let token = resolve::fingerprint(text.as_bytes());
    eprintln!(
        "todo: would prune {} closed task{} of {}, leaving {}",
        plan.removable.len(),
        if plan.removable.len() == 1 { "" } else { "s" },
        register.tasks.len(),
        remaining.tasks.len()
    );
    for task in &plan.removable {
        eprintln!(
            "  {} [{}] {}",
            task.id,
            register::token(task.status),
            task.title
        );
    }
    eprintln!("todo: they would be kept in {archive_path}");

    let confirm = std::env::var("CONFIRM").unwrap_or_default();
    if confirm.is_empty() {
        eprintln!("todo: nothing written. To write this exact content, re-run the same");
        eprintln!("      command with this prefix:");
        eprintln!("  CONFIRM={token}");
        return Ok(ExitCode::SUCCESS);
    }
    if confirm != token {
        eprintln!("todo: the confirmation does not match this content.");
        eprintln!("  given:    {confirm}");
        eprintln!("  computed: {token}");
        return fail(
            "a stale confirmation means the queue changed since it was printed; \
             re-read the new output and confirm that",
            EX_DATAERR,
        );
    }

    if std::path::Path::new(&archive_path).exists() {
        // Two prunes in one day are legitimate, so the second is merged into the
        // first day's archive rather than refused or overwriting it.
        let existing = read(&archive_path)?;
        let mut combined = archive.clone();
        for task in existing.tasks {
            if combined.find(&task.id).is_none() {
                combined.tasks.push(task);
            }
        }
        combined.sort();
        write(&archive_path, &combined)?;
    } else {
        write(&archive_path, &archive)?;
    }
    write(path, &remaining)?;

    eprintln!("todo: wrote {archive_path} and {path} — stage both:");
    eprintln!("  git add {archive_path} {path}");
    Ok(ExitCode::SUCCESS)
}

/// Resolve an id collision in a conflicted queue.
///
/// Prints the resolved queue to stdout and the confirmation to stderr, so
/// `… > preview.json` keeps the file and leaves the token on screen. Nothing is
/// written until that token comes back in CONFIRM.
///
/// CONFIRM is an environment variable rather than a flag on purpose: it is a
/// token this tool produced, not an option a caller picks, and a flag would have
/// to be either documented — inviting an invented one — or exempted from the
/// every-flag-is-documented contract.
fn resolve_command(path: &str, args: &cli::Args) -> Result<ExitCode, Failure> {
    let sides = match resolve::read_sides(path) {
        Ok(sides) => sides,
        Err(resolve::SidesError::NotConflicted) => {
            println!("clean {path}");
            eprintln!("todo: no conflict to resolve; nothing was written");
            return Ok(ExitCode::SUCCESS);
        }
        Err(resolve::SidesError::MissingStage(side)) => {
            return fail(
                format!("no '{side}' stage for {path} in the index; is this a merge conflict?"),
                EX_DATAERR,
            )
        }
        Err(resolve::SidesError::Unparsable { side, why }) => {
            return fail(
                format!(
                    "the '{side}' side of {path} does not parse as a queue ({why}), \
                     so the conflict is not only an id collision. Resolve it by hand."
                ),
                EX_DATAERR,
            )
        }
    };

    // Each side is checked on its own before any decision is asked for, so an
    // already-unsound branch is reported as that branch's fault rather than
    // blamed on the decisions.
    let mut unsound = false;
    for (label, register) in [
        (&sides.ours_label, &sides.ours),
        (&sides.theirs_label, &sides.theirs),
    ] {
        let findings = register.findings();
        if !findings.is_empty() {
            unsound = true;
            eprintln!("todo: the '{label}' side is not a sound queue on its own:");
            for finding in findings {
                eprintln!("    {finding}");
            }
        }
    }
    if unsound {
        eprintln!("todo: fix that side on its own branch first — no renaming can make a");
        eprintln!(
            "      queue sound that was already unsound, and merging it carries the fault in."
        );
        return fail("refusing to resolve into an unsound queue", EX_DATAERR);
    }

    let changed = sides.since_base();
    if !changed.is_empty() {
        eprintln!("todo: changed since the merge base:");
        eprint!("{changed}");
    }

    let contests = sides.contests();
    let divergences: Vec<&String> = contests
        .iter()
        .filter_map(|c| match c {
            resolve::Contest::Divergence(id) => Some(id),
            _ => None,
        })
        .collect();
    if !divergences.is_empty() {
        eprintln!("todo: these tasks were edited differently on both sides. A new id would");
        eprintln!("      not resolve those: the two versions are the same task, so one");
        eprintln!("      content has to win.");
        for id in divergences {
            eprintln!("    divergence: {id}");
        }
        return fail(
            "resolve the divergent tasks by hand, then re-run",
            EX_DATAERR,
        );
    }

    let collisions: Vec<String> = contests
        .iter()
        .filter_map(|c| match c {
            resolve::Contest::Collision(id) => Some(id.clone()),
            _ => None,
        })
        .collect();
    if collisions.is_empty() {
        eprintln!("todo: no id collisions between the two sides; the conflict is textual only.");
        eprintln!("      Take either side: git checkout --theirs -- {path}");
        return fail("nothing for this command to decide", EX_DATAERR);
    }

    let mut renames = Vec::new();
    for spec in &args.positional {
        match resolve::parse_rename(spec, &sides.ours_label, &sides.theirs_label) {
            Ok(rename) => renames.push(rename),
            Err(message) => return fail(message, EX_USAGE),
        }
    }

    let mut suggestion = sides.next_free();
    let undecided: Vec<&String> = collisions
        .iter()
        .filter(|id| !renames.iter().any(|r| &&r.from == id))
        .collect();
    if !undecided.is_empty() {
        eprintln!("These ids exist on both sides as different tasks, and each needs a decision:\n");
        for id in &undecided {
            let ours = sides.ours.find(id).map(|t| t.title.as_str()).unwrap_or("-");
            let theirs = sides
                .theirs
                .find(id)
                .map(|t| t.title.as_str())
                .unwrap_or("-");
            eprintln!("  {id}");
            eprintln!("      ours   ({}): {ours}", sides.ours_label);
            eprintln!("      theirs ({}): {theirs}", sides.theirs_label);
            eprintln!("      decide with: theirs:{id}:T{suggestion}\n");
            suggestion += 1;
        }
        return fail(
            format!(
                "{} undecided collision(s); pass a decision for each in a single run",
                undecided.len()
            ),
            EX_DATAERR,
        );
    }

    let merged = resolve::merge(&sides, &renames);
    let findings = merged.findings();
    if !findings.is_empty() {
        for finding in &findings {
            eprintln!("  {finding}");
        }
        return fail(
            "the decisions given do not produce a sound queue".to_string(),
            EX_DATAERR,
        );
    }

    let text = serialised(&merged)?;
    print!("{text}");

    let token = resolve::fingerprint(text.as_bytes());
    eprintln!("todo: resolved {path}: {} tasks", merged.tasks.len());
    for rename in &renames {
        eprintln!(
            "  renamed {} -> {} on {}",
            rename.from,
            rename.to,
            match rename.side {
                resolve::Side::Ours => &sides.ours_label,
                resolve::Side::Theirs => &sides.theirs_label,
            }
        );
    }
    let prose = resolve::prose_mentions(&merged, &renames);
    if !prose.is_empty() {
        eprintln!("todo: renamed ids are still named in prose, left for a person to read:");
        for line in prose {
            eprintln!("{line}");
        }
    }

    let confirm = std::env::var("CONFIRM").unwrap_or_default();
    if confirm.is_empty() {
        eprintln!("todo: nothing written. To write this exact content, re-run the same");
        eprintln!("      command with this prefix:");
        eprintln!("  CONFIRM={token}");
        return Ok(ExitCode::SUCCESS);
    }
    if confirm != token {
        eprintln!("todo: the confirmation does not match this content.");
        eprintln!("  given:    {confirm}");
        eprintln!("  computed: {token}");
        return fail(
            "a stale confirmation means the queue or the decisions changed since it \
             was printed; re-read the new output and confirm that",
            EX_DATAERR,
        );
    }
    write(path, &merged)?;
    eprintln!("todo: wrote {path} — stage it when you have read it: git add {path}");
    Ok(ExitCode::SUCCESS)
}

fn filter(args: &cli::Args) -> Result<query::Filter, Failure> {
    Ok(query::Filter {
        status: opt_enum(args.flag("status"), "--status", register::STATUSES)?,
        priority: opt_enum(args.flag("priority"), "--priority", register::PRIORITIES)?,
        parent: args.flag("parent").map(str::to_string),
        touching: args.flag("touching").map(str::to_string),
        since: args.flag("since").map(str::to_string),
    })
}

fn enum_or<T: serde::de::DeserializeOwned>(
    given: Option<&str>,
    default: &str,
    field: &str,
    allowed: &[&str],
) -> Result<T, Failure> {
    cli::enum_value(given.unwrap_or(default), field, allowed).map_err(|message| Failure {
        message,
        code: EX_USAGE,
    })
}

fn opt_enum<T: serde::de::DeserializeOwned>(
    given: Option<&str>,
    field: &str,
    allowed: &[&str],
) -> Result<Option<T>, Failure> {
    match given {
        None => Ok(None),
        Some(raw) => cli::enum_value(raw, field, allowed)
            .map(Some)
            .map_err(|message| Failure {
                message,
                code: EX_USAGE,
            }),
    }
}

fn migrate_command(path: &str) -> Result<ExitCode, Failure> {
    let text = read_text(path)?;
    let loose: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            return fail(
                format!(
                    "{path} is not JSON at all ({error}); there is nothing to convert. \
                     Read it yourself and re-file what it holds with `todo add`."
                ),
                EX_DATAERR,
            )
        }
    };

    let claimed = migrate::claimed_version(&loose);
    if migrate::is_current(&claimed) {
        println!(
            "{path} is already {} — nothing to migrate",
            migrate::SUPPORTED
        );
        return Ok(ExitCode::SUCCESS);
    }

    // The backup is written before anything is parsed into the new shape, so a
    // conversion that goes wrong has already preserved the original.
    let backup = migrate::backup_path(path, &claimed);
    if std::path::Path::new(&backup).exists() {
        return fail(
            format!("{backup} already exists; move it aside rather than overwriting the evidence"),
            EX_DATAERR,
        );
    }
    if std::fs::write(&backup, &text).is_err() {
        return fail(format!("cannot write the backup at {backup}"), EX_IOERR);
    }

    let total = count_entries(&loose);
    let (carried, archived, unconvertible) = migrate::attempt(&loose);
    let register = migrate::rebuilt(&loose, carried);
    let carried_count = register.tasks.len();
    write(path, &register)?;

    eprintln!("backed up {total} tasks to {backup}");
    // Not "open tasks": a closed ancestor of an open task comes along too, or
    // the migrated file would be unsound the moment it was written.
    let closed_carried = register
        .tasks
        .iter()
        .filter(|t| !t.status.is_open())
        .count();
    eprintln!(
        "carried {carried_count} task{} into {}{}",
        if carried_count == 1 { "" } else { "s" },
        migrate::SUPPORTED,
        if closed_carried == 0 {
            String::new()
        } else {
            format!(
                " ({closed_carried} of them closed, kept because live work still hangs off them)"
            )
        }
    );
    if !archived.is_empty() {
        // Migration prunes as it converts: a closed task has no business in a
        // queue this tool is about to start reading every day, and the backup
        // holds it.
        eprintln!(
            "left {} closed task{} in the backup: {}",
            archived.len(),
            if archived.len() == 1 { "" } else { "s" },
            archived.join(" ")
        );
    }
    if unconvertible.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    eprint!("\n{}", migrate::instructions(&backup, &unconvertible));
    Ok(ExitCode::from(1))
}

fn count_entries(value: &serde_json::Value) -> usize {
    value
        .get("tasks")
        .and_then(|t| t.as_array())
        .map_or(0, |a| a.len())
}

/// `--file` beats the environment, which beats the default.
///
/// The other order looked harmless and was not: with TODO_JSON exported, every
/// `--file <other>` silently read and wrote the exported file instead. A flag on
/// the command line is the more specific instruction, and a caller who passes one
/// is entitled to have it obeyed — the variable is a default for a session, not an
/// override of what the caller just said.
fn resolve_path(explicit: Option<String>) -> String {
    if let Some(path) = explicit.filter(|p| !p.is_empty()) {
        return path;
    }
    if let Ok(from_env) = std::env::var("TODO_JSON") {
        if !from_env.is_empty() {
            return from_env;
        }
    }
    "TODO.json".to_string()
}

fn read_text(path: &str) -> Result<String, Failure> {
    let mut text = String::new();
    match std::fs::File::open(path) {
        Ok(mut handle) => {
            if handle.read_to_string(&mut text).is_err() {
                return fail(format!("{path} is not readable text"), EX_DATAERR);
            }
        }
        Err(_) => {
            return fail(
                format!("no queue at {path} — point TODO_JSON at one"),
                EX_NOINPUT,
            )
        }
    }
    Ok(text)
}

fn read(path: &str) -> Result<Register, Failure> {
    let text = read_text(path)?;
    // A version this binary did not write is not read in place. Saying so and
    // naming the one command that handles it beats a compatibility branch.
    if let Ok(loose) = serde_json::from_str::<serde_json::Value>(&text) {
        let claimed = migrate::claimed_version(&loose);
        if !claimed.is_empty() && !migrate::is_current(&claimed) {
            return fail(
                format!(
                    "{path} was written by version {claimed}; this binary writes {}. \
                     Run `todo migrate` — it backs the file up first.",
                    migrate::SUPPORTED
                ),
                EX_DATAERR,
            );
        }
    }
    match serde_json::from_str(&text) {
        Ok(register) => Ok(register),
        // The parse error names the field and the line, which is the whole point
        // of the types: "unknown variant `wip`" says where the fault is, where a
        // string comparison would only have said "unknown status".
        Err(error) => fail(format!("{path}: {error}"), EX_DATAERR),
    }
}

/// Canonical form: two-space pretty JSON with a trailing newline, which is what
/// the queue has always been on disk.
fn serialised(register: &Register) -> Result<String, Failure> {
    let mut text = match serde_json::to_string_pretty(register) {
        Ok(text) => text,
        Err(error) => return fail(format!("cannot serialise: {error}"), EX_IOERR),
    };
    text.push('\n');
    Ok(text)
}

/// Written through a temp file in the target's own directory, so a failed write
/// cannot truncate the queue and the rename never crosses a filesystem.
fn write(path: &str, register: &Register) -> Result<(), Failure> {
    let text = serialised(register)?;

    let directory = std::path::Path::new(path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let temp = directory.join(format!(".todo-write.{}", std::process::id()));

    let written = std::fs::File::create(&temp)
        .and_then(|mut handle| handle.write_all(text.as_bytes()).map(|_| handle))
        .and_then(|mut handle| handle.flush());
    if written.is_err() {
        let _ = std::fs::remove_file(&temp);
        return fail(format!("cannot write beside {path}"), EX_IOERR);
    }
    if std::fs::rename(&temp, path).is_err() {
        let _ = std::fs::remove_file(&temp);
        return fail(format!("cannot replace {path}"), EX_IOERR);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_file_wins_over_the_environment() {
        // One test rather than three: TODO_JSON is process-wide, so splitting this
        // across tests would have them race under cargo's thread-per-test.
        std::env::set_var("TODO_JSON", "/from/the/environment.json");
        assert_eq!(
            resolve_path(Some("/on/the/command/line.json".into())),
            "/on/the/command/line.json"
        );
        assert_eq!(resolve_path(None), "/from/the/environment.json");
        // An empty flag value is not a path, so it does not shadow the variable.
        assert_eq!(
            resolve_path(Some(String::new())),
            "/from/the/environment.json"
        );
        std::env::remove_var("TODO_JSON");
        assert_eq!(resolve_path(None), "TODO.json");
    }
}
