// MODE: DEV
// PACKAGE: PROD
//! installer — the binary a downloaded release package hands off to once the
//! tiny curl-piped bootstrap script has fetched and extracted it. Runs from
//! inside the extracted tree; knows nothing about fetching itself.
//!
//! This is an early slice, not full parity with install.sh yet: no
//! interactive TUI, no plan migration. Skill discovery (discover.rs) still
//! finds any directory with a SKILL.md, looser than install.sh's hand-
//! maintained SKILL_NAMES table (no hidden-skill support yet); manifest.rs
//! supplies descriptions and the --agent shortcut for the ones it knows
//! about; permissions.rs and mcp.rs cover the planning/worktree permission
//! grants and mcp-mode registration for claude/codex/opencode.

mod backup;
mod digest;
mod discover;
mod install;
mod manifest;
mod mcp;
mod permissions;
mod plan_migration;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
installer — installs skills from the release tree it ships inside.

Usage:
  installer --platform                     print the resolved target triple
  installer list [--source DIR]            print every discovered skill
  installer install (--target DIR | --agent NAME)
                     (--all | --skill NAME [--skill NAME ...])
                     [--source DIR]
  installer grant-permissions --agent NAME (--scripts DIR --plans DIR --tmp DIR | --worktrees DIR)
                     grant that agent read/write on the planning skill's own
                     scripts/plan-root/tmp directory, or on a worktree root
  installer mcp-register --agent NAME --name NAME --path PATH
                     register PATH as an mcp-mode stdio server named NAME
  installer mcp-unregister --agent NAME --name NAME --dir DIR
                     remove NAME's registration, only if it points inside DIR
  installer migrate-plans --target-root DIR [--target-root DIR ...]
                     move plans out of each DIR's old planning/plans into
                     the single portable plan root
  installer --help

--agent NAME is one of: claude, codex, opencode, universal, openclaw, cline
             (resolves to that agent's own skills directory under $HOME).
             grant-permissions only knows claude, codex and opencode.
";

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match run(&argv) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("installer: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(argv: &[String]) -> Result<ExitCode, String> {
    match argv.first().map(String::as_str) {
        None | Some("--help") | Some("-h") => {
            print!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        Some("--platform") => {
            let target = installer_platform::current().map_err(|e| e.to_string())?;
            println!("{target}");
            Ok(ExitCode::SUCCESS)
        }
        Some("list") => run_list(&argv[1..]),
        Some("install") => run_install(&argv[1..]),
        Some("grant-permissions") => run_grant_permissions(&argv[1..]),
        Some("mcp-register") => run_mcp_register(&argv[1..]),
        Some("mcp-unregister") => run_mcp_unregister(&argv[1..]),
        Some("migrate-plans") => run_migrate_plans(&argv[1..]),
        Some(other) => Err(format!("unknown command: {other}")),
    }
}

/// The extracted release tree's own root, next to wherever this binary
/// itself was run from: skill directories sit directly under it
/// (`todo/SKILL.md`, `bug-report/SKILL.md`, ...), matching what
/// `installer/build-release.sh` actually packs. `--source` overrides it;
/// nothing else needs to know the layout the bootstrap script's tar
/// extracted.
fn default_source() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("resolving own path: {e}"))?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "installer binary has no parent directory".to_string())
}

fn resolve_source(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    match explicit {
        Some(s) => Ok(s),
        None => default_source(),
    }
}

fn run_list(argv: &[String]) -> Result<ExitCode, String> {
    let mut source: Option<PathBuf> = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--source" => {
                i += 1;
                source = Some(PathBuf::from(argv.get(i).ok_or("--source needs a value")?));
            }
            other => return Err(format!("list: unknown option: {other}")),
        }
        i += 1;
    }
    let source = resolve_source(source)?;
    let discovered = discover::discover_skills(&source).map_err(|e| e.to_string())?;
    for skill in discovered {
        match manifest::known_skill(&skill) {
            Some(known) => println!("{skill}  -- {}", known.description),
            None => println!("{skill}"),
        }
    }
    Ok(ExitCode::SUCCESS)
}

struct InstallArgs {
    skills: Vec<String>,
    all: bool,
    source: Option<PathBuf>,
    target: Option<PathBuf>,
    agent: Option<String>,
}

