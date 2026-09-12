// MODE: DEV
// PACKAGE: PROD
//! The shipped skill list and the agent targets install.sh's menu offers,
//! ported from installer/src/05-config.sh's `SKILL_NAMES`/`SKILL_DESCRIPTIONS`
//! and `TARGET_NAMES`/`TARGET_PATHS`/`TARGET_KINDS`. Hand-kept in sync with
//! that file (index-parallel there, a struct here) rather than generated,
//! same as install.sh itself describes those arrays: a new skill or agent is
//! one edit, in both places, until this replaces install.sh outright.
//!
//! `SKILL_DETAILS` (the interactive picker's long-form body) is not ported
//! yet -- it belongs to the TUI slice, not the manifest.

pub struct Skill {
    pub name: &'static str,
    pub description: &'static str,
}

pub const SKILLS: &[Skill] = &[
    Skill { name: "planning", description: "Durable, resumable plans with steps and verification." },
    Skill { name: "project-specifics", description: "Records project conventions, quirks, and deviations." },
    Skill { name: "resource-limited-testing", description: "Caps CPU and memory for demanding tool runs." },
    Skill { name: "brainstorm", description: "Shapes an idea into a recorded, agreed picture before planning." },
    Skill { name: "post-implementation-review", description: "After-the-fact review and proposed fixes for built code." },
    Skill { name: "todo", description: "A nested queue of work in one JSON file, read with rjq." },
    Skill { name: "bug-report", description: "Defects with their reproduction, mechanism and verification, in JSON." },
    Skill { name: "chat", description: "IRC-basis agent chat over TLS: a rust server and client, UDP discovery, deltas." },
    Skill { name: "git-worktrees", description: "Separate checkouts so parallel work and long verifications cannot collide." },
    Skill { name: "git-merge-resolving", description: "Conflicts resolved by what each side changed, and a merged tree you can trust." },
    Skill { name: "merge-request-etiquette", description: "Merge requests in your voice: own branch, one squashed commit, a TLDR, then the fix." },
    Skill { name: "text-etiquette", description: "Shorthand and a clipped register for an agent prose: chat, dev talk, and its own thinking. Short, factual, no people-please prose; plain english on request." },
    Skill { name: "ai-text-editor", description: "Server-owned editor tabs for agents: bounded reads, explicit search modes, revision-aware edits, undo/redo, raw-byte and hex access, SQLite metadata, and Unix-socket or TCP transport." },
    Skill { name: "interactive-shell", description: "Drives unknown full-screen terminal programs through a PTY wrapper and a unix-socket input client." },
    Skill { name: "www", description: "A brake the human can pull, and one the agent pulls on itself when it is thrashing: stop, answer three questions, then one reasoned step." },
    Skill { name: "ci-failures", description: "What actually failed in a CI run or pipeline, from a run/pipeline id, a PR/MR number or a branch -- on GitHub or GitLab." },
];

pub fn known_skill(name: &str) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| s.name == name)
}

/// The reason `skill` cannot be installed on the running host, or `None`
/// when this platform supports it -- ported from
/// installer/src/50-manifest.sh's `skill_unsupported_here`. Only
/// interactive-shell is gated today: its PTY wrapper is POSIX-only (no
/// Windows build exists at all, see interactive-shell/binaries.tsv), so a
/// Windows install would otherwise try to copy a binary that was never
/// shipped for this platform instead of naming the real reason up front.
/// Checked against `cfg!(windows)` rather than bash's own MINGW*/MSYS*/
/// CYGWIN*/Windows* `uname -s` match: those four are how Windows looks to a
/// bash script running under different POSIX layers, but this is a native
/// Rust binary asking about its own compiled target, which `cfg!(windows)`
/// already answers directly.
pub fn skill_unsupported_here(skill: &str) -> Option<&'static str> {
    if skill == "interactive-shell" && cfg!(windows) {
        return Some("no Windows build exists; the PTY wrapper is POSIX-only");
    }
    None
}

pub struct Agent {
    pub name: &'static str,
    pub kind: &'static str,
    /// Joined onto $HOME with `/`, matching install.sh's TARGET_PATHS.
    pub home_suffix: &'static str,
}

pub const AGENTS: &[Agent] = &[
    Agent {
        name: "Universal Agent Skills",
        kind: "universal",
        home_suffix: ".agents/skills",
    },
    Agent {
        name: "Codex",
        kind: "codex",
        home_suffix: ".codex/skills",
    },
    Agent {
        name: "Claude Code",
        kind: "claude",
        home_suffix: ".claude/skills",
    },
    Agent {
        name: "OpenCode",
        kind: "opencode",
        home_suffix: ".config/opencode/skills",
    },
    Agent {
        name: "OpenClaw",
        kind: "openclaw",
        home_suffix: ".openclaw/skills",
    },
    Agent {
        name: "Cline",
        kind: "cline",
        home_suffix: ".cline/skills",
    },
];

