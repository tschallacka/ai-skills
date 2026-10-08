// MODE: DEV
// PACKAGE: PROD
//! decisions — the question register's tools in one binary.
//!
//! Rust rather than shell, the same reason bug-report's and todo's own
//! binaries are: no shell dependency, no `rjq` requirement to read or write
//! the register, behaves the same everywhere this ships.

use decisions::{
    add, answer, close, implement, list, migrate, stub, Filter, NewQuestion, Priority, Register,
};
use std::io::{Read, Write};
use std::process::ExitCode;

const EX_USAGE: u8 = 64;
const EX_DATAERR: u8 = 65;
const EX_NOINPUT: u8 = 66;
const EX_IOERR: u8 = 74;

const USAGE: &str = "\
decisions — the question register's tools.

Usage:
  decisions add --title T --option a:LABEL --option b:LABEL [--option c:LABEL ...]
                [--priority normal] [--context C] [--file PATH]
  decisions list [--status open|decided|implemented|closed|dropped|obsolete]
                 [--priority urgent|high|normal|low|someday] [--branch B] [--file PATH]
  decisions answer <ID> <LETTER> [--file PATH]
  decisions stub <ID> <ASSUMPTION> [--file PATH]
  decisions implement <ID> [NOTE] [--file PATH]
  decisions apply <ID> <RESOLUTION> [--file PATH]
  decisions close <ID> <RESOLUTION> [--file PATH]   (an alias for apply)
  decisions resolve-path   print the register path a mutating command would
                           use, with no prompting and nothing created
  decisions --help

Any command takes --file PATH, which wins over everything else. Failing that
the register is DECISIONS_JSON, else ./DECISIONS.json.

A question's lifecycle is open -> decided -> implemented: `answer` records
the user's pick and moves it to decided; `implement` then records that the
pick was actually carried out in the code. `close`/`apply` can withdraw a
question from any status, with or without ever implementing it.

`add` records the current git branch automatically, as context on the
question: it travels with the question, it is not a separate filter
mechanism.

A register this version did not write is migrated on read: the original is
backed up to a versioned .back.json beside it, and every entry that still
converts is carried forward.
";

#[derive(Debug)]
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

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match run(&argv) {
        Ok(code) => code,
        Err(failure) => {
            eprintln!("decisions: {}", failure.message);
            ExitCode::from(failure.code)
        }
    }
}

fn run(argv: &[String]) -> Result<ExitCode, Failure> {
    if argv.is_empty() || argv.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return Ok(ExitCode::SUCCESS);
    }

    // Dispatched before resolve_path itself runs: resolve_path may prompt and
    // create a worktree as a side effect, and resolve-path's own entire point
    // is to report where that would land WITHOUT ever doing either.
    if argv[0] == "resolve-path" {
        return run_resolve_path_command(argv);
    }

    let path = resolve_path(flag_value(argv, "--file"));

    match argv[0].as_str() {
        "add" => add_command(&path, argv),
        "list" => list_command(&path, argv),
        "answer" => answer_command(&path, argv),
        "stub" => stub_command(&path, argv),
        "implement" => implement_command(&path, argv),
        "apply" | "close" => close_command(&path, argv),
        other => fail(format!("unknown command: {other}"), EX_USAGE),
    }
}

/// The value following the first occurrence of `name`, if both it and a
/// following argument exist.
fn flag_value(argv: &[String], name: &str) -> Option<String> {
    argv.iter()
        .position(|a| a == name)
        .and_then(|i| argv.get(i + 1))
        .cloned()
}

/// Every occurrence of `--option letter:label`, split on the first colon.
fn option_values(argv: &[String]) -> Result<Vec<decisions::Choice>, Failure> {
    let mut options = Vec::new();
    for (i, arg) in argv.iter().enumerate() {
        if arg != "--option" {
            continue;
        }
        let raw = argv.get(i + 1).ok_or_else(|| Failure {
            message: "--option needs a value".to_string(),
            code: EX_USAGE,
        })?;
        let (letter, label) = raw.split_once(':').ok_or_else(|| Failure {
            message: format!("--option {raw} must be letter:label"),
            code: EX_USAGE,
        })?;
        options.push(decisions::Choice {
            letter: letter.to_string(),
            label: label.to_string(),
        });
    }
    Ok(options)
}

