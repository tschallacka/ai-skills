// MODE: DEV
// PACKAGE: PROD
//! Installs agent profiles (T102): a canonical JSON persona
//! (`.agents/profiles/<name>.json`) translated per harness kind into that
//! harness's own native custom-subagent format. Claude Code and opencode both
//! read Markdown + YAML frontmatter, but different schemas; codex reads TOML
//! entirely. `ProfileTranslator` is the whole extensibility point: a harness
//! not yet supported is one new translator registered in `TRANSLATORS`, not a
//! change to any existing one.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The canonical, harness-neutral shape of one profile. Exactly the fields
/// `.agents/profiles/nitpicker.json` has today -- a field is not added here
/// speculatively for a harness capability (`tools`, `model`, `mode`, ...) the
/// one real profile has never used.
#[derive(Debug, Clone, Deserialize)]
pub struct ProfileSpec {
    pub name: String,
    pub description: String,
    pub instructions: String,
}

impl ProfileSpec {
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|error| format!("invalid profile JSON: {error}"))
    }
}

/// One harness's translation from [`ProfileSpec`] to its own native format.
/// Adding a harness is implementing this trait once and adding one entry to
/// [`TRANSLATORS`] -- no existing translator changes.
pub trait ProfileTranslator {
    /// The agent kind this targets, matching `manifest::Agent::kind`.
    fn kind(&self) -> &'static str;
    /// Destination directory, joined onto `$HOME` -- the same way
    /// `manifest::Agent::home_suffix` is for skills.
    fn dest_suffix(&self) -> &'static str;
    /// The filename (with extension) this translator writes.
    fn filename(&self, spec: &ProfileSpec) -> String;
    /// Render the destination file's full content.
    fn render(&self, spec: &ProfileSpec) -> Result<String, String>;
}

/// Claude Code's own subagent frontmatter: `name` and `description`, exactly
/// what `.claude/agents/*.md` already reads.
#[derive(Serialize)]
struct ClaudeFrontmatter<'a> {
    name: &'a str,
    description: &'a str,
}

pub struct ClaudeTranslator;

impl ProfileTranslator for ClaudeTranslator {
    fn kind(&self) -> &'static str {
        "claude"
    }
    fn dest_suffix(&self) -> &'static str {
        ".claude/agents"
    }
    fn filename(&self, spec: &ProfileSpec) -> String {
        format!("{}.md", spec.name)
    }
    fn render(&self, spec: &ProfileSpec) -> Result<String, String> {
        render_frontmatter_markdown(
            &ClaudeFrontmatter {
                name: &spec.name,
                description: &spec.description,
            },
            &spec.instructions,
        )
    }
}

/// opencode's own subagent frontmatter: `description` only -- opencode takes
/// the agent's name from the filename, so `name` has no field here at all
/// (not merely omitted when rendering; the struct cannot carry it).
#[derive(Serialize)]
struct OpencodeFrontmatter<'a> {
    description: &'a str,
}

pub struct OpencodeTranslator;

impl ProfileTranslator for OpencodeTranslator {
    fn kind(&self) -> &'static str {
        "opencode"
    }
    fn dest_suffix(&self) -> &'static str {
        ".config/opencode/agents"
    }
    fn filename(&self, spec: &ProfileSpec) -> String {
        format!("{}.md", spec.name)
    }
    fn render(&self, spec: &ProfileSpec) -> Result<String, String> {
        render_frontmatter_markdown(
            &OpencodeFrontmatter {
                description: &spec.description,
            },
            &spec.instructions,
        )
    }
}

fn render_frontmatter_markdown(
    frontmatter: &impl Serialize,
    instructions: &str,
) -> Result<String, String> {
    let yaml = serde_yml::to_string(frontmatter)
        .map_err(|error| format!("cannot render frontmatter: {error}"))?;
    Ok(format!("---\n{yaml}---\n\n{instructions}\n"))
}

/// codex's own subagent shape: TOML, not Markdown at all.
#[derive(Serialize)]
struct CodexProfile<'a> {
    name: &'a str,
    description: &'a str,
    developer_instructions: &'a str,
}

pub struct CodexTranslator;

impl ProfileTranslator for CodexTranslator {
    fn kind(&self) -> &'static str {
        "codex"
    }
    fn dest_suffix(&self) -> &'static str {
        ".codex/agents"
    }
    fn filename(&self, spec: &ProfileSpec) -> String {
        format!("{}.toml", spec.name)
    }
    fn render(&self, spec: &ProfileSpec) -> Result<String, String> {
        let profile = CodexProfile {
            name: &spec.name,
            description: &spec.description,
            developer_instructions: &spec.instructions,
        };
        toml::to_string_pretty(&profile)
            .map_err(|error| format!("cannot render codex profile: {error}"))
    }
}