fn parse_install_args(argv: &[String]) -> Result<InstallArgs, String> {
    let mut skills = Vec::new();
    let mut all = false;
    let mut source: Option<PathBuf> = None;
    let mut target: Option<PathBuf> = None;
    let mut agent: Option<String> = None;

    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--skill" => {
                i += 1;
                skills.push(argv.get(i).ok_or("--skill needs a value")?.clone());
            }
            "--all" => all = true,
            "--source" => {
                i += 1;
                source = Some(PathBuf::from(argv.get(i).ok_or("--source needs a value")?));
            }
            "--target" => {
                i += 1;
                target = Some(PathBuf::from(argv.get(i).ok_or("--target needs a value")?));
            }
            "--agent" => {
                i += 1;
                agent = Some(argv.get(i).ok_or("--agent needs a value")?.clone());
            }
            other => return Err(format!("install: unknown option: {other}")),
        }
        i += 1;
    }
    Ok(InstallArgs {
        skills,
        all,
        source,
        target,
        agent,
    })
}

/// `--agent NAME` resolves to that agent's own directory under $HOME
/// (manifest::AGENTS), matching install.sh's TARGET_PATHS; `--target DIR`
/// names a directory outright. Exactly one of the two selects where skills
/// land.
fn resolve_target(target: Option<PathBuf>, agent: Option<String>) -> Result<PathBuf, String> {
    match (target, agent) {
        (Some(_), Some(_)) => {
            Err("install: --target and --agent are mutually exclusive".to_string())
        }
        (Some(t), None) => Ok(t),
        (None, Some(kind)) => {
            let known = manifest::known_agent(&kind).ok_or_else(|| {
                let choices: Vec<_> = manifest::AGENTS
                    .iter()
                    .map(|a| format!("{} ({})", a.kind, a.name))
                    .collect();
                format!(
                    "install: unknown --agent {kind}; known agents are: {}",
                    choices.join(", ")
                )
            })?;
            let home = std::env::var("HOME")
                .map_err(|_| "install: --agent needs $HOME set".to_string())?;
            Ok(PathBuf::from(home).join(known.home_suffix))
        }
        (None, None) => Err("install: --target or --agent is required".to_string()),
    }
}

fn run_install(argv: &[String]) -> Result<ExitCode, String> {
    let args = parse_install_args(argv)?;
    if args.all && !args.skills.is_empty() {
        return Err("install: --all and --skill are mutually exclusive".to_string());
    }
    if !args.all && args.skills.is_empty() {
        return Err("install: --all or at least one --skill is required".to_string());
    }
    let source = resolve_source(args.source)?;
    let target = resolve_target(args.target, args.agent)?;

    let skills = if args.all {
        discover::discover_skills(&source).map_err(|e| e.to_string())?
    } else {
        args.skills
    };
    if skills.is_empty() {
        return Err(format!(
            "install: no skills found under {}",
            source.display()
        ));
    }

    for skill in &skills {
        install::install_skill(&source, skill, &target).map_err(|e| e.to_string())?;
        println!("installed {skill} -> {}", target.join(skill).display());
    }
    Ok(ExitCode::SUCCESS)
}

enum GrantTarget {
    Planning {
        scripts: String,
        plans: String,
        tmp: String,
    },
    Worktrees {
        worktrees: String,
    },
}

struct GrantArgs {
    agent: String,
    target: GrantTarget,
}

fn parse_grant_args(argv: &[String]) -> Result<GrantArgs, String> {
    let mut agent: Option<String> = None;
    let mut scripts: Option<String> = None;
    let mut plans: Option<String> = None;
    let mut tmp: Option<String> = None;
    let mut worktrees: Option<String> = None;

    let mut i = 0;
    while i < argv.len() {
        macro_rules! value {
            () => {{
                i += 1;
                argv.get(i)
                    .ok_or_else(|| format!("{} needs a value", argv[i - 1]))?
                    .clone()
            }};
        }
        match argv[i].as_str() {
            "--agent" => agent = Some(value!()),
            "--scripts" => scripts = Some(value!()),
            "--plans" => plans = Some(value!()),
            "--tmp" => tmp = Some(value!()),
            "--worktrees" => worktrees = Some(value!()),
            other => return Err(format!("grant-permissions: unknown option: {other}")),
        }
        i += 1;
    }
    let agent = agent.ok_or("grant-permissions: --agent is required")?;
    let target = match (scripts, plans, tmp, worktrees) {
        (Some(scripts), Some(plans), Some(tmp), None) => GrantTarget::Planning {
            scripts,
            plans,
            tmp,
        },
        (None, None, None, Some(worktrees)) => GrantTarget::Worktrees { worktrees },
        _ => return Err(
            "grant-permissions: pass either --scripts/--plans/--tmp together, or --worktrees alone"
                .to_string(),
        ),
    };
    Ok(GrantArgs { agent, target })
}