/// Positional (non-flag) arguments after the subcommand, in order: `add`'s
/// and `list`'s flags are all `--name value` pairs, but `answer`/`stub`/
/// `apply`/`close` take an id and free text positionally.
fn positional(argv: &[String]) -> Vec<&str> {
    let mut out = Vec::new();
    let mut skip_next = false;
    for arg in &argv[1..] {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg.starts_with("--") {
            skip_next = true;
            continue;
        }
        out.push(arg.as_str());
    }
    out
}

fn parse_priority(value: &str) -> Result<Priority, Failure> {
    serde_json::from_value(serde_json::Value::String(value.to_lowercase())).map_err(|_| Failure {
        message: format!("unknown priority {value}; one of urgent, high, normal, low, someday"),
        code: EX_USAGE,
    })
}

fn parse_status(value: &str) -> Result<decisions::Status, Failure> {
    serde_json::from_value(serde_json::Value::String(value.to_lowercase())).map_err(|_| Failure {
        message: format!(
            "unknown status {value}; one of open, decided, implemented, closed, dropped, obsolete"
        ),
        code: EX_USAGE,
    })
}

/// The branch `add` records as context, read from the real repository the
/// current directory sits in. Not being in one, or `git` not being on PATH,
/// is not an error: the question is still worth raising, just with no
/// branch to show.
fn current_branch() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|branch| branch.trim().to_string())
        .unwrap_or_default()
}

fn add_command(path: &str, argv: &[String]) -> Result<ExitCode, Failure> {
    let title = flag_value(argv, "--title").ok_or_else(|| Failure {
        message: "--title is required".to_string(),
        code: EX_USAGE,
    })?;
    let options = option_values(argv)?;
    if options.is_empty() {
        return fail("at least one --option is required", EX_USAGE);
    }
    let priority = match flag_value(argv, "--priority") {
        Some(value) => parse_priority(&value)?,
        None => Priority::Normal,
    };
    let context = flag_value(argv, "--context").unwrap_or_default();

    let mut register = read(path)?;
    let id = add(
        &mut register,
        NewQuestion {
            title,
            options,
            priority,
            branch: current_branch(),
            context,
        },
    )
    .map_err(|message| Failure {
        message,
        code: EX_DATAERR,
    })?;
    write(path, &register)?;
    println!("{id}");
    Ok(ExitCode::SUCCESS)
}

