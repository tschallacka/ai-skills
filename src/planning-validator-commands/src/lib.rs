// MODE: DEV
// PACKAGE: PROD
//! Command-literal validation formerly provided by
//! `validate-plan-commands-lib.sh`.

use planning_validator_common::Findings;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const CORE_WORDS: &[&str] = &[
    "git", "make", "docker", "sh", "bash", "zsh", "env", "sudo", "npx",
];
const BUILTIN_WORDS: &[&str] = &[
    "composer", "npm", "php", "mysql", "curl", "python", "node", "magerun", "phpunit", "phpstan",
];

#[derive(Debug, Default)]
pub struct CommandRegistry {
    path: Option<PathBuf>,
    commands: BTreeSet<String>,
    registered_words: BTreeSet<String>,
    never_executable_extensions: BTreeSet<String>,
    available: bool,
}

impl CommandRegistry {
    pub fn from_file(path: &Path) -> Self {
        Self::from_files(path, Path::new("never-executable-extensions.json"))
    }

    pub fn from_files(path: &Path, extensions_path: &Path) -> Self {
        let available = path.is_file();
        let value = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .unwrap_or(Value::Null);
        let commands: BTreeSet<String> = value
            .as_object()
            .map(|object| {
                object
                    .values()
                    .filter_map(|entry| entry.get("cmd").and_then(Value::as_str))
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let never_executable_extensions = fs::read_to_string(extensions_path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .map(|value| {
                value
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_ascii_lowercase)
                    .collect()
            })
            .unwrap_or_default();
        // report 20 §3: a registered command's first token teaches the
        // detector that tool word, so an unregistered but shape-plausible
        // sibling invocation (a new flag, a different subcommand) is still a
        // candidate worth checking against the registry, matching
        // `command_shaped`'s own `$registered_words` membership test.
        let registered_words = commands
            .iter()
            .filter_map(|command| command.split_whitespace().next())
            .map(str::to_owned)
            .collect();
        Self {
            path: Some(path.to_path_buf()),
            commands,
            registered_words,
            never_executable_extensions,
            available,
        }
    }

    pub fn validate_plan(&self, plan: &Path, complete: bool, findings: &mut Findings) {
        if !self.available {
            return;
        }
        let _ = &self.path;
        let mut files = Vec::new();
        collect_files(plan, &mut files);
        files.retain(|path| {
            path.extension().and_then(|ext| ext.to_str()) == Some("md")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with("-testing.md") || is_step_file(name))
        });
        files.sort();
        for file in files {
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            for span in command_spans(&text) {
                if command_candidate(&span, &self.registered_words)
                    && !command_disqualified(
                        &span,
                        &self.never_executable_extensions,
                        &self.registered_words,
                    )
                    && !self.registered(&span)
                {
                    let message = format!(
                        "{} uses unregistered command literal '{}'; register it with register-command.sh so its 'when' context is recorded",
                        file.file_name().and_then(|n| n.to_str()).unwrap_or_default(), span
                    );
                    if complete {
                        findings.fail(message);
                    } else {
                        findings.warn(message);
                    }
                }
            }
        }
    }

    fn registered(&self, span: &str) -> bool {
        self.commands.iter().any(|command| {
            span == command
                || span
                    .strip_prefix(command)
                    .is_some_and(|rest| rest.starts_with(' '))
        })
    }
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

fn is_step_file(name: &str) -> bool {
    let Some(rest) = name.strip_suffix(".md") else {
        return false;
    };
    rest.len() > 7
        && rest.as_bytes()[0..2].iter().all(u8::is_ascii_digit)
        && rest[2..].starts_with("-step-")
}

fn command_spans(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut in_fence = false;
    for raw in text.lines() {
        if raw.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        let mut line = raw.trim_start().to_owned();
        if line.is_empty() {
            continue;
        }
        if in_fence {
            line = strip_prefix(line, "$ ");
            line = strip_prefix(line, "# ");
            if line.starts_with("bin/")
                || line.starts_with("vendor/")
                || line.starts_with("./")
                || line.starts_with("~/")
                || (line.starts_with('/')
                    && (line.matches('/').count() >= 2
                        || line.ends_with(".sh")
                        || line.contains(char::is_whitespace)))
                || builtin_command(&line)
            {
                result.push(format!("LINE:{line}"));
            }
            continue;
        }
        let mut remainder = line.as_str();
        while let Some(start) = remainder.find('`') {
            let after = &remainder[start + 1..];
            let Some(end) = after.find('`') else { break };
            let span = after[..end].trim().to_owned();
            let span = strip_prefix(span, "$ ");
            if !span.is_empty() {
                result.push(format!("SPAN:{span}"));
            }
            remainder = &after[end + 1..];
        }
        if line.starts_with("bin/")
            || line.starts_with("vendor/")
            || line.starts_with("./")
            || line.starts_with("~/")
            || (line.starts_with('/')
                && (line.matches('/').count() >= 2
                    || line.ends_with(".sh")
                    || line.contains(char::is_whitespace)))
            || builtin_command(&line)
        {
            result.push(format!("LINE:{line}"));
        }
    }
    result.sort();
    result.dedup();
    result
        .into_iter()
        .map(|candidate| candidate[5..].to_owned())
        .collect()
}

fn strip_prefix(mut value: String, prefix: &str) -> String {
    if value.starts_with(prefix) {
        value.drain(..prefix.len());
    }
    value
}

fn builtin_command(line: &str) -> bool {
    let token = line.split_whitespace().next().unwrap_or_default();
    CORE_WORDS
        .iter()
        .chain(BUILTIN_WORDS)
        .any(|word| *word == token)
        && line.contains(char::is_whitespace)
}

fn command_candidate(span: &str, registered_words: &BTreeSet<String>) -> bool {
    let token = span.split_whitespace().next().unwrap_or_default();
    // Only the small cross-language core is a candidate by bare word alone
    // (matches `command_shaped`'s own `core_words`, which excludes
    // BUILTIN_WORDS -- those wider names only widen the fenced-code-block
    // LINE gate in `builtin_command`, one rung earlier). A bare `composer` or
    // `phpunit` invocation is silent until registered or path-shaped: rules
    // 1-3 are the only entry points, and this list is rule 1's static half.
    CORE_WORDS.contains(&token)
        || registered_words.contains(token)
        || bin_under(token)
        || (token.contains('/')
            && fs::metadata(token).is_ok_and(|meta| meta.is_file() && is_executable(token)))
}

fn command_disqualified(
    span: &str,
    never_executable_extensions: &BTreeSet<String>,
    registered_words: &BTreeSet<String>,
) -> bool {
    let token = span.split_whitespace().next().unwrap_or_default();
    let last = span.split_whitespace().last().unwrap_or_default();
    let extension = last
        .rsplit_once('.')
        .map(|(_, ext)| format!(".{ext}"))
        .unwrap_or_default()
        .to_ascii_lowercase();
    if never_executable_extensions.contains(&extension)
        || span.contains(':') && span.chars().any(|c| c.is_ascii_digit())
        || span.contains("#L")
        || span.contains("#l")
    {
        return true;
    }
    if span.starts_with('/') && !bin_like(span) && !command_candidate(token, registered_words) {
        return true;
    }
    if let Some(arg) = span.split_whitespace().nth(1) {
        if arg.starts_with('/') && !bin_like(span) && !command_candidate(token, registered_words) {
            return true;
        }
    }
    fs::metadata(token).is_ok_and(|meta| meta.is_dir())
}

fn bin_like(path: &str) -> bool {
    path.split('/')
        .any(|segment| matches!(segment, "bin" | "sbin" | ".bin" | "Scripts"))
}
fn bin_under(token: &str) -> bool {
    token.rsplit_once('/').is_some_and(|(parent, _)| {
        matches!(
            parent.rsplit('/').next(),
            Some("bin" | "sbin" | ".bin" | "Scripts")
        )
    })
}
fn is_executable(_path: &str) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(_path).is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        // No permission bits to read: a file is a command when its extension
        // says it runs (the same set Git for Windows' bash and cmd start).
        executable_by_extension(_path)
    }
}

