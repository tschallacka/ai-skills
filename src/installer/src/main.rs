// MODE: DEV
// PACKAGE: PROD
//! installer — the binary a downloaded release package hands off to once the
//! tiny curl-piped bootstrap script has fetched and extracted it. Runs from
//! inside the extracted tree; knows nothing about fetching itself.
//!
//! This is an early slice, not full parity with install.sh yet: no
//! interactive TUI, no per-agent permission grants, no MCP registration, no
//! plan migration. Skill discovery (discover.rs) still finds any directory
//! with a SKILL.md, looser than install.sh's hand-maintained SKILL_NAMES
//! table (no hidden-skill support yet); manifest.rs supplies descriptions
//! and the --agent shortcut for the ones it knows about.

mod backup;
mod digest;
mod discover;
mod install;
mod manifest;
mod permissions;

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
  installer grant-claude-permissions --scripts DIR --plans DIR --tmp DIR
  installer grant-opencode-permissions --scripts DIR --plans DIR --tmp DIR
                     grant Claude Code / opencode read/write on the planning
                     skill's own scripts, plan root, and tmp directory
  installer --help

--agent NAME is one of: claude, codex, opencode, universal, openclaw, cline
             (resolves to that agent's own skills directory under $HOME).
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
        Some("grant-claude-permissions") => run_grant_claude_permissions(&argv[1..]),
        Some("grant-opencode-permissions") => run_grant_opencode_permissions(&argv[1..]),
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

struct GrantArgs {
    scripts: String,
    plans: String,
    tmp: String,
}

fn parse_grant_args(command: &str, argv: &[String]) -> Result<GrantArgs, String> {
    let mut scripts: Option<String> = None;
    let mut plans: Option<String> = None;
    let mut tmp: Option<String> = None;

    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--scripts" => {
                i += 1;
                scripts = Some(argv.get(i).ok_or("--scripts needs a value")?.clone());
            }
            "--plans" => {
                i += 1;
                plans = Some(argv.get(i).ok_or("--plans needs a value")?.clone());
            }
            "--tmp" => {
                i += 1;
                tmp = Some(argv.get(i).ok_or("--tmp needs a value")?.clone());
            }
            other => return Err(format!("{command}: unknown option: {other}")),
        }
        i += 1;
    }
    Ok(GrantArgs {
        scripts: scripts.ok_or(format!("{command}: --scripts is required"))?,
        plans: plans.ok_or(format!("{command}: --plans is required"))?,
        tmp: tmp.ok_or(format!("{command}: --tmp is required"))?,
    })
}

fn home_dir(command: &str) -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| format!("{command}: needs $HOME set"))
}

fn run_grant_claude_permissions(argv: &[String]) -> Result<ExitCode, String> {
    let args = parse_grant_args("grant-claude-permissions", argv)?;
    let home = home_dir("grant-claude-permissions")?;

    match permissions::claude_planning_permissions(&args.scripts, &args.plans, &args.tmp, &home)
        .map_err(|e| e.to_string())?
    {
        permissions::PermissionOutcome::NoConfigFile => {
            println!("claude-code: no settings.json found; skipped");
        }
        permissions::PermissionOutcome::AlreadyPresent => {
            println!("claude-code: permissions already present");
        }
        permissions::PermissionOutcome::Added(entries) => {
            println!("claude-code: added to permissions.allow:");
            for entry in entries {
                println!("  - {entry}");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run_grant_opencode_permissions(argv: &[String]) -> Result<ExitCode, String> {
    let args = parse_grant_args("grant-opencode-permissions", argv)?;
    let home = home_dir("grant-opencode-permissions")?;

    match permissions::opencode_planning_permissions(&args.scripts, &args.plans, &args.tmp, &home)
        .map_err(|e| e.to_string())?
    {
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
                println!("opencode: permissions already present");
            } else {
                println!("opencode: allowed:");
                for entry in added {
                    println!("  - {entry}");
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