fn print_permission_outcome(
    agent: &str,
    already_present: &str,
    outcome: permissions::PermissionOutcome,
) {
    match outcome {
        permissions::PermissionOutcome::NoConfigFile => {
            println!("{agent}: no settings.json found; skipped");
        }
        permissions::PermissionOutcome::AlreadyPresent => {
            println!("{agent}: {already_present}");
        }
        permissions::PermissionOutcome::Added(entries) => {
            println!("{agent}: added to permissions.allow:");
            for entry in entries {
                println!("  - {entry}");
            }
        }
    }
}

fn print_opencode_outcome(already_present: &str, outcome: permissions::OpencodePermissionOutcome) {
    match outcome {
        permissions::OpencodePermissionOutcome::NotStrictJson => {
            println!("opencode: config is not strict JSON; add the rules by hand");
        }
        permissions::OpencodePermissionOutcome::Merged {
            legacy_removed,
            added,
        } => {
            if legacy_removed {
                println!("opencode: removed invalid claude-style permission.allow list");
            }
            if added.is_empty() {
                println!("opencode: {already_present}");
            } else {
                println!("opencode: allowed:");
                for entry in added {
                    println!("  - {entry}");
                }
            }
        }
    }
}

/// install.sh's `codex_write_fresh_roots` always prints its caller's own
/// "done" label at the end, even on a fresh create or prepend -- `label`
/// carries that same per-context wording through (planning says "writable_
/// roots already present", worktrees says "worktree grant already in
/// place", regardless of which of those two branches actually ran).
fn print_codex_outcome(label: &str, outcome: permissions::CodexOutcome) {
    match outcome {
        permissions::CodexOutcome::Created => {
            println!("codex: created config.toml");
            println!("codex: {label}");
        }
        permissions::CodexOutcome::Prepended | permissions::CodexOutcome::AlreadyPresent => {
            println!("codex: {label}");
        }
        permissions::CodexOutcome::Appended(paths) => {
            println!("codex: added to writable_roots:");
            for path in paths {
                println!("  - {path}");
            }
        }
        permissions::CodexOutcome::NotSingleLineArray => {
            println!("codex: writable_roots is not a single-line array; add these by hand");
        }
    }
}

fn run_grant_permissions(argv: &[String]) -> Result<ExitCode, String> {
    let args = parse_grant_args(argv)?;
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "grant-permissions: needs $HOME set".to_string())?;

    match (args.agent.as_str(), &args.target) {
        (
            "claude",
            GrantTarget::Planning {
                scripts,
                plans,
                tmp,
            },
        ) => {
            let outcome = permissions::claude_planning_permissions(scripts, plans, tmp, &home)
                .map_err(|e| e.to_string())?;
            print_permission_outcome("claude-code", "permissions already present", outcome);
        }
        ("claude", GrantTarget::Worktrees { worktrees }) => {
            let outcome = permissions::claude_worktrees_permissions(worktrees, &home)
                .map_err(|e| e.to_string())?;
            print_permission_outcome("claude-code", "worktree grant already in place", outcome);
        }
        (
            "opencode",
            GrantTarget::Planning {
                scripts,
                plans,
                tmp,
            },
        ) => {
            let outcome = permissions::opencode_planning_permissions(scripts, plans, tmp, &home)
                .map_err(|e| e.to_string())?;
            print_opencode_outcome("permissions already present", outcome);
        }
        ("opencode", GrantTarget::Worktrees { worktrees }) => {
            let outcome = permissions::opencode_worktrees_permissions(worktrees, &home)
                .map_err(|e| e.to_string())?;
            print_opencode_outcome("worktree grant already in place", outcome);
        }
        (
            "codex",
            GrantTarget::Planning {
                scripts,
                plans,
                tmp,
            },
        ) => {
            let outcome = permissions::codex_planning_permissions(scripts, plans, tmp, &home)
                .map_err(|e| e.to_string())?;
            print_codex_outcome("writable_roots already present", outcome);
        }
        ("codex", GrantTarget::Worktrees { worktrees }) => {
            let outcome = permissions::codex_worktrees_permissions(worktrees, &home)
                .map_err(|e| e.to_string())?;
            print_codex_outcome("worktree grant already in place", outcome);
        }
        (other, _) => {
            return Err(format!(
            "grant-permissions: unknown --agent {other}; known agents are: claude, codex, opencode"
        ))
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn known_mcp_agent(command: &str, agent: &str) -> Result<(), String> {
    match agent {
        "claude" | "codex" | "opencode" => Ok(()),
        other => Err(format!(
            "{command}: unknown --agent {other}; known agents are: claude, codex, opencode"
        )),
    }
}

struct McpRegisterArgs {
    agent: String,
    name: String,
    path: String,
}

fn parse_mcp_register_args(argv: &[String]) -> Result<McpRegisterArgs, String> {
    let mut agent: Option<String> = None;
    let mut name: Option<String> = None;
    let mut path: Option<String> = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--agent" => {
                i += 1;
                agent = Some(argv.get(i).ok_or("--agent needs a value")?.clone());
            }
            "--name" => {
                i += 1;
                name = Some(argv.get(i).ok_or("--name needs a value")?.clone());
            }
            "--path" => {
                i += 1;
                path = Some(argv.get(i).ok_or("--path needs a value")?.clone());
            }
            other => return Err(format!("mcp-register: unknown option: {other}")),
        }
        i += 1;
    }
    Ok(McpRegisterArgs {
        agent: agent.ok_or("mcp-register: --agent is required")?,
        name: name.ok_or("mcp-register: --name is required")?,
        path: path.ok_or("mcp-register: --path is required")?,
    })
}

