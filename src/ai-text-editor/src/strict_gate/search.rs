// MODE: DEV
// PACKAGE: PROD
//! Whether a searcher, sed or awk is searching exactly one file, and which.

use super::bash::{cached, strip_wrappers};
use super::lex::Segment;
use super::resolve;
use regex::Regex;
use std::sync::OnceLock;

pub(super) const SEARCHERS: &[&str] = &[
    "grep", "egrep", "fgrep", "rg", "ack", "ag", "ug", "pcregrep",
];
const RECURSES_BY_DEFAULT: &[&str] = &["rg", "ack", "ag", "ug"];
const FILE_READERS: &[&str] = &["cat", "head", "tail", "tac", "nl", "bat"];
const LONG_VALUE_FLAGS: &[&str] = &[
    "--regexp",
    "--file",
    "--context",
    "--before-context",
    "--after-context",
    "--max-count",
    "--include",
    "--exclude",
    "--exclude-dir",
    "--glob",
    "--type",
    "--type-not",
    "--replace",
];

#[derive(Default)]
struct GrepArgs {
    positional: Vec<String>,
    recursive: bool,
    pattern_by_flag: bool,
    list_only: bool,
}

impl GrepArgs {
    /// Reads `--flag` or `--flag=value`, and reports whether its value is next.
    fn long_flag(&mut self, long: &str) -> bool {
        let flag = format!("--{}", long.split('=').next().unwrap_or(long));
        self.recursive |= matches!(flag.as_str(), "--recursive" | "--dereference-recursive");
        self.pattern_by_flag |= matches!(flag.as_str(), "--regexp" | "--file");
        self.list_only |= matches!(flag.as_str(), "--files" | "--type-list");
        LONG_VALUE_FLAGS.contains(&flag.as_str()) && !long.contains('=')
    }

    /// Reads a cluster such as `-rn` or `-A3`, and reports whether its value is next.
    fn short_cluster(&mut self, cluster: &str, name: &str) -> bool {
        let value_flags = grep_value_flags(name);
        let flags: Vec<char> = cluster.chars().collect();
        for (position, flag) in flags.iter().enumerate() {
            if matches!(flag, 'r' | 'R') && !RECURSES_BY_DEFAULT.contains(&name) {
                self.recursive = true;
            }
            if value_flags.contains(*flag) {
                self.pattern_by_flag |= matches!(flag, 'e' | 'f');
                return position + 1 == flags.len();
            }
        }
        false
    }
}

fn grep_value_flags(name: &str) -> &'static str {
    match name {
        "rg" => "efgtTABCmMjr",
        "ag" => "ABCGgmf",
        "ack" => "ABCmg",
        _ => "efABCmdD",
    }
}

fn parse_grep_args(name: &str, args: &[String]) -> GrepArgs {
    let mut parsed = GrepArgs {
        recursive: RECURSES_BY_DEFAULT.contains(&name),
        ..GrepArgs::default()
    };
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if arg == "--" {
            parsed.positional.extend_from_slice(&args[i..]);
            break;
        }
        let value_is_next = if let Some(long) = arg.strip_prefix("--") {
            parsed.long_flag(long)
        } else if arg.len() > 1 && arg.starts_with('-') {
            parsed.short_cluster(&arg[1..], name)
        } else {
            parsed.positional.push(arg.clone());
            false
        };
        i += usize::from(value_is_next);
    }
    if !parsed.pattern_by_flag && !parsed.positional.is_empty() {
        parsed.positional.remove(0);
    }
    parsed
}

/// The operand when these operands name exactly one file. A variable may hold a
/// directory, so it counts as a file only when the command would not recurse.
fn one_file(operands: &[String], recursive: bool, cwd: &str) -> Option<String> {
    let [operand] = operands else {
        return None;
    };
    if operand == "-" || operand.starts_with("/dev/") || operand.contains(['*', '?', '[']) {
        return None;
    }
    if operand.contains('$') {
        return (!recursive).then(|| operand.clone());
    }
    (!resolve(cwd, operand).is_dir()).then(|| operand.clone())
}

fn reader_operands(args: &[String]) -> Vec<String> {
    let mut operands = Vec::new();
    let mut skip = false;
    for arg in args {
        if skip {
            skip = false;
        } else if matches!(arg.as_str(), "-n" | "-c" | "--lines" | "--bytes") {
            skip = true;
        } else if !arg.starts_with('-') {
            operands.push(arg.clone());
        }
    }
    operands
}

