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
mod integration;
mod plan_migration;
mod plugins;
mod requirements;
mod tools;
mod ui;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
installer — installs skills from the release tree it ships inside.

Usage:
  installer --platform                     print the resolved target triple
  installer list [--source DIR]            print every discovered skill
  installer install (--target DIR [--target DIR ...] | --agent NAME [--agent NAME ...])
                     (--all | --skill NAME [--skill NAME ...])
                     [--source DIR] [--integration MODE|SKILL=MODE ...]
                     [--editor-integration skill|mcp] [--yes]
                     [--dev-build]
                     runs the planning/worktrees/interactive-shell/editor
                     permission prompts unless --yes auto-answers them;
                     --dev-build also ships MODE:DEV-marked files (tests,
                     maintainer docs) instead of filtering them out, for
                     installing straight from a raw checkout during dev
  installer grant-permissions --agent NAME (--scripts DIR --plans DIR --tmp DIR | --worktrees DIR | --bins DIR)
                     grant that agent read/write on the planning skill's own
                     scripts/plan-root/tmp directory, or on a worktree root
  installer mcp-register --agent NAME --name NAME --path PATH
                     register PATH as an mcp-mode stdio server named NAME
  installer mcp-unregister --agent NAME --name NAME --dir DIR
                     remove NAME's registration, only if it points inside DIR
  installer migrate-plans --target-root DIR [--target-root DIR ...]
                     move plans out of each DIR's old planning/plans into
                     the single portable plan root
  installer install-tui-hint-plugin --agent claude|opencode --source DIR [--target DIR]
  installer install-editor-gate-plugin --source DIR --target DIR
                     install the vendor-shipped plugin that rides with
                     interactive-shell / ai-text-editor
  installer set-claude-env --key KEY --value VALUE
                     merge one env.KEY setting into Claude's settings.json
  installer interactive (--target DIR | --agent NAME) [--source DIR]
                     [--integration MODE|SKILL=MODE ...] [--yes] [--dev-build]
                     full-screen skill picker; installs the confirmed
                     selection, or does nothing if the user quits
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
        Some("interactive") => run_interactive(&argv[1..]),
        Some("grant-permissions") => run_grant_permissions(&argv[1..]),
        Some("mcp-register") => run_mcp_register(&argv[1..]),
        Some("mcp-unregister") => run_mcp_unregister(&argv[1..]),
        Some("migrate-plans") => run_migrate_plans(&argv[1..]),
        Some("install-tui-hint-plugin") => run_install_tui_hint_plugin(&argv[1..]),
        Some("install-editor-gate-plugin") => run_install_editor_gate_plugin(&argv[1..]),
        Some("set-claude-env") => run_set_claude_env(&argv[1..]),
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
    /// Repeatable: `run_install` installs into every resolved root, matching
    /// install.sh's own multi-root `SELECTED_TARGET_PATHS`.
    targets: Vec<PathBuf>,
    agents: Vec<String>,
    integration: Vec<String>,
    yes: bool,
    dev_build: bool,
}

fn parse_install_args(argv: &[String]) -> Result<InstallArgs, String> {
    let mut skills = Vec::new();
    let mut all = false;
    let mut source: Option<PathBuf> = None;
    let mut targets: Vec<PathBuf> = Vec::new();
    let mut agents: Vec<String> = Vec::new();
    let mut integration = Vec::new();
    let mut yes = false;
    let mut dev_build = false;

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
                targets.push(PathBuf::from(argv.get(i).ok_or("--target needs a value")?));
            }
            "--agent" => {
                i += 1;
                agents.push(argv.get(i).ok_or("--agent needs a value")?.clone());
            }
            "--integration" => {
                i += 1;
                integration.push(argv.get(i).ok_or("--integration needs a mode, or skill=mode")?.clone());
            }
            "--editor-integration" => {
                i += 1;
                let mode = argv.get(i).ok_or("--editor-integration needs skill or mcp")?;
                integration.push(format!("ai-text-editor={mode}"));
            }
            "--yes" => yes = true,
            "--dev-build" => dev_build = true,
            other => return Err(format!("install: unknown option: {other}")),
        }
        i += 1;
    }
    Ok(InstallArgs {
        skills,
        all,
        source,
        targets,
        agents,
        integration,
        yes,
        dev_build,
    })
}

/// A yes/no prompt gate, ported from install.sh's `confirm()`/`ask()`:
/// `--yes` (or a prior "a"/"all" answer, `YES_ALL` there) answers every
/// question without reading stdin at all -- the flag headless runs need so
/// an unattended install cannot block on a question nobody will answer.
struct Confirms {
    yes: bool,
}

impl Confirms {
    fn new(yes: bool) -> Self {
        Confirms { yes }
    }