fn list_command(path: &str, argv: &[String]) -> Result<ExitCode, Failure> {
    let mut filter = Filter::default();
    if let Some(value) = flag_value(argv, "--status") {
        filter.status = Some(parse_status(&value)?);
    }
    if let Some(value) = flag_value(argv, "--priority") {
        filter.priority = Some(parse_priority(&value)?);
    }
    if let Some(value) = flag_value(argv, "--branch") {
        filter.branch = Some(value);
    }
    let register = read(path)?;
    for question in list(&register, &filter) {
        println!(
            "{} [{:?}/{:?}] {} ({})",
            question.id, question.priority, question.status, question.title, question.branch
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn answer_command(path: &str, argv: &[String]) -> Result<ExitCode, Failure> {
    let positional = positional(argv);
    let [id, letter] = positional[..] else {
        return fail("answer needs an id and an option letter", EX_USAGE);
    };
    let mut register = read(path)?;
    answer(&mut register, id, letter).map_err(|message| Failure {
        message,
        code: EX_DATAERR,
    })?;
    write(path, &register).map(|_| ExitCode::SUCCESS)
}

fn stub_command(path: &str, argv: &[String]) -> Result<ExitCode, Failure> {
    let positional = positional(argv);
    if positional.len() < 2 {
        return fail("stub needs an id and an assumption", EX_USAGE);
    }
    let id = positional[0];
    let assumption = positional[1..].join(" ");
    let mut register = read(path)?;
    stub(&mut register, id, &assumption).map_err(|message| Failure {
        message,
        code: EX_DATAERR,
    })?;
    write(path, &register).map(|_| ExitCode::SUCCESS)
}

fn implement_command(path: &str, argv: &[String]) -> Result<ExitCode, Failure> {
    let positional = positional(argv);
    if positional.is_empty() {
        return fail("implement needs an id", EX_USAGE);
    }
    let id = positional[0];
    let note = positional[1..].join(" ");
    let mut register = read(path)?;
    implement(&mut register, id, &note).map_err(|message| Failure {
        message,
        code: EX_DATAERR,
    })?;
    write(path, &register).map(|_| ExitCode::SUCCESS)
}

fn close_command(path: &str, argv: &[String]) -> Result<ExitCode, Failure> {
    let positional = positional(argv);
    if positional.len() < 2 {
        return fail("apply/close needs an id and a resolution", EX_USAGE);
    }
    let id = positional[0];
    let resolution = positional[1..].join(" ");
    let mut register = read(path)?;
    close(&mut register, id, &resolution).map_err(|message| Failure {
        message,
        code: EX_DATAERR,
    })?;
    write(path, &register).map(|_| ExitCode::SUCCESS)
}

/// `--file` beats the environment, which beats the default, the same order
/// bug-report's and todo's own `resolve_path` use.
fn resolve_path(explicit: Option<String>) -> String {
    if let Some(path) = explicit.filter(|p| !p.is_empty()) {
        return path;
    }
    if let Ok(from_env) = std::env::var("DECISIONS_JSON") {
        if !from_env.is_empty() {
            return from_env;
        }
    }
    if let Ok(project) = planning_core::project_root_for(None) {
        if let Ok(candidate) = planning_core::registers_scoped_root(&project) {
            if let Some(path) = resolved_registers_path(&project, &candidate) {
                return path;
            }
        }
    }
    "DECISIONS.json".to_string()
}

/// Resolves the shared registers worktree's own DECISIONS.json for the
/// bare-filename fallback case: a project's own tsch config is consulted
/// first (never prompting for an explicit `registers_access` opinion), an
/// already-recognized worktree is used silently, and only a genuinely
/// undecided project reaches the interactive first-use prompt. `None` means
/// "fall through to today's bare DECISIONS.json", whether because the
/// project opted out, declined once already, or a worktree-creation attempt
/// failed (disk full, permissions) -- a creation failure must never abort
/// the command outright.
fn resolved_registers_path(
    project: &std::path::Path,
    candidate: &std::path::Path,
) -> Option<String> {
    let cfg = planning_core::read_tsch_config(project);
    if cfg.registers_access.as_deref() == Some("main-checkout") {
        return None;
    }
    if cfg.registers_access.as_deref() == Some("dedicated-worktree") {
        if planning_core::worktree_recognized(candidate) {
            return Some(register_file(candidate));
        }
        let branch = cfg
            .registers_branch
            .unwrap_or_else(|| "registers".to_string());
        return create_registers_worktree(project, candidate, &branch)
            .then(|| register_file(candidate));
    }
    if planning_core::worktree_recognized(candidate) {
        return Some(register_file(candidate));
    }
    if planning_core::is_declined(candidate) {
        return None;
    }
    prompt_for_registers_worktree(project, candidate)
}

fn prompt_for_registers_worktree(
    project: &std::path::Path,
    candidate: &std::path::Path,
) -> Option<String> {
    use planning_core::PromptOutcome;
    let outcome = planning_core::prompt_yes_no_with_default(
        "No registers worktree is set up for this project yet. Create a \
         dedicated sparse git worktree for BUGS.json/TODO.json/DECISIONS.json \
         on branch \"registers\" (recommended -- avoids merge conflicts between \
         concurrent agents)?",
        true,
        false,
    );
    match outcome {
        PromptOutcome::Answered(true) | PromptOutcome::DefaultedNonInteractive(true) => {
            if !create_registers_worktree(project, candidate, "registers") {
                return None;
            }
            let _ = planning_core::write_tsch_config_patch(
                project,
                Some("dedicated-worktree"),
                Some("registers"),
                None,
                None,
            );
            Some(register_file(candidate))
        }
        PromptOutcome::Answered(false) => {
            match planning_core::prompt_branch_name(
                "Use a different branch name instead (leave empty to decline entirely)",
                "registers",
            ) {
                Some(name) => {
                    if !create_registers_worktree(project, candidate, &name) {
                        return None;
                    }
                    let _ = planning_core::write_tsch_config_patch(
                        project,
                        Some("dedicated-worktree"),
                        Some(&name),
                        None,
                        None,
                    );
                    Some(register_file(candidate))
                }
                None => {
                    let _ = planning_core::mark_declined(candidate);
                    None
                }
            }
        }
        // A piped/non-interactive run never answered anything, so nothing is
        // declined and no config is written -- the feature stays open for the
        // next, possibly interactive, invocation (AR-04).
        PromptOutcome::DefaultedNonInteractive(false) => None,
    }
}

fn create_registers_worktree(
    project: &std::path::Path,
    candidate: &std::path::Path,
    branch: &str,
) -> bool {
    match planning_core::create_sparse_worktree(
        project,
        candidate,
        branch,
        false,
        &["/BUGS.json", "/TODO.json", "/DECISIONS.json"],
    ) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("decisions: could not create the registers worktree: {error}");
            false
        }
    }
}

fn register_file(candidate: &std::path::Path) -> String {
    candidate.join("DECISIONS.json").display().to_string()
}

/// `decisions resolve-path`: reports where a mutating command would
/// read/write, without ever prompting, deciding or creating anything -- a
/// mod shells out to this to find the real register file instead of
/// hardcoding the session root.
fn run_resolve_path_command(argv: &[String]) -> Result<ExitCode, Failure> {
    println!("{}", resolve_path_preview(flag_value(argv, "--file")));
    Ok(ExitCode::SUCCESS)
}

fn resolve_path_preview(explicit: Option<String>) -> String {
    if let Some(path) = explicit.filter(|p| !p.is_empty()) {
        return path;
    }
    if let Ok(from_env) = std::env::var("DECISIONS_JSON") {
        if !from_env.is_empty() {
            return from_env;
        }
    }
    if let Ok(project) = planning_core::project_root_for(None) {
        if let Ok(candidate) = planning_core::registers_scoped_root(&project) {
            let cfg = planning_core::read_tsch_config(&project);
            if cfg.registers_access.as_deref() == Some("main-checkout") {
                return "DECISIONS.json".to_string();
            }
            if planning_core::worktree_recognized(&candidate) {
                return register_file(&candidate);
            }
        }
    }
    "DECISIONS.json".to_string()
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
                format!("no register at {path} — point DECISIONS_JSON at one"),
                EX_NOINPUT,
            )
        }
    }
    Ok(text)
}

