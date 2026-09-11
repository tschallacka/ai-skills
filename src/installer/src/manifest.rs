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
    Skill { name: "project-specificies", description: "Records project conventions, quirks, and deviations." },
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