    /// Prints `prompt` to stderr (matching install.sh's `ask`, which never
    /// writes a prompt to stdout) and reads one line from stdin. `y`/`yes`
    /// answers this question only; `a`/`all` answers it and every question
    /// after it for the rest of the run, same as install.sh's `YES_ALL`.
    /// A read error (no stdin at all, e.g. under `curl | bash`) reads as
    /// "no" rather than blocking -- the same failure mode `ask`'s own `read`
    /// has on a closed stdin.
    fn ask(&mut self, prompt: &str) -> bool {
        if self.yes {
            return true;
        }
        eprint!("{prompt} [y/N/a] ");
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Ok(0) | Err(_) => return false, // EOF or no stdin at all
            Ok(_) => {}
        }
        match line.trim().to_lowercase().as_str() {
            "y" | "yes" => true,
            "a" | "all" => {
                self.yes = true;
                true
            }
            _ => false,
        }
    }
}

/// A resolved `--integration` selection: any number of `skill=mode` choices
/// plus at most one run-wide bare-mode default -- ported from
/// installer/src/05-config.sh's `record_integration`/`record_skill_integration`
/// and `INTEGRATION_SELECTION`/`INTEGRATION_DEFAULT`. A later `--integration`
/// for the same skill overwrites an earlier one (the usual last-flag-wins CLI
/// convention); install.sh gets the same result through a different
/// mechanism (bash 3.2 has no associative arrays, so it prepends records to a
/// list and reads the first match), so this does not reproduce that
/// mechanism, only its outcome.
#[derive(Default)]
struct IntegrationSelection {
    per_skill: std::collections::HashMap<String, String>,
    default_mode: Option<String>,
}

impl IntegrationSelection {
    fn choice_for(&self, skill: &str) -> Option<&str> {
        self.per_skill
            .get(skill)
            .map(String::as_str)
            .or(self.default_mode.as_deref())
    }
}

/// Parses and validates every `--integration` argument against `source`'s
/// own declared modes -- a `skill=mode` naming a mode that skill does not
/// offer is refused by name, same as install.sh's `record_skill_integration`
/// refusing at the door rather than failing silently mid-install.
///
/// `EDITOR_INTEGRATION` (install.sh's older, ai-text-editor-only env-var
/// spelling of the same choice) is folded in first, so an explicit
/// `--integration`/`--editor-integration` on the command line still
/// overrides it -- same precedence install.sh's own INTEGRATION_SELECTION
/// prepend order gives the CLI flag over the env var.
fn build_integration_selection(
    source: &Path,
    raw: &[String],
) -> Result<IntegrationSelection, String> {
    let mut selection = IntegrationSelection::default();
    let env_editor_integration = std::env::var("EDITOR_INTEGRATION")
        .ok()
        .filter(|v| !v.is_empty())
        .map(|mode| format!("ai-text-editor={mode}"));
    let combined: Vec<String> = env_editor_integration.into_iter().chain(raw.iter().cloned()).collect();
    for arg in &combined {
        let (skill, mode) = match arg.split_once('=') {
            Some((s, m)) => (Some(s.to_string()), m.to_string()),
            None => (None, arg.clone()),
        };
        if mode.is_empty() {
            return Err("--integration needs a mode, or skill=mode".to_string());
        }
        if let Some(skill) = &skill {
            let offered = integration::modes(source, skill);
            if offered.is_empty() {
                return Err(format!("{skill} offers no integration modes to choose between"));
            }
            if !offered.contains(&mode) {
                return Err(format!(
                    "{skill} has no {mode} integration; it offers: {}",
                    offered.join(" ")
                ));
            }
        }
        match skill {
            Some(skill) => {
                selection.per_skill.insert(skill, mode);
            }
            None => selection.default_mode = Some(mode),
        }
    }
    Ok(selection)
}

/// `--agent NAME` resolves to that agent's own directory under $HOME
/// (manifest::AGENTS), matching install.sh's TARGET_PATHS; `--target DIR`
/// names a directory outright. Exactly one of the two selects where skills
/// land.
/// `--agent NAME` resolves to that agent's own directory under $HOME
/// (manifest::AGENTS), matching install.sh's TARGET_PATHS; `--target DIR`
/// names a directory outright. Exactly one of the two selects where skills
/// land. Also returns the resolved agent *kind* when known -- either named
/// directly by `--agent`, or inferred from an explicit `--target` that
/// happens to match one of AGENTS' own paths under $HOME, the same way
/// install.sh's `agent_kind_for_root` works by path alone regardless of how
/// the path was chosen. `None` means a custom target this installer has no
/// grants for, same as install.sh's "custom" row.
fn resolve_target_and_kind(
    target: Option<PathBuf>,
    agent: Option<String>,
) -> Result<(PathBuf, Option<String>), String> {
    match (target, agent) {
        (Some(_), Some(_)) => {
            Err("install: --target and --agent are mutually exclusive".to_string())
        }
        (Some(t), None) => {
            let kind = home_dir_opt().and_then(|home| {
                manifest::AGENTS
                    .iter()
                    .find(|a| home.join(a.home_suffix) == t)
                    .map(|a| a.kind.to_string())
            });
            Ok((t, kind))
        }
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
            Ok((
                PathBuf::from(home).join(known.home_suffix),
                Some(known.kind.to_string()),
            ))
        }
        (None, None) => Err("install: --target or --agent is required".to_string()),
    }
}

