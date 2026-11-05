// MODE: DEV
// PACKAGE: PROD
//! One shell command against the gate: nested shells and substitutions first, then
//! each simple command in it.

use super::lex::{segments_of, Segment};
use super::script::{check_interpreter, is_interpreter};
use super::search::{check_awk, check_searcher, check_sed, SEARCHERS};
use super::{expand_home, resolve, search_message, write_message};
use regex::Regex;
use std::sync::OnceLock;

const SHELLS: &[&str] = &["bash", "sh", "zsh", "dash"];
const WRAPPERS: &[&str] = &[
    "sudo", "doas", "command", "builtin", "exec", "env", "time", "nice", "ionice", "nohup",
    "stdbuf", "timeout", "gtimeout", "xargs", "then", "do", "else", "elif", "if", "while", "until",
    "!", "{", "}", "fi", "done",
];
const MAX_NESTING: usize = 3;

pub(super) fn cached(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("a valid built-in pattern"))
}

fn is_assignment(word: &str) -> bool {
    match word.find('=') {
        Some(index) if index > 0 => {
            let name = word[..index].strip_suffix('+').unwrap_or(&word[..index]);
            !name.is_empty()
                && !name.starts_with(|c: char| c.is_ascii_digit())
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

fn base_name(word: &str) -> &str {
    word.rsplit(['/', '\\']).next().unwrap_or(word)
}

/// The command a segment really runs: wrappers and assignments removed, and the
/// first word reduced to its base name.
pub(super) fn strip_wrappers(words: &[String]) -> Vec<String> {
    let mut rest = words;
    while let Some(word) = rest.first() {
        let base = base_name(word);
        if is_assignment(word) {
            rest = &rest[1..];
        } else if WRAPPERS.contains(&base) {
            let takes_count = matches!(base, "timeout" | "gtimeout");
            rest = &rest[1..];
            while rest
                .first()
                .is_some_and(|w| w.starts_with('-') || is_assignment(w))
            {
                rest = &rest[1..];
            }
            if takes_count && !rest.is_empty() {
                rest = &rest[1..];
            }
        } else {
            break;
        }
    }
    let mut out = rest.to_vec();
    if let Some(first) = out.first_mut() {
        *first = base_name(first).to_string();
    }
    out
}

fn allowed_write_target(target: &str, cwd: &str) -> bool {
    if target.starts_with("/dev/") || target.starts_with('&') {
        return true;
    }
    if target.contains('$') {
        let upper = target.to_uppercase();
        return upper.contains("TMP") || upper.contains("SCRATCH");
    }
    let path = resolve(cwd, target);
    let mut roots = vec![
        "/tmp".to_string(),
        "/private/tmp".to_string(),
        "/var/folders".to_string(),
        "/private/var/folders".to_string(),
        expand_home("~/.config/tsch-ai-skills/tmp"),
    ];
    roots.extend(std::env::var("TMPDIR"));
    roots.extend(std::env::var("TEMP"));
    roots.iter().any(|root| path.starts_with(root))
}

fn check_shell_write(segment: &Segment, name: &str, args: &[String]) -> Option<String> {
    let tee_targets: Vec<String>;
    let targets: &[String] = match name {
        "tee" => {
            tee_targets = args
                .iter()
                .filter(|a| !a.starts_with('-'))
                .cloned()
                .collect();
            &tee_targets
        }
        "echo" | "printf" | ":" | "true" | "" => &segment.out_targets,
        "cat" if segment.heredoc.is_some() => &segment.out_targets,
        _ => return None,
    };
    let target = targets
        .iter()
        .find(|target| !allowed_write_target(target, &segment.cwd))?;
    let tool = if name.is_empty() {
        "a bare redirect"
    } else {
        name
    };
    Some(format!("{tool} to {target}"))
}

fn check_nested(name: &str, args: &[String], cwd: &str, depth: usize) -> Option<String> {
    if SHELLS.contains(&name) {
        let flag = args.iter().position(|arg| arg == "-c")?;
        return check(args.get(flag + 1)?, cwd, depth + 1);
    }
    if name == "eval" {
        return check(&args.join(" "), cwd, depth + 1);
    }
    None
}

fn check_segment(segments: &[Segment], segment: &Segment, depth: usize) -> Option<String> {
    let words = strip_wrappers(&segment.words);
    let (name, args) = match words.split_first() {
        Some((name, args)) => (name.as_str(), args),
        None => ("", &[][..]),
    };
    if let Some(found) = check_nested(name, args, &segment.cwd, depth) {
        return Some(found);
    }
    let searching = if SEARCHERS.contains(&name) {
        check_searcher(segments, segment, name, args)
    } else if matches!(name, "sed" | "gsed") {
        check_sed(segments, segment, args)
    } else if matches!(name, "awk" | "gawk" | "mawk") {
        check_awk(segments, segment, args)
    } else if is_interpreter(name) {
        check_interpreter(segments, segment, name, args)
    } else {
        None
    };
    searching
        .map(|what| search_message(&what))
        .or_else(|| check_shell_write(segment, name, args).map(|what| write_message(&what)))
}

/// The denial for this command, or `None`. Text that does not lex is let through,
/// because an unterminated quote is refused by the shell as well.
pub(super) fn check(command: &str, cwd: &str, depth: usize) -> Option<String> {
    static SUBSTITUTION: OnceLock<Regex> = OnceLock::new();
    if depth > MAX_NESTING {
        return None;
    }
    let substitution = cached(&SUBSTITUTION, r"\$\(([^()]*)\)");
    for capture in substitution.captures_iter(command) {
        if let Some(found) = check(&capture[1], cwd, depth + 1) {
            return Some(found);
        }
    }
    let segments = segments_of(command, cwd)?;
    segments
        .iter()
        .find_map(|segment| check_segment(&segments, segment, depth))
}