/// Migrates transparently on read: a register at the wrong version is backed
/// up and converted in place here, rather than refusing and naming a
/// separate `migrate` command, since this register has no pre-existing
/// unversioned file to convert from in the first place.
fn read(path: &str) -> Result<Register, Failure> {
    let text = read_text(path)?;
    let loose: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => return fail(format!("{path}: {error}"), EX_DATAERR),
    };
    let claimed = migrate::claimed_version(&loose);
    if migrate::is_current(&claimed) {
        return match serde_json::from_value(loose) {
            Ok(register) => Ok(register),
            Err(error) => fail(format!("{path}: {error}"), EX_DATAERR),
        };
    }

    let backup = migrate::backup_path(path, &claimed);
    if !std::path::Path::new(&backup).exists() && std::fs::write(&backup, &text).is_err() {
        return fail(format!("cannot write the backup at {backup}"), EX_IOERR);
    }
    let (carried, _archived, unconvertible) = migrate::attempt(&loose);
    let register = migrate::rebuilt(&loose, carried);
    if !unconvertible.is_empty() {
        eprint!("{}", migrate::instructions(&backup, &unconvertible));
    }
    Ok(register)
}

/// Two-space pretty JSON with a trailing newline, written through a temp
/// file in the target's own directory, the same pattern bug-report's and
/// todo's own `write` use.
fn write(path: &str, register: &Register) -> Result<(), Failure> {
    let mut text = match serde_json::to_string_pretty(register) {
        Ok(text) => text,
        Err(error) => return fail(format!("cannot serialise: {error}"), EX_IOERR),
    };
    text.push('\n');

    let directory = std::path::Path::new(path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let temp = directory.join(format!(".decisions-write.{}", std::process::id()));

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
    fn help_prints_usage_and_exits_zero() {
        assert!(run(&[]).is_ok());
        assert!(run(&["--help".to_string()]).is_ok());
        assert!(run(&["add".to_string(), "--help".to_string()]).is_ok());
    }

    #[test]
    fn an_unknown_command_is_refused() {
        let err = run(&["bogus".to_string()]).unwrap_err();
        assert_eq!(err.code, EX_USAGE);
    }

    #[test]
    fn an_explicit_file_wins_over_the_environment() {
        std::env::set_var("DECISIONS_JSON", "/from/the/environment.json");
        assert_eq!(
            resolve_path(Some("/on/the/command/line.json".into())),
            "/on/the/command/line.json"
        );
        assert_eq!(resolve_path(None), "/from/the/environment.json");
    }

    #[test]
    fn option_values_split_on_the_first_colon() {
        let argv = vec![
            "add".to_string(),
            "--option".to_string(),
            "a:Yes, really".to_string(),
        ];
        let options = option_values(&argv).unwrap();
        assert_eq!(options[0].letter, "a");
        assert_eq!(options[0].label, "Yes, really");
    }
}