/// The multi-root form `run_install` uses: `--target`/`--agent` are each
/// repeatable there (not in `interactive`, whose picker has no root-
/// selection UI of its own to drive more than one), matching install.sh's
/// own `SELECTED_TARGET_PATHS` array and its main loop's `for root in
/// SELECTED_TARGET_PATHS; do for skill in SELECTED_SKILLS; do install_skill`
/// nesting -- every skill installs into every named root. Still mutually
/// exclusive as families (`--target` and `--agent` cannot both be given),
/// same as the single-root form. A duplicate root (typed twice, or two
/// `--agent` names that happen to share a home directory) collapses to one
/// entry so nothing installs, registers, or prompts twice for the same
/// destination.
fn resolve_targets_and_kinds(
    targets: Vec<PathBuf>,
    agents: Vec<String>,
) -> Result<Vec<(PathBuf, Option<String>)>, String> {
    if !targets.is_empty() && !agents.is_empty() {
        return Err("install: --target and --agent are mutually exclusive".to_string());
    }
    let mut resolved = Vec::new();
    if !targets.is_empty() {
        for t in targets {
            resolved.push(resolve_target_and_kind(Some(t), None)?);
        }
    } else if !agents.is_empty() {
        for a in agents {
            resolved.push(resolve_target_and_kind(None, Some(a))?);
        }
    } else {
        return Err("install: --target or --agent is required".to_string());
    }
    let mut deduped: Vec<(PathBuf, Option<String>)> = Vec::new();
    for entry in resolved {
        if !deduped.iter().any(|(p, _)| *p == entry.0) {
            deduped.push(entry);
        }
    }
    Ok(deduped)
}

fn home_dir_opt() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// Accumulates what a run actually did, for the final `== Summary ==` block
/// -- ported from install.sh's `SUMMARY_LINES`/`summary_add` and
/// `summary_blocked_block`/`replay_commands`. install.sh's own per-line
/// soft-requirement/dev-build/integration-mode suffixes
/// (`summary_soft_note`/`summary_dev_build_note`/`summary_integration_note`)
/// are not reproduced here; this covers the two lines install.sh's own
/// comment calls the whole point of the block -- what was installed, and,
/// for anything blocked on a hard requirement, the exact command that
/// retries once it's met.
#[derive(Default)]
struct Summary {
    installed: Vec<String>,
    platform_blocked: Vec<(String, String)>,
    hard_blocked: Vec<HardBlocked>,
}

struct HardBlocked {
    skill: String,
    /// (label, install-hint lines) for every unmet hard requirement, in
    /// `requirements::skill_status`'s own order.
    unmet: Vec<(String, Vec<String>)>,
}

impl Summary {
    /// `roots` and `yes` are threaded in at print time, not recorded per
    /// skill: install.sh's own `replay_commands` reads `SELECTED_TARGET_PATHS`
    /// and `YES` fresh when the summary prints, not when the skill was
    /// blocked, since the whole run shares one root list and one --yes.
    fn print(&self, roots: &[PathBuf], yes: bool) {
        if self.installed.is_empty() && self.platform_blocked.is_empty() && self.hard_blocked.is_empty() {
            return;
        }
        println!();
        println!("== Summary ==");
        for line in &self.installed {
            println!("{line}");
        }
        for (skill, reason) in &self.platform_blocked {
            println!("Skipped:   {skill} -- {reason}, nothing was written");
        }
        let replay_prefix = std::env::args()
            .next()
            .unwrap_or_else(|| "installer".to_string());
        for blocked in &self.hard_blocked {
            println!(
                "Skipped:   {} -- a hard requirement is missing, nothing was written",
                blocked.skill
            );
            println!("To install {} once its requirements are met:", blocked.skill);
            let mut step = 1;
            for (label, hint) in &blocked.unmet {
                println!("  {step}. install {label}:");
                for line in hint {
                    println!("  {line}");
                }
                step += 1;
            }
            println!("  {step}. replay this run:");
            for root in roots {
                let yes_flag = if yes { " --yes" } else { "" };
                println!(
                    "  {replay_prefix} install --skill {} --target {}{yes_flag}",
                    blocked.skill,
                    root.display()
                );
            }
        }
    }
}

