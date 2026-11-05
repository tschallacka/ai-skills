// MODE: DEV
// PACKAGE: PROD
//! A script whose only job is searching one file: inline, in a here-document, or
//! small and on disk.

use super::bash::cached;
use super::lex::Segment;
use super::resolve;
use super::search::fed_file;
use regex::Regex;
use std::sync::OnceLock;

const MAX_SCRIPT_BYTES: u64 = 8192;
const MAX_SCRIPT_LINES: usize = 40;
const SCRIPT_SUFFIXES: &[&str] = &[".py", ".rb", ".pl", ".js", ".mjs", ".cjs"];

/// Reads exactly one file and searches it, with no sign of doing anything else:
/// no directory walk, no parsing, no network, no write.
fn only_searches_one_file(body: &str) -> bool {
    static SEARCH: OnceLock<Regex> = OnceLock::new();
    static READ: OnceLock<Regex> = OnceLock::new();
    static OTHER_WORK: OnceLock<Regex> = OnceLock::new();
    let search = cached(
        &SEARCH,
        r"\bre\.(?:search|match|fullmatch|findall|finditer|compile)\b|\.find\(|\.index\(|\.count\(|\.includes\(|\.indexOf\(|\.test\(|\.match\(|\.scan\(|\.grep\b|=~|\.startswith\(|\.endswith\(|\bif\b[^\n]*\bin\s+\w*(?:line|text|content|contents|data|src|source|body|buf)\w*\b",
    );
    let read = cached(
        &READ,
        r"\bopen\s*\(|\bopen\s+my\b|\.read_text\b|\.read_bytes\b|\breadFileSync\b|\b(?:File|IO)\.(?:read|foreach|open|readlines)\b",
    );
    let other_work = cached(
        &OTHER_WORK,
        r#"os\.walk|\bglob\b|\brglob\b|iterdir|listdir|readdir|subprocess|os\.system|urllib|requests|socket|\bhttp\b|\bjson\b|\byaml\b|\bxml\b|\bcsv\b|\btoml\b|sqlite3|bs4|BeautifulSoup|\bast\b|pandas|write_text|writeFileSync|File\.write|open\s*\([^)]*['"][wax]b?\+?['"]"#,
    );
    let writes = body
        .match_indices(".write(")
        .any(|(index, _)| !body[..index].ends_with("stdout"));
    !body.is_empty()
        && body.lines().count() < MAX_SCRIPT_LINES
        && !writes
        && !other_work.is_match(body)
        && search.is_match(body)
        && read.find_iter(body).count() == 1
}

pub(super) fn is_interpreter(name: &str) -> bool {
    matches!(name, "ruby" | "perl" | "node")
        || name
            .strip_prefix("python")
            .is_some_and(|rest| rest.chars().all(|c| c.is_ascii_digit() || c == '.'))
}

fn script_file_body(segment: &Segment, path: &str) -> Option<String> {
    if !SCRIPT_SUFFIXES.iter().any(|suffix| path.ends_with(suffix)) {
        return None;
    }
    let full = resolve(&segment.cwd, path);
    if std::fs::metadata(&full).ok()?.len() > MAX_SCRIPT_BYTES {
        return None;
    }
    let bytes = std::fs::read(full).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

#[derive(Default)]
struct ScriptArgs {
    body: Option<String>,
    line_loop: bool,
    in_place: bool,
    positional: Vec<String>,
}

fn parse_python_args(args: &[String]) -> ScriptArgs {
    let mut parsed = ScriptArgs::default();
    for (i, arg) in args.iter().enumerate() {
        if arg == "-c" {
            parsed.body = args.get(i + 1).cloned();
            parsed.positional.extend(args.iter().skip(i + 2).cloned());
            break;
        }
        if !arg.starts_with('-') {
            parsed.positional.push(arg.clone());
        }
    }
    parsed
}

fn parse_other_args(name: &str, args: &[String]) -> ScriptArgs {
    let mut parsed = ScriptArgs::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if matches!(arg.as_str(), "--eval" | "--print") {
            parsed.body = args.get(i).cloned();
            i += 1;
        } else if arg.len() > 1 && arg.starts_with('-') && !arg.starts_with("--") {
            let cluster = &arg[1..];
            parsed.in_place |= name == "perl" && cluster.contains('i');
            parsed.line_loop |= matches!(name, "perl" | "ruby") && cluster.contains(['n', 'p']);
            if cluster.contains('e') || (name == "node" && cluster.contains('p')) {
                parsed.body = args.get(i).cloned();
                i += 1;
            }
        } else if !arg.starts_with('-') {
            parsed.positional.push(arg.clone());
        }
    }
    parsed
}

pub(super) fn check_interpreter(
    segments: &[Segment],
    segment: &Segment,
    name: &str,
    args: &[String],
) -> Option<String> {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let parsed = if name.starts_with("python") {
        parse_python_args(args)
    } else {
        parse_other_args(name, args)
    };
    if parsed.in_place {
        return None;
    }
    if let (Some(text), true) = (&parsed.body, parsed.line_loop) {
        if !cached(&PATTERN, r"/.+/|=~").is_match(text) {
            return None;
        }
        let target = fed_file(segments, segment, &parsed.positional, false)?;
        return Some(format!("{name} one-liner search on {target}"));
    }
    let body = parsed
        .body
        .or_else(|| segment.heredoc.clone())
        .or_else(|| script_file_body(segment, parsed.positional.first()?))?;
    only_searches_one_file(&body)
        .then(|| format!("a {name} script whose only job is searching one file"))
}
