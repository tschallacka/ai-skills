// MODE: DEV
// PACKAGE: PROD
//! Argument parsing.
//!
//! Hand-rolled rather than a parser crate, for the same reason plan-crypt has
//! no dependencies: the flag set is small, fixed, and the binary ships. Two
//! dependencies for JSON are worth it because hand-rolling a JSON parser is how
//! you get a subtly wrong one; a `--flag value` loop is not in that category.
//!
//! Every value-taking flag refuses a missing value rather than consuming the
//! next flag as its argument, which is the one mistake this shape invites.
//!
//! Nothing here knows what a register holds. The vocabularies live beside the
//! enums in each register's own `register.rs`, so this file is IDENTICAL in the
//! bug-report and todo crates and a test pins that — two copies of a parser that
//! had drifted apart would be worse than one shared crate, and the drift is what
//! the pin prevents.

use std::collections::HashMap;
use std::io::Read;

pub struct Args {
    pub command: String,
    pub positional: Vec<String>,
    flags: HashMap<String, String>,
}

#[derive(Debug)]
pub enum ParseError {
    MissingValue(String),
    UnknownFlag(String),
}

/// Whether `value` is itself one of `known_flags` in `--name` form, rather
/// than a value that merely starts with `--` (B242: a title or note about a
/// flag, e.g. `--takeover-stale-endpoint is unreachable`, is not that flag).
fn is_known_flag(value: &str, known_flags: &[&str]) -> bool {
    value
        .strip_prefix("--")
        .is_some_and(|name| known_flags.contains(&name))
}

/// `known_switches` are the flags that take no value. Anything else beginning
/// with `-` is expected to take one, so a typo produces "unknown flag" rather
/// than silently swallowing the next argument.
pub fn parse(argv: &[String], known_flags: &[&str]) -> Result<Args, ParseError> {
    let mut command = String::new();
    let mut positional = Vec::new();
    let mut flags = HashMap::new();

    let mut index = 0;
    while index < argv.len() {
        let arg = &argv[index];
        if let Some(name) = arg.strip_prefix("--") {
            let name = name.to_string();
            if !known_flags.contains(&name.as_str()) {
                return Err(ParseError::UnknownFlag(format!("--{name}")));
            }
            match argv.get(index + 1) {
                Some(value) if !is_known_flag(value, known_flags) => {
                    flags.insert(name, value.clone());
                    index += 2;
                }
                _ => return Err(ParseError::MissingValue(format!("--{name}"))),
            }
            continue;
        }
        if command.is_empty() {
            command = arg.clone();
        } else {
            positional.push(arg.clone());
        }
        index += 1;
    }

    Ok(Args {
        command,
        positional,
        flags,
    })
}

impl Args {
    pub fn flag(&self, name: &str) -> Option<&str> {
        self.flags.get(name).map(String::as_str)
    }

    /// A comma-separated list, with the empty case being an empty vector rather
    /// than one empty string.
    pub fn list(&self, name: &str) -> Vec<String> {
        self.flag(name)
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Resolve `<name>-file` into `<name>`: read the named file, or stdin for
    /// `-`, and store its bytes verbatim. Refused if `<name>` is also set --
    /// only one may be the source.
    pub fn resolve_file_flag(&mut self, name: &str, file_flag: &str) -> Result<(), String> {
        let Some(path) = self.flags.remove(file_flag) else {
            return Ok(());
        };
        if self.flags.contains_key(name) {
            return Err(format!(
                "--{name} and --{file_flag} are mutually exclusive; use one"
            ));
        }
        let text = if path == "-" {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|error| format!("cannot read stdin for --{file_flag}: {error}"))?;
            buf
        } else {
            std::fs::read_to_string(&path)
                .map_err(|error| format!("cannot read {path} for --{file_flag}: {error}"))?
        };
        self.flags.insert(name.to_string(), text);
        Ok(())
    }
}