/// A skill missing a hard requirement is skipped rather than installed and
/// then left half-usable -- same rule as install.sh's
/// summary_blocked_block/RUNTIME_BLOCKED_SKILLS: "Skipped: %s -- a hard
/// requirement is missing, nothing was written". The interactive picker
/// already keeps a Blocked skill out of `skills` before this is called
/// (`PickerState::toggle` refuses to select one); this is the same rule
/// applied to a name that arrived directly via `--skill`/`--all`, which never
/// passed through the picker at all.
///
/// `manifest::skill_unsupported_here` is checked first, same order as
/// install.sh's own per-skill loop: a platform-unsupported skill has no
/// requirements worth checking and nothing worth replaying, so it gets its
/// own reason rather than being reported as a missing-tool block.
fn install_selected_skills(
    source: &Path,
    target: &Path,
    skills: &[String],
    integration_selection: &IntegrationSelection,
    dev_build: bool,
    summary: &mut Summary,
) -> Result<Vec<String>, String> {
    let mut installed = Vec::with_capacity(skills.len());
    for skill in skills {
        if let Some(reason) = manifest::skill_unsupported_here(skill) {
            println!("Skipped: {skill} -- {reason}, nothing was written");
            summary.platform_blocked.push((skill.clone(), reason.to_string()));
            continue;
        }
        let status = requirements::skill_status(source, skill);
        if status.state == requirements::SkillState::Blocked {
            let reason = status.blocker.clone().unwrap_or_else(|| "a required tool".to_string());
            println!("Skipped: {skill} -- {reason} is required and missing; nothing was written");
            // Install hints are per bare tool (installer/tools.tsv is keyed
            // by a single tool id, not a group), so a group requirement's
            // hint is every member's hint concatenated -- same reasoning as
            // ui::model::PickerState::dep_hint.
            let unmet: Vec<(String, Vec<String>)> = status
                .requirements
                .iter()
                .filter(|(r, met)| !met && r.strength == requirements::Strength::Hard)
                .map(|(r, _)| {
                    let label = requirements::requirement_label(r);
                    let members: Vec<&str> = if r.group.is_some() {
                        r.tool.split(", ").collect()
                    } else {
                        vec![r.tool.as_str()]
                    };
                    let hint = members.iter().flat_map(|m| tools::install_hint(m)).collect();
                    (label, hint)
                })
                .collect();
            summary.hard_blocked.push(HardBlocked {
                skill: skill.clone(),
                unmet,
            });
            continue;
        }
        install::install_skill(
            source,
            skill,
            target,
            integration_selection.choice_for(skill),
            dev_build,
        )
        .map_err(|e| e.to_string())?;
        let line = format!("installed {skill} -> {}", target.join(skill).display());
        println!("{line}");
        summary.installed.push(line);
        installed.push(skill.clone());
    }
    Ok(installed)
}

/// Registers (or removes) each installed skill's mcp adapter with the
/// target agent's own CLI/config -- ported from
/// installer/src/72-mcp-registration.sh's `mcp_registration_step`, run once
/// after the whole install loop, for every installed skill that declares any
/// integration mode at all (almost none do). Silently does nothing when
/// `kind` is not one `mcp.rs` knows how to register against, same as
/// `run_post_install_steps`.
fn run_mcp_registration_step(kind: Option<&str>, source: &Path, target: &Path, skills: &[String]) {
    let Some(kind) = kind else { return };
    if !matches!(kind, "claude" | "opencode" | "codex") {
        return;
    }
    let Some(home) = home_dir_opt() else { return };
    let mut announced = false;
    for skill in skills {
        if integration::modes(source, skill).is_empty() {
            continue;
        }
        if !announced {
            println!();
            println!("== MCP registration ==");
            announced = true;
        }
        let dir = target.join(skill);
        match integration::mcp_adapter_path(source, skill, &dir) {
            Some(path) => {
                let path_str = path.to_string_lossy().to_string();
                match mcp::register_for_kind(kind, skill, &path_str, &home) {
                    Ok(mcp::RegisterOutcome::Registered) => {
                        println!("  {kind}: registered MCP server {skill}")
                    }
                    Ok(mcp::RegisterOutcome::Manual) => {
                        for line in mcp::manual_instructions(kind, skill, &path_str) {
                            println!("  {line}");
                        }
                    }
                    Err(e) => println!("  {kind}: {e}"),
                }
            }
            None => match mcp::unregister_for_kind(kind, skill, &dir, &home) {
                Ok(true) => println!("  {kind}: removed MCP server {skill}"),
                Ok(false) => {}
                Err(e) => println!("  {kind}: {e}"),
            },
        }
    }
}