/// Every registered translator. A harness not listed here (universal,
/// openclaw, cline) has no researched agent-profile convention, so
/// `translator_for` answers `None` for it and the install step skips that
/// kind silently -- the same shape T90's session-dependent skill gating
/// already established.
pub const TRANSLATORS: &[&dyn ProfileTranslator] =
    &[&ClaudeTranslator, &OpencodeTranslator, &CodexTranslator];

pub fn translator_for(kind: &str) -> Option<&'static dyn ProfileTranslator> {
    TRANSLATORS.iter().copied().find(|t| t.kind() == kind)
}

/// Renders `spec` with `translator` and writes it under `home`, creating the
/// destination directory if it does not exist. Returns the path written.
pub fn install_profile(
    spec: &ProfileSpec,
    translator: &dyn ProfileTranslator,
    home: &Path,
) -> io::Result<PathBuf> {
    let content = translator.render(spec).map_err(io::Error::other)?;
    let dest_dir = home.join(translator.dest_suffix());
    fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(translator.filename(spec));
    fs::write(&dest, content)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nitpicker_spec() -> ProfileSpec {
        ProfileSpec {
            name: "nitpicker".to_string(),
            description: "Guards the repo's rules.".to_string(),
            instructions: "You are the nitpicker.".to_string(),
        }
    }

    #[test]
    fn the_real_profile_json_parses() {
        let text = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.agents/profiles/nitpicker.json"),
        )
        .unwrap();
        let spec = ProfileSpec::from_json(&text).unwrap();
        assert_eq!(spec.name, "nitpicker");
        assert!(!spec.description.is_empty());
        assert!(spec.instructions.starts_with("You are the nitpicker."));
    }

    #[test]
    fn a_missing_field_is_refused_by_name() {
        let error = ProfileSpec::from_json(r#"{"name":"x","description":"y"}"#).unwrap_err();
        assert!(error.contains("invalid profile JSON"), "{error}");
    }

    #[test]
    fn claude_frontmatter_round_trips_name_and_description_and_keeps_the_body_verbatim() {
        let spec = nitpicker_spec();
        let rendered = ClaudeTranslator.render(&spec).unwrap();
        assert!(rendered.starts_with("---\n"));
        assert!(rendered.contains("name: nitpicker"));
        assert!(rendered.contains("description: Guards the repo's rules."));
        assert!(rendered.ends_with("You are the nitpicker.\n"));
    }

    #[test]
    fn opencode_frontmatter_never_carries_a_name_key() {
        let spec = nitpicker_spec();
        let rendered = OpencodeTranslator.render(&spec).unwrap();
        assert!(rendered.contains("description: Guards the repo's rules."));
        assert!(
            !rendered.contains("name:"),
            "opencode takes identity from the filename: {rendered}"
        );
        assert!(rendered.ends_with("You are the nitpicker.\n"));
    }

    #[test]
    fn codex_output_is_valid_toml_with_developer_instructions_verbatim() {
        let spec = nitpicker_spec();
        let rendered = CodexTranslator.render(&spec).unwrap();
        let parsed: toml::Value = toml::from_str(&rendered).unwrap();
        assert_eq!(parsed["name"].as_str(), Some("nitpicker"));
        assert_eq!(
            parsed["developer_instructions"].as_str(),
            Some("You are the nitpicker.")
        );
    }

    #[test]
    fn translator_for_finds_every_registered_kind_and_refuses_an_unknown_one() {
        assert!(translator_for("claude").is_some());
        assert!(translator_for("opencode").is_some());
        assert!(translator_for("codex").is_some());
        assert!(translator_for("universal").is_none());
        assert!(translator_for("openclaw").is_none());
        assert!(translator_for("cline").is_none());
    }

    #[test]
    fn install_profile_writes_to_the_translators_own_destination() {
        let home = tempfile::tempdir().unwrap();
        let spec = nitpicker_spec();
        let dest = install_profile(&spec, &ClaudeTranslator, home.path()).unwrap();
        assert_eq!(
            dest,
            home.path().join(".claude/agents").join("nitpicker.md")
        );
        assert!(dest.is_file());
        let dest = install_profile(&spec, &CodexTranslator, home.path()).unwrap();
        assert_eq!(
            dest,
            home.path().join(".codex/agents").join("nitpicker.toml")
        );
        assert!(dest.is_file());
    }
}