#[cfg_attr(unix, allow(dead_code))]
fn executable_by_extension(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| {
            matches!(
                extension.as_str(),
                "exe" | "bat" | "cmd" | "com" | "ps1" | "sh"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use planning_validator_common::Findings;
    use std::fs;
    #[test]
    fn detects_fenced_and_inline_commands() {
        let spans = command_spans("Use `git status` here.\n```sh\n$ npm test\n```");
        assert!(spans.contains(&"git status".into()));
        assert!(spans.contains(&"npm test".into()));
    }
    #[test]
    fn bin_paths_are_candidates_but_citations_are_not() {
        assert!(command_candidate("bin/tool --check", &BTreeSet::new()));
        assert!(command_disqualified(
            "docs/report.md:12",
            &BTreeSet::from([".md".into()]),
            &BTreeSet::new(),
        ));
    }

    #[test]
    fn without_permission_bits_an_extension_says_what_is_a_command() {
        for command in ["vendor/x/tool.sh", "tools/run.EXE", "a/b.cmd", "a/b.ps1"] {
            assert!(executable_by_extension(command), "{command}");
        }
        for cited in [
            "vendor/x/config.xml",
            "generated/Foo.php",
            "README.md",
            "bin/tool",
        ] {
            assert!(!executable_by_extension(cited), "{cited}");
        }
    }

    #[test]
    fn a_registered_commands_first_word_is_a_candidate_even_unregistered() {
        // Registering "pytest -q" teaches "pytest" as a tool word (report 20
        // §3), so a DIFFERENT pytest invocation ("pytest --forked") is still a
        // candidate worth checking, even though it is not itself registered.
        let registered_words = BTreeSet::from(["pytest".to_owned()]);
        assert!(command_candidate("pytest --forked", &registered_words));
        assert!(!command_candidate("pytest --forked", &BTreeSet::new()));
    }

    #[test]
    fn absent_registry_does_not_scan_the_plan() {
        let root = std::env::temp_dir().join(format!("validator-commands-{}", std::process::id()));
        let _ = fs::create_dir_all(root.join("goal/steps"));
        fs::write(
            root.join("goal/steps/01-step-test.md"),
            "Run `git status`.\n",
        )
        .unwrap();
        let mut findings = Findings::default();
        CommandRegistry::from_file(&root.join("commands.json")).validate_plan(
            &root,
            false,
            &mut findings,
        );
        assert_eq!(findings.errors, 0);
        assert_eq!(findings.warnings, 0);
        let _ = fs::remove_dir_all(root);
    }
}