/// Parse an enum from its on-disk spelling, listing the vocabulary on failure.
/// Going through serde means the accepted words are exactly the ones the
/// register stores — there is no second list to drift.
pub fn enum_value<T: serde::de::DeserializeOwned>(
    raw: &str,
    field: &str,
    allowed: &[&str],
) -> Result<T, String> {
    serde_json::from_value::<T>(serde_json::Value::String(raw.to_string()))
        .map_err(|_| format!("{field} \"{raw}\" is not one of: {}", allowed.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn a_flag_missing_its_value_is_an_error_not_a_swallowed_flag() {
        // The trap this shape invites: --title consuming --severity as its text.
        let result = parse(
            &argv(&["add", "--title", "--severity"]),
            &["title", "severity"],
        );
        assert!(matches!(result, Err(ParseError::MissingValue(_))));
    }

    /// B242: a title naming a flag is a legitimate value, not an attempt to
    /// pass that flag, as long as it is not the bare flag itself.
    #[test]
    fn a_value_that_merely_starts_with_dashes_is_accepted() {
        let args = parse(
            &argv(&["add", "--title", "--takeover-stale-endpoint is unreachable"]),
            &["title"],
        )
        .expect("a value starting with -- is still a value");
        assert_eq!(
            args.flag("title"),
            Some("--takeover-stale-endpoint is unreachable")
        );
    }

    #[test]
    fn an_unknown_flag_is_named_rather_than_ignored() {
        let result = parse(&argv(&["add", "--ttile", "x"]), &["title"]);
        match result {
            Err(ParseError::UnknownFlag(name)) => assert_eq!(name, "--ttile"),
            _ => panic!("expected the typo to be reported"),
        }
    }

    #[test]
    fn a_flag_and_its_value_are_paired() {
        let args = parse(&argv(&["list", "--status", "open"]), &["status"]).expect("parses");
        assert_eq!(args.command, "list");
        assert_eq!(args.flag("status"), Some("open"));
    }

    #[test]
    fn a_comma_list_drops_empty_parts() {
        let args = parse(&argv(&["add", "--paths", "a.sh, b.sh,,"]), &["paths"]).expect("parses");
        assert_eq!(args.list("paths"), vec!["a.sh", "b.sh"]);
    }

    // A local enum rather than one of the register's, so this file depends on
    // nothing in the crate around it and can be pinned identical across both.
    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum Colour {
        Red,
        SeaGreen,
    }

    #[test]
    fn an_out_of_vocabulary_word_lists_the_vocabulary() {
        let allowed = &["red", "sea-green"];
        let error = enum_value::<Colour>("purple", "--colour", allowed)
            .expect_err("purple is not in the vocabulary");
        assert!(error.contains("sea-green"), "{error}");
        assert!(error.contains("purple"), "{error}");
        // The kebab-case spelling is the accepted one, since serde defines it.
        assert!(enum_value::<Colour>("sea-green", "--colour", allowed).is_ok());
        assert!(enum_value::<Colour>("SeaGreen", "--colour", allowed).is_err());
    }

    #[test]
    fn a_file_flag_reads_the_file_verbatim() {
        let dir = std::env::temp_dir().join(format!("cli-file-flag-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("note.txt");
        std::fs::write(&path, "it's a `note` with shell-hostile characters").unwrap();

        let mut args = parse(
            &argv(&["add", "--note-file", path.to_str().unwrap()]),
            &["note", "note-file"],
        )
        .unwrap();
        args.resolve_file_flag("note", "note-file").unwrap();
        assert_eq!(
            args.flag("note"),
            Some("it's a `note` with shell-hostile characters")
        );
    }

    #[test]
    fn both_the_plain_flag_and_its_file_form_are_refused_together() {
        let mut args = parse(
            &argv(&["add", "--note", "argv text", "--note-file", "/dev/null"]),
            &["note", "note-file"],
        )
        .unwrap();
        assert!(args.resolve_file_flag("note", "note-file").is_err());
    }

    #[test]
    fn a_missing_file_is_refused_naming_the_flag() {
        let mut args = parse(
            &argv(&["add", "--note-file", "/nonexistent/definitely-missing.txt"]),
            &["note-file"],
        )
        .unwrap();
        let result = args.resolve_file_flag("note", "note-file");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("note-file"));
    }

    #[test]
    fn no_file_flag_given_leaves_the_plain_flag_untouched() {
        let mut args = parse(
            &argv(&["add", "--note", "argv text"]),
            &["note", "note-file"],
        )
        .unwrap();
        args.resolve_file_flag("note", "note-file").unwrap();
        assert_eq!(args.flag("note"), Some("argv text"));
    }
}