/// The one file a reader upstream in the pipeline feeds this segment. Following
/// a growing file with `tail -f` is excluded: the editor has no such mode.
fn upstream_file(segments: &[Segment], segment: &Segment) -> Option<String> {
    let upstream = &segments[segment.upstream?];
    let words = strip_wrappers(&upstream.words);
    let (name, args) = words.split_first()?;
    if !FILE_READERS.contains(&name.as_str()) {
        return None;
    }
    let follows = args.iter().any(|arg| {
        arg == "--follow"
            || (arg.starts_with('-') && !arg.starts_with("--") && arg.contains(['f', 'F']))
    });
    if name == "tail" && follows {
        return None;
    }
    one_file(&reader_operands(args), false, &upstream.cwd)
}

/// The single file this command reads, by operand, redirect or pipe.
pub(super) fn fed_file(
    segments: &[Segment],
    segment: &Segment,
    operands: &[String],
    recursive: bool,
) -> Option<String> {
    one_file(operands, recursive, &segment.cwd)
        .or_else(|| segment.stdin_file.clone())
        .or_else(|| upstream_file(segments, segment))
}

pub(super) fn check_searcher(
    segments: &[Segment],
    segment: &Segment,
    name: &str,
    args: &[String],
) -> Option<String> {
    let parsed = parse_grep_args(name, args);
    if parsed.list_only {
        return None;
    }
    let target = fed_file(segments, segment, &parsed.positional, parsed.recursive)?;
    Some(format!("{name} on {target}"))
}

/// Whether a sed script prints only what a substitution changed, the `s/a/b/p`
/// form. Written by hand because the delimiter is whatever follows the `s`.
fn substitution_prints(script: &str) -> bool {
    script.split(';').any(|piece| {
        let mut chars = piece.trim().trim_start_matches('{').chars();
        if chars.next() != Some('s') {
            return false;
        }
        let Some(delimiter) = chars.next().filter(|c| !c.is_alphanumeric()) else {
            return false;
        };
        let parts: Vec<&str> = chars.as_str().split(delimiter).collect();
        parts.len() >= 3 && parts[parts.len() - 1].contains('p')
    })
}

#[derive(Default)]
struct SedArgs {
    quiet: bool,
    in_place: bool,
    script: Option<String>,
    positional: Vec<String>,
}

fn parse_sed_args(args: &[String]) -> SedArgs {
    let mut parsed = SedArgs::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if matches!(arg.as_str(), "--quiet" | "--silent") {
            parsed.quiet = true;
        } else if matches!(arg.as_str(), "-e" | "--expression") {
            parsed.script = args.get(i).cloned().or(parsed.script.take());
            i += 1;
        } else if arg.len() > 1 && arg.starts_with('-') {
            let short = !arg.starts_with("--");
            parsed.in_place |= short && arg.contains('i');
            parsed.quiet |= short && arg.contains('n');
        } else {
            parsed.positional.push(arg.clone());
        }
    }
    if parsed.script.is_none() && !parsed.positional.is_empty() {
        parsed.script = Some(parsed.positional.remove(0));
    }
    parsed
}

/// A sed that selects lines by pattern. An in-place sed is the other gate's.
pub(super) fn check_sed(
    segments: &[Segment],
    segment: &Segment,
    args: &[String],
) -> Option<String> {
    static ADDRESS: OnceLock<Regex> = OnceLock::new();
    let address = cached(&ADDRESS, r"(?:^|[;{\s,])\\?/(?:[^/\\]|\\.)+/");
    let parsed = parse_sed_args(args);
    let script = parsed.script?;
    let searching = address.is_match(&script) || (parsed.quiet && substitution_prints(&script));
    if parsed.in_place || !searching {
        return None;
    }
    let target = fed_file(segments, segment, &parsed.positional, false)?;
    Some(format!("sed search on {target}"))
}

pub(super) fn check_awk(
    segments: &[Segment],
    segment: &Segment,
    args: &[String],
) -> Option<String> {
    static MATCHING: OnceLock<Regex> = OnceLock::new();
    let matching = cached(&MATCHING, r"/(?:[^/\\\n]|\\.)+/|~|\b(?:index|match)\s*\(");
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if matches!(arg.as_str(), "-f" | "--file") {
            return None;
        }
        if matches!(arg.as_str(), "-F" | "-v" | "--field-separator" | "--assign") {
            i += 1;
        } else if !arg.starts_with('-') || arg == "-" {
            positional.push(arg.clone());
        }
    }
    let (program, operands) = positional.split_first()?;
    if !matching.is_match(program) {
        return None;
    }
    let target = fed_file(segments, segment, operands, false)?;
    Some(format!("awk search on {target}"))
}