pub fn known_agent(kind: &str) -> Option<&'static Agent> {
    AGENTS.iter().find(|a| a.kind == kind)
}

fn on_path(bin: &str) -> bool {
    let Ok(path_var) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(bin).is_file())
}

/// Any entry directly under `dir` whose filename starts with `prefix` --
/// Rust's answer to install.sh's `compgen -G "$dir/$prefix*"` glob probe
/// for Cline's own versioned VS Code extension directory name.
fn any_entry_starts_with(dir: &std::path::Path, prefix: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries
        .filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().starts_with(prefix))
}

/// Is this agent worth offering as an install root on this host -- ported
/// from install.sh's `agent_target_available`, keyed by `kind` instead of
/// bash's array index (this installer has no positional TARGET_PATHS array
/// to index into). Universal Agent Skills has no owning application, so it
/// is always offered; every other kind needs either its own CLI on PATH or
/// evidence it is already installed. Cline's own check is the widest: no
/// CLI at all, just its skills directory, its VS Code extension directory
/// (a fixed name or a versioned `saoudrizwan.claude-dev-<version>` one,
/// local or on a remote/server VS Code install), or its global storage
/// directory.
pub fn agent_available(kind: &str, home: &std::path::Path) -> bool {
    match kind {
        "universal" => true,
        "codex" => on_path("codex") || home.join(".codex").is_dir(),
        "claude" => on_path("claude") || home.join(".claude").is_dir(),
        "opencode" => on_path("opencode") || home.join(".config/opencode").is_dir(),
        "openclaw" => on_path("openclaw") || home.join(".openclaw").is_dir(),
        "cline" => {
            let config_home = std::env::var("XDG_CONFIG_HOME")
                .ok()
                .filter(|s| !s.is_empty())
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| home.join(".config"));
            home.join(".cline/skills").is_dir()
                || home
                    .join(".vscode/extensions/saoudrizwan.claude-dev")
                    .is_dir()
                || any_entry_starts_with(
                    &home.join(".vscode/extensions"),
                    "saoudrizwan.claude-dev-",
                )
                || any_entry_starts_with(
                    &home.join(".vscode-server/extensions"),
                    "saoudrizwan.claude-dev-",
                )
                || config_home
                    .join("Code/User/globalStorage/saoudrizwan.claude-dev")
                    .is_dir()
                || any_entry_starts_with(
                    &config_home.join("Code/User/globalStorage"),
                    "saoudrizwan.claude-dev",
                )
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_shell_is_unsupported_only_on_windows() {
        let reason = skill_unsupported_here("interactive-shell");
        if cfg!(windows) {
            assert!(reason.is_some());
        } else {
            assert!(reason.is_none());
        }
    }

    #[test]
    fn every_other_skill_is_always_supported() {
        assert!(skill_unsupported_here("planning").is_none());
        assert!(skill_unsupported_here("todo").is_none());
    }

    #[test]
    fn universal_agent_skills_is_always_available() {
        let home = tempfile::tempdir().unwrap();
        assert!(agent_available("universal", home.path()));
    }

    #[test]
    fn an_unknown_kind_is_never_available() {
        let home = tempfile::tempdir().unwrap();
        assert!(!agent_available("not-a-real-agent", home.path()));
    }

    #[test]
    fn claude_is_available_when_its_directory_already_exists() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".claude")).unwrap();
        assert!(agent_available("claude", home.path()));
    }

    #[test]
    fn cline_is_available_from_its_versioned_vscode_extension_directory() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(
            home.path()
                .join(".vscode/extensions/saoudrizwan.claude-dev-3.1.4"),
        )
        .unwrap();
        assert!(agent_available("cline", home.path()));
    }

    #[test]
    fn every_skill_name_is_unique() {
        let mut names: Vec<_> = SKILLS.iter().map(|s| s.name).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate skill name in SKILLS");
    }

    #[test]
    fn every_agent_kind_is_unique() {
        let mut kinds: Vec<_> = AGENTS.iter().map(|a| a.kind).collect();
        let before = kinds.len();
        kinds.sort_unstable();
        kinds.dedup();
        assert_eq!(kinds.len(), before, "duplicate agent kind in AGENTS");
    }

    #[test]
    fn known_skill_finds_an_existing_name_and_rejects_an_unknown_one() {
        assert!(known_skill("todo").is_some());
        assert!(known_skill("not-a-real-skill").is_none());
    }

    #[test]
    fn known_agent_finds_an_existing_kind_and_rejects_an_unknown_one() {
        assert_eq!(known_agent("claude").unwrap().home_suffix, ".claude/skills");
        assert!(known_agent("not-a-real-agent").is_none());
    }
}