/// The permission grants, plan migration and vendor plugins install.sh
/// bundles with a skill's own install step (sections 12-13 of
/// 70-permissions.sh/65-plan-migration.sh), run here for the same skills
/// just installed. Silently does nothing beyond the install itself when
/// `kind` is unknown (a custom --target, or an agent this installer has no
/// auto-editable grant for) -- the standalone grant-permissions/mcp-register/
/// migrate-plans/install-*-plugin subcommands remain the manual fallback,
/// same role install.sh's print_manual_permissions plays for a "custom" row.
fn run_post_install_steps(
    kind: Option<&str>,
    source: &Path,
    target: &Path,
    skills: &[String],
    confirms: &mut Confirms,
) {
    let Some(kind) = kind else { return };
    if !matches!(kind, "claude" | "opencode" | "codex") {
        return;
    }
    let Some(home) = home_dir_opt() else { return };

    // install.sh's worktrees_permission_step runs for every install, whatever
    // skills were selected -- unlike everything else below, not gated on any
    // particular skill being among them.
    run_worktrees_permission_step(kind, confirms, &home);

    if skills.iter().any(|s| s == "planning") {
        run_planning_post_install(kind, target, &home, confirms);
    }
    run_mcp_registration_step(Some(kind), source, target, skills);
    if skills.iter().any(|s| s == "interactive-shell") {
        run_interactive_shell_post_install(kind, source, target, &home, confirms);
    }
    if kind == "claude" && skills.iter().any(|s| s == "ai-text-editor") {
        run_editor_steering_step(&home, confirms);
        match plugins::install_editor_gate_plugin(source, target) {
            Ok(destination) => println!(
                "Installed: {} (gates sed -i/perl -i/heredoc writes behind a minted token)",
                destination.display()
            ),
            Err(e) => println!("editor-gate-plugin: {e}"),
        }
    }
}

/// Runs for every install with a known agent kind, whatever skills were
/// selected -- ported from install.sh's `worktrees_permission_step`, which
/// is deliberately outside the `contains planning ...` branch for the same
/// reason (any agent may be asked to take a worktree).
fn run_worktrees_permission_step(kind: &str, confirms: &mut Confirms, home: &Path) {
    println!();
    println!("== Agent worktree permissions ==");
    let worktrees = permissions::default_worktrees_root(home);
    let worktrees_str = worktrees.to_string_lossy().to_string();
    if confirms.ask(&format!("Create {worktrees_str} as the agent worktree root?")) {
        match std::fs::create_dir_all(&worktrees) {
            Ok(()) => println!("  Created {worktrees_str}"),
            Err(e) => println!("  cannot create {worktrees_str}: {e}"),
        }
    }
    if !confirms.ask(&format!(
        "Grant the selected agents read/write/execute on {worktrees_str}, so a worktree there \
         needs no prompt per file? (Each edited config is backed up beside itself, unless git \
         already tracks it)"
    )) {
        return;
    }
    match kind {
        "claude" => match permissions::claude_worktrees_permissions(&worktrees_str, home) {
            Ok(outcome) => {
                print_permission_outcome("claude-code", "worktree permissions already present", outcome)
            }
            Err(e) => println!("claude-code: {e}"),
        },
        "opencode" => match permissions::opencode_worktrees_permissions(&worktrees_str, home) {
            Ok(outcome) => print_opencode_outcome("worktree permissions already present", outcome),
            Err(e) => println!("opencode: {e}"),
        },
        "codex" => match permissions::codex_worktrees_permissions(&worktrees_str, home) {
            Ok(outcome) => print_codex_outcome("writable_roots already present", outcome),
            Err(e) => println!("codex: {e}"),
        },
        _ => {}
    }
}

fn run_planning_post_install(kind: &str, target: &Path, home: &Path, confirms: &mut Confirms) {
    println!("== planning runtime permissions ==");
    let scripts = target.join("planning").join("scripts");
    let plans = plan_migration::default_root(home);
    let tmp = std::env::temp_dir().join("planning-agent");
    let scripts = scripts.to_string_lossy();
    let plans_str = plans.to_string_lossy();
    let tmp_str = tmp.to_string_lossy();
    if confirms.ask(&format!("Create {plans_str} as the global plans directory?")) {
        match std::fs::create_dir_all(&plans) {
            Ok(()) => println!("  Created {plans_str}"),
            Err(e) => println!("  cannot create {plans_str}: {e}"),
        }
    }
    if confirms.ask(&format!(
        "Grant the selected agents read/write on {plans_str} and {tmp_str}, and allow them to \
         execute the planning shell scripts? (Each edited config is backed up beside itself, \
         unless git already tracks it)"
    )) {
        match kind {
            "claude" => {
                match permissions::claude_planning_permissions(&scripts, &plans_str, &tmp_str, home) {
                    Ok(outcome) => {
                        print_permission_outcome("claude-code", "permissions already present", outcome)
                    }
                    Err(e) => println!("claude-code: {e}"),
                }
            }
            "opencode" => {
                match permissions::opencode_planning_permissions(&scripts, &plans_str, &tmp_str, home) {
                    Ok(outcome) => print_opencode_outcome("permissions already present", outcome),
                    Err(e) => println!("opencode: {e}"),
                }
            }
            "codex" => {
                match permissions::codex_planning_permissions(&scripts, &plans_str, &tmp_str, home) {
                    Ok(outcome) => print_codex_outcome("writable_roots already present", outcome),
                    Err(e) => println!("codex: {e}"),
                }
            }
            _ => {}
        }
    }
    match plan_migration::migrate_legacy_plans(&[target.to_path_buf()], home) {
        Ok(outcome) => {
            for plan in &outcome.migrated {
                println!("Migrated plan: -> {}", plan.display());
            }
            for (plan, reason) in &outcome.blocked {
                println!("Plan migration blocked: {}: {reason}", plan.display());
            }
        }
        Err(e) => println!("migrate-plans: {e}"),
    }
}

