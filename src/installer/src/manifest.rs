// MODE: DEV
// PACKAGE: PROD
//! The shipped skill list and the agent targets this installer's menu
//! offers. Hand-kept as a struct list rather than generated: a new skill or
//! agent is one edit here.
//!
//! The interactive picker's long-form body is not carried here -- it
//! belongs to the TUI slice, not the manifest.

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
    Skill { name: "question-etiquette", description: "Numbered questions, lettered options, never a bullet: so a reply like Q7b is unambiguous and a partial answer names exactly which numbers are still open." },
    Skill { name: "ai-text-editor", description: "Server-owned editor tabs for agents: bounded reads, explicit search modes, revision-aware edits, undo/redo, raw-byte and hex access, SQLite metadata, and Unix-socket or TCP transport." },
    Skill { name: "interactive-shell", description: "Drives unknown full-screen terminal programs through a PTY wrapper and a unix-socket input client." },
    Skill { name: "www", description: "A brake the human can pull, and one the agent pulls on itself when it is thrashing: stop, answer three questions, then one reasoned step." },
    Skill { name: "ci-failures", description: "What actually failed in a CI run or pipeline, from a run/pipeline id, a PR/MR number or a branch -- on GitHub or GitLab." },
];

pub fn known_skill(name: &str) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| s.name == name)
}

/// A profile shipped from `.agents/profiles/<name>.json` (T102): one agent
/// persona, translated per harness kind by `profiles::ProfileTranslator`
/// rather than copied verbatim -- Claude Code, opencode and codex each read a
/// different native format for a custom subagent.
pub struct Profile {
    pub name: &'static str,
    /// Repo-root-relative path to the canonical JSON source.
    pub source: &'static str,
}

pub const PROFILES: &[Profile] = &[
    Profile {
        name: "nitpicker",
        source: ".agents/profiles/nitpicker.json",
    },
    Profile {
        name: "benny",
        source: ".agents/profiles/benny.json",
    },
    Profile {
        name: "chris",
        source: ".agents/profiles/chris.json",
    },
    Profile {
        name: "christian",
        source: ".agents/profiles/christian.json",
    },
    Profile {
        name: "christoph",
        source: ".agents/profiles/christoph.json",
    },
    Profile {
        name: "dana",
        source: ".agents/profiles/dana.json",
    },
    Profile {
        name: "frank",
        source: ".agents/profiles/frank.json",
    },
    Profile {
        name: "maintainer",
        source: ".agents/profiles/maintainer.json",
    },
    Profile {
        name: "installer",
        source: ".agents/profiles/installer.json",
    },
    Profile {
        name: "oracle",
        source: ".agents/profiles/oracle.json",
    },
    Profile {
        name: "eve",
        source: ".agents/profiles/eve.json",
    },
];

/// The reason `skill` cannot be installed on the running host, or `None`
/// when this platform supports it. interactive-shell was the only skill
/// ever gated here (its PTY wrapper had no Windows build); T84 shipped one
/// (ConPTY + a loopback TCP transport), so every shipped skill now has a
/// build for every platform this binary itself can run on. Kept, rather
/// than removed, as the one place a future platform-specific gap would be
/// named up front instead of failing as a missing file partway through
/// copying a skill's other files.
pub fn skill_unsupported_here(_skill: &str) -> Option<&'static str> {
    None
}

pub struct Agent {
    pub name: &'static str,
    pub kind: &'static str,
    /// Joined onto $HOME with `/` to form this agent's skills directory.
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

/// Any entry directly under `dir` whose filename starts with `prefix` -- a
/// glob probe for Cline's own versioned VS Code extension directory name.
fn any_entry_starts_with(dir: &std::path::Path, prefix: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries
        .filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().starts_with(prefix))
}

/// Is this agent worth offering as an install root on this host. Universal
/// Agent Skills has no owning application, so it is always offered; every
/// other kind needs either its own CLI on PATH or evidence it is already
/// installed. Cline's own check is the widest: no CLI at all, just its
/// skills directory, its VS Code extension directory (a fixed name or a
/// versioned `saoudrizwan.claude-dev-<version>` one, local or on a
/// remote/server VS Code install), or its global storage directory.
pub fn agent_available(kind: &str, home: &std::path::Path) -> bool {
    match kind {
        "universal" => true,
        "codex" => on_path("codex") || home.join(".codex").is_dir(),
        "claude" => on_path("claude") || home.join(".claude").is_dir(),
        "opencode" => on_path("opencode") || home.join(".config/opencode").is_dir(),
        "openclaw" => on_path("openclaw") || home.join(".openclaw").is_dir(),
        "cline" => {
            let config_home = crate::shared_bin::xdg_config_home_or(home);
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
    fn interactive_shell_is_supported_everywhere_now() {
        assert!(skill_unsupported_here("interactive-shell").is_none());
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

    /// A copy-paste typo in one of the near-identical `Profile{}` literals
    /// must fail `cargo test`, not silently no-op at real install time.
    #[test]
    fn every_profile_entry_resolves_to_a_real_parseable_matching_file() {
        for profile in PROFILES {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(profile.source);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let spec = crate::profiles::ProfileSpec::from_json(&text)
                .unwrap_or_else(|error| panic!("{}: {error}", profile.source));
            assert_eq!(spec.name, profile.name, "{}", profile.source);
        }
    }
}