fn run_mcp_register(argv: &[String]) -> Result<ExitCode, String> {
    let args = parse_mcp_register_args(argv)?;
    known_mcp_agent("mcp-register", &args.agent)?;
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "mcp-register: needs $HOME set".to_string())?;

    match mcp::register_for_kind(&args.agent, &args.name, &args.path, &home)
        .map_err(|e| e.to_string())?
    {
        mcp::RegisterOutcome::Registered => {
            println!("{}: registered MCP server {}", args.agent, args.name);
        }
        mcp::RegisterOutcome::Manual => {
            for line in mcp::manual_instructions(&args.agent, &args.name, &args.path) {
                println!("{line}");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

struct McpUnregisterArgs {
    agent: String,
    name: String,
    dir: PathBuf,
}

fn parse_mcp_unregister_args(argv: &[String]) -> Result<McpUnregisterArgs, String> {
    let mut agent: Option<String> = None;
    let mut name: Option<String> = None;
    let mut dir: Option<PathBuf> = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--agent" => {
                i += 1;
                agent = Some(argv.get(i).ok_or("--agent needs a value")?.clone());
            }
            "--name" => {
                i += 1;
                name = Some(argv.get(i).ok_or("--name needs a value")?.clone());
            }
            "--dir" => {
                i += 1;
                dir = Some(PathBuf::from(argv.get(i).ok_or("--dir needs a value")?));
            }
            other => return Err(format!("mcp-unregister: unknown option: {other}")),
        }
        i += 1;
    }
    Ok(McpUnregisterArgs {
        agent: agent.ok_or("mcp-unregister: --agent is required")?,
        name: name.ok_or("mcp-unregister: --name is required")?,
        dir: dir.ok_or("mcp-unregister: --dir is required")?,
    })
}

fn run_mcp_unregister(argv: &[String]) -> Result<ExitCode, String> {
    let args = parse_mcp_unregister_args(argv)?;
    known_mcp_agent("mcp-unregister", &args.agent)?;
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "mcp-unregister: needs $HOME set".to_string())?;

    let removed = mcp::unregister_for_kind(&args.agent, &args.name, &args.dir, &home)
        .map_err(|e| e.to_string())?;
    if removed {
        println!("{}: removed MCP server {}", args.agent, args.name);
    } else {
        println!(
            "{}: no registration owned by this install found for {}",
            args.agent, args.name
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn run_migrate_plans(argv: &[String]) -> Result<ExitCode, String> {
    let mut target_roots = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--target-root" => {
                i += 1;
                target_roots.push(PathBuf::from(
                    argv.get(i).ok_or("--target-root needs a value")?,
                ));
            }
            other => return Err(format!("migrate-plans: unknown option: {other}")),
        }
        i += 1;
    }
    if target_roots.is_empty() {
        return Err("migrate-plans: at least one --target-root is required".to_string());
    }
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "migrate-plans: needs $HOME set".to_string())?;

    let outcome =
        plan_migration::migrate_legacy_plans(&target_roots, &home).map_err(|e| e.to_string())?;
    for plan in &outcome.migrated {
        println!("Migrated plan: -> {}", plan.display());
    }
    for (plan, reason) in &outcome.blocked {
        println!("Plan migration blocked: {}: {reason}", plan.display());
    }
    println!("Portable plan root ready: {}", outcome.plan_root.display());
    Ok(ExitCode::SUCCESS)
}