fn run_interactive_shell_post_install(
    kind: &str,
    source: &Path,
    target: &Path,
    home: &Path,
    confirms: &mut Confirms,
) {
    println!("== interactive-shell execution permission ==");
    if kind == "claude" {
        if confirms.ask(
            "Allow the selected agents to execute the interactive-shell binaries, so driving a \
             terminal program needs no prompt per call? (Each edited config is backed up beside \
             itself, unless git already tracks it)",
        ) {
            let bins = target.join("interactive-shell").join("bin");
            match permissions::claude_interactive_shell_permissions(&bins.to_string_lossy(), home) {
                Ok(outcome) => print_permission_outcome(
                    "claude-code",
                    "interactive-shell grant already in place",
                    outcome,
                ),
                Err(e) => println!("claude-code: {e}"),
            }
        } else {
            println!("  Left unchanged. A refused wrapper call reads as a broken tool, so");
            println!("  expect the skill to be skipped in favour of a headless command.");
        }
        match plugins::install_tui_hint_plugin_claude(source, target) {
            Ok(destination) => println!("Installed: {}", destination.display()),
            Err(e) => println!("tui-hint-plugin: {e}"),
        }
    } else if kind == "opencode" {
        match plugins::install_tui_hint_plugin_opencode(source, home) {
            Ok(plugins::OpencodePluginOutcome::Registered) => {
                println!("opencode: added the tui-hint-plugin to the plugin array")
            }
            Ok(plugins::OpencodePluginOutcome::AlreadyRegistered) => {
                println!("opencode: tui-hint-plugin already registered")
            }
            Ok(plugins::OpencodePluginOutcome::NotShipped) => {}
            Ok(plugins::OpencodePluginOutcome::NotStrictJson) => {
                println!("opencode: config is not strict JSON; register tui-hint-plugin by hand")
            }
            Err(e) => println!("tui-hint-plugin: {e}"),
        }
    }
}

/// Offered only when a Claude Code root was selected (these are Claude
/// Code's own settings) and the run placed ai-text-editor -- ported from
/// install.sh's `editor_steering_step`. Two independent off-switches, in
/// the same order bash offers them; declining both leaves the setting
/// unchanged, same as bash's own final message.
fn run_editor_steering_step(home: &Path, confirms: &mut Confirms) {
    println!();
    println!("== ai-text-editor tool steering ==");
    println!(
        "  Claude Code may instruct the agent to make file changes with sed, heredocs or short \
         scripts instead of an editor. While that instruction is active the ai-text-editor MCP \
         is usually skipped, and these are what it costs:"
    );
    println!(
        "    - an in-place sed rewrites the file and exits 0 whether or not the pattern \
         matched, so a mistype is indistinguishable from success"
    );
    println!(
        "    - a script heredoc stacks the shell's escaping on top of the language's on top of \
         the target file's syntax"
    );
    println!(
        "    - neither verifies what it replaces, while the editor's expected_text refuses on \
         mismatch and its journal survives a git checkout"
    );
    println!("  Two settings turn it down, and either is enough:");
    println!("    CLAUDE_CODE_THRIFTY_SONIC=false  the instruction is not injected at all");
    println!(
        "    CLAUDE_CODE_COZY_TEAPOT=relaxed  softer wording that leaves the choice to the \
         agent, so the editor still competes"
    );
    if confirms.ask("Turn the instruction off (env CLAUDE_CODE_THRIFTY_SONIC=false)?") {
        apply_claude_env_setting("CLAUDE_CODE_THRIFTY_SONIC", "false", home);
        return;
    }
    if confirms.ask("Soften it instead (env CLAUDE_CODE_COZY_TEAPOT=relaxed)?") {
        apply_claude_env_setting("CLAUDE_CODE_COZY_TEAPOT", "relaxed", home);
        return;
    }
    println!("  Left unchanged. Expect the editor to be bypassed for sed and heredocs.");
}

fn apply_claude_env_setting(key: &str, value: &str, home: &Path) {
    match permissions::claude_env_setting(key, value, home) {
        Ok(permissions::EnvSettingOutcome::NoConfigFile) => {
            println!("  claude-code: no settings.json found; set env.{key} to \"{value}\" by hand")
        }
        Ok(permissions::EnvSettingOutcome::AlreadySet) => {
            println!("  claude-code: env.{key} is already \"{value}\"")
        }
        Ok(permissions::EnvSettingOutcome::Set) => {
            println!("  claude-code: set env.{key} to \"{value}\"")
        }
        Err(e) => println!("  claude-code: {e}"),
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
    let roots = resolve_targets_and_kinds(args.targets, args.agents)?;

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

    let integration_selection = build_integration_selection(&source, &args.integration)?;
    // One shared Confirms across every root, matching install.sh's single
    // run-wide YES/YES_ALL: an "a" (all) answer for the first root's prompt
    // must still auto-answer every later root's prompts too.
    let mut confirms = Confirms::new(args.yes);
    let mut summary = Summary::default();
    for (target, kind) in &roots {
        let installed = install_selected_skills(
            &source,
            target,
            &skills,
            &integration_selection,
            args.dev_build,
            &mut summary,
        )?;
        run_post_install_steps(kind.as_deref(), &source, target, &installed, &mut confirms);
    }
    let root_paths: Vec<PathBuf> = roots.iter().map(|(t, _)| t.clone()).collect();
    summary.print(&root_paths, args.yes);
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
    Bins {
        bins: String,
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
    let mut bins: Option<String> = None;

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
            "--bins" => bins = Some(value!()),
            other => return Err(format!("grant-permissions: unknown option: {other}")),
        }
        i += 1;
    }
    let agent = agent.ok_or("grant-permissions: --agent is required")?;
    let target = match (scripts, plans, tmp, worktrees, bins) {
        (Some(scripts), Some(plans), Some(tmp), None, None) => GrantTarget::Planning {
            scripts,
            plans,
            tmp,
        },
        (None, None, None, Some(worktrees), None) => GrantTarget::Worktrees { worktrees },
        (None, None, None, None, Some(bins)) => GrantTarget::Bins { bins },
        _ => return Err(
            "grant-permissions: pass exactly one of --scripts/--plans/--tmp together, --worktrees, or --bins"
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
        ("claude", GrantTarget::Bins { bins }) => {
            let outcome = permissions::claude_interactive_shell_permissions(bins, &home)
                .map_err(|e| e.to_string())?;
            print_permission_outcome(
                "claude-code",
                "interactive-shell grant already in place",
                outcome,
            );
        }
        (_, GrantTarget::Bins { .. }) => {
            return Err(
                "grant-permissions: --bins is only auto-editable for --agent claude".to_string(),
            )
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

struct PluginArgs {
    agent: Option<String>,
    source: PathBuf,
    target: Option<PathBuf>,
}

fn parse_plugin_args(command: &str, argv: &[String]) -> Result<PluginArgs, String> {
    let mut agent: Option<String> = None;
    let mut source: Option<PathBuf> = None;
    let mut target: Option<PathBuf> = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--agent" => {
                i += 1;
                agent = Some(argv.get(i).ok_or("--agent needs a value")?.clone());
            }
            "--source" => {
                i += 1;
                source = Some(PathBuf::from(argv.get(i).ok_or("--source needs a value")?));
            }
            "--target" => {
                i += 1;
                target = Some(PathBuf::from(argv.get(i).ok_or("--target needs a value")?));
            }
            other => return Err(format!("{command}: unknown option: {other}")),
        }
        i += 1;
    }
    Ok(PluginArgs {
        agent,
        source: source.ok_or(format!("{command}: --source is required"))?,
        target,
    })
}

fn run_install_tui_hint_plugin(argv: &[String]) -> Result<ExitCode, String> {
    let args = parse_plugin_args("install-tui-hint-plugin", argv)?;
    let agent = args
        .agent
        .ok_or("install-tui-hint-plugin: --agent is required")?;
    match agent.as_str() {
        "claude" => {
            let target = args
                .target
                .ok_or("install-tui-hint-plugin: --target is required for --agent claude")?;
            let destination = plugins::install_tui_hint_plugin_claude(&args.source, &target)
                .map_err(|e| e.to_string())?;
            println!(
                "Installed: {} (reminds an agent of a shipped app profile before it runs a Bash command headlessly)",
                destination.display()
            );
        }
        "opencode" => {
            let home = std::env::var("HOME")
                .map(PathBuf::from)
                .map_err(|_| "install-tui-hint-plugin: needs $HOME set".to_string())?;
            match plugins::install_tui_hint_plugin_opencode(&args.source, &home)
                .map_err(|e| e.to_string())?
            {
                plugins::OpencodePluginOutcome::NotShipped => {
                    println!("opencode: this checkout does not ship the opencode tui-hint-plugin variant");
                }
                plugins::OpencodePluginOutcome::NotStrictJson => {
                    println!("opencode: config is not strict JSON; register the plugin by hand");
                }
                plugins::OpencodePluginOutcome::AlreadyRegistered => {
                    println!("opencode: plugin already registered");
                }
                plugins::OpencodePluginOutcome::Registered => {
                    println!("opencode: added the plugin to the plugin array");
                }
            }
        }
        other => {
            return Err(format!(
            "install-tui-hint-plugin: unknown --agent {other}; known agents are: claude, opencode"
        ))
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run_install_editor_gate_plugin(argv: &[String]) -> Result<ExitCode, String> {
    let args = parse_plugin_args("install-editor-gate-plugin", argv)?;
    let target = args
        .target
        .ok_or("install-editor-gate-plugin: --target is required")?;
    let destination =
        plugins::install_editor_gate_plugin(&args.source, &target).map_err(|e| e.to_string())?;
    println!(
        "Installed: {} (gates sed -i/perl -i/heredoc writes behind a minted token; see editor-gate-plugin/README.md)",
        destination.display()
    );
    Ok(ExitCode::SUCCESS)
}

fn run_set_claude_env(argv: &[String]) -> Result<ExitCode, String> {
    let mut key: Option<String> = None;
    let mut value: Option<String> = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--key" => {
                i += 1;
                key = Some(argv.get(i).ok_or("--key needs a value")?.clone());
            }
            "--value" => {
                i += 1;
                value = Some(argv.get(i).ok_or("--value needs a value")?.clone());
            }
            other => return Err(format!("set-claude-env: unknown option: {other}")),
        }
        i += 1;
    }
    let key = key.ok_or("set-claude-env: --key is required")?;
    let value = value.ok_or("set-claude-env: --value is required")?;
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "set-claude-env: needs $HOME set".to_string())?;

    match permissions::claude_env_setting(&key, &value, &home).map_err(|e| e.to_string())? {
        permissions::EnvSettingOutcome::NoConfigFile => {
            println!("claude-code: no settings.json found; skipped");
        }
        permissions::EnvSettingOutcome::AlreadySet => {
            println!("claude-code: env.{key} is already \"{value}\"");
        }
        permissions::EnvSettingOutcome::Set => {
            println!("claude-code: set env.{key} to \"{value}\"");
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run_interactive(argv: &[String]) -> Result<ExitCode, String> {
    let mut source: Option<PathBuf> = None;
    let mut target: Option<PathBuf> = None;
    let mut agent: Option<String> = None;
    let mut integration_args = Vec::new();
    let mut yes = false;
    let mut dev_build = false;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
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
            "--integration" => {
                i += 1;
                integration_args.push(argv.get(i).ok_or("--integration needs a mode, or skill=mode")?.clone());
            }
            "--editor-integration" => {
                i += 1;
                let mode = argv.get(i).ok_or("--editor-integration needs skill or mcp")?;
                integration_args.push(format!("ai-text-editor={mode}"));
            }
            "--yes" => yes = true,
            "--dev-build" => dev_build = true,
            other => return Err(format!("interactive: unknown option: {other}")),
        }
        i += 1;
    }
    let source = resolve_source(source)?;
    let (target, kind) = resolve_target_and_kind(target, agent)?;
    let integration_selection = build_integration_selection(&source, &integration_args)?;

    let names = discover::discover_skills(&source).map_err(|e| e.to_string())?;
    let skills = names
        .into_iter()
        .map(|name| {
            let description = manifest::known_skill(&name)
                .map(|s| s.description.to_string())
                .unwrap_or_default();
            let destination = target.join(&name);
            let installed = destination.join("SKILL.md").is_file();
            let status = requirements::skill_status(&source, &name);
            let offered_modes = integration::modes(&source, &name);
            let mode = integration::resolve_mode(
                &source,
                &name,
                Some(&destination),
                integration_selection.choice_for(&name),
            );
            ui::model::SkillEntry {
                name,
                description,
                installed,
                status,
                offered_modes,
                mode,
            }
        })
        .collect();

    match ui::run_picker(skills, &source) {
        None => {
            println!("interactive: no changes made");
            Ok(ExitCode::SUCCESS)
        }
        Some(selected) if selected.is_empty() => {
            println!("interactive: nothing selected; no changes made");
            Ok(ExitCode::SUCCESS)
        }
        Some(selected) => {
            let names: Vec<String> = selected.iter().map(|(name, _)| name.clone()).collect();
            let mut picked = IntegrationSelection::default();
            for (name, mode) in &selected {
                picked.per_skill.insert(name.clone(), mode.clone());
            }
            let mut summary = Summary::default();
            let installed = install_selected_skills(
                &source,
                &target,
                &names,
                &picked,
                dev_build,
                &mut summary,
            )?;
            let mut confirms = Confirms::new(yes);
            run_post_install_steps(kind.as_deref(), &source, &target, &installed, &mut confirms);
            summary.print(std::slice::from_ref(&target), yes);
            Ok(ExitCode::SUCCESS)
        }
    }
}
