// MODE: DEV
// PACKAGE: DEV
//! prune-transcript — shrink provably-static or provably-superseded content
//! inside a Claude Code session transcript (`~/.claude/projects/**/*.jsonl`),
//! in place, without ever deleting a line or touching a structural key
//! (`uuid`, `parentUuid`, `type`).
//!
//! T128: a real session transcript grows a lot of content that is either
//! static (re-injected identically on every resume) or that a maintainer
//! tool ships to re-read from disk anyway. None of that needs to survive
//! in the transcript at full size for the model's own memory to stay
//! intact — only its EXISTENCE and rough size do. This tool only replaces
//! the heavy VALUE of specific, positively-classified fields with a short
//! placeholder; it never removes a line, never touches `uuid`/`parentUuid`/
//! `type`, and it self-verifies that guarantee before any write is ever
//! finalized.
//!
//! Three categories are handled here, each self-contained (one JSON object,
//! no cross-referencing another line):
//!
//!   - `attachment.type == "skill_listing"`: `attachment.content` is the
//!     harness's own skill catalog text, regenerated identically on resume.
//!   - `attachment.type == "prompt_snapshot"`: `attachment.systemPrompt` is
//!     an array of plain strings, each a chunk of this same, static system
//!     prompt boilerplate.
//!   - `attachment.type == "invoked_skills"`: `attachment.skills` is an
//!     array of `{name, path, content}` — `content` is a SKILL.md's full
//!     body, inlined at invocation time; `path` says exactly where to
//!     re-read it from disk.
//!
//! NOT handled here, deliberately: correlating an ai-text-editor `search`
//! result (or its own `--query` argument) with a LATER, same-tab `replace`
//! response's `deleted.text` to prove the search result is now stale. That
//! is the same underlying mechanism (does a candidate string exactly equal
//! some later authoritative text?), but reaching it reliably means either
//! double-decoding a nested JSON string (the MCP tool_result shape) or
//! parsing arbitrary shell stdout for embedded JSON (the Bash-CLI shape),
//! and a wrongly-matched `deleted.text` would shrink content that was not
//! actually superseded. Left for a follow-up once that correlation is
//! worked out and tested on its own.

use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Every placeholder this tool ever writes starts with this, so a second
/// pass recognizes already-pruned content and leaves it alone.
const PRUNE_MARKER: &str = "[pruned:prune-transcript v1]";

fn is_already_pruned(value: &str) -> bool {
    value.starts_with(PRUNE_MARKER)
}

fn placeholder_skill_listing(bytes: usize) -> String {
    format!("{PRUNE_MARKER} {bytes} bytes of skill_listing content -- static, harness-regenerated on resume")
}

fn placeholder_prompt_snapshot(bytes: usize) -> String {
    format!("{PRUNE_MARKER} {bytes} bytes of prompt_snapshot content -- static, harness-regenerated on resume")
}

fn placeholder_invoked_skills(bytes: usize, path: &str) -> String {
    format!("{PRUNE_MARKER} {bytes} bytes of invoked_skills content from {path} -- re-readable from disk")
}

/// One shrink actually applied to one line, for the run summary.
struct ShrinkOutcome {
    category: &'static str,
    bytes_before: usize,
    bytes_after: usize,
}

/// Shrinks `line` (no trailing newline) if -- and only if -- it is one of
/// the three known-safe categories. Anything else, including a line that
/// merely happens to contain one of the marker substrings without matching
/// the real shape, or a line that fails to parse as JSON at all, is
/// returned completely unchanged: the caller re-emits the ORIGINAL bytes
/// for those, never these reserialized ones, so a classification miss can
/// never silently reformat something this tool did not actually touch.
fn shrink_line(line: &str) -> (Option<String>, Vec<ShrinkOutcome>) {
    if !(line.contains("skill_listing")
        || line.contains("prompt_snapshot")
        || line.contains("invoked_skills"))
    {
        return (None, Vec::new());
    }
    let Ok(mut value) = serde_json::from_str::<Value>(line) else {
        return (None, Vec::new());
    };
    let Some(attachment_type) = value
        .pointer("/attachment/type")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return (None, Vec::new());
    };

    let mut outcomes = Vec::new();
    match attachment_type.as_str() {
        "skill_listing" => {
            if let Some(content) = value.pointer_mut("/attachment/content") {
                shrink_string_field(content, "skill_listing", &mut outcomes, |bytes| {
                    placeholder_skill_listing(bytes)
                });
            }
        }
        "prompt_snapshot" => {
            if let Some(entries) = value
                .pointer_mut("/attachment/systemPrompt")
                .and_then(Value::as_array_mut)
            {
                for entry in entries.iter_mut() {
                    shrink_string_field(entry, "prompt_snapshot", &mut outcomes, |bytes| {
                        placeholder_prompt_snapshot(bytes)
                    });
                }
            }
        }
        "invoked_skills" => {
            if let Some(entries) = value
                .pointer_mut("/attachment/skills")
                .and_then(Value::as_array_mut)
            {
                for entry in entries.iter_mut() {
                    let path = entry
                        .get("path")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    if let Some(content) = entry.get_mut("content") {
                        shrink_string_field(content, "invoked_skills", &mut outcomes, |bytes| {
                            placeholder_invoked_skills(bytes, &path)
                        });
                    }
                }
            }
        }
        _ => {}
    }

    if outcomes.is_empty() {
        (None, outcomes)
    } else {
        // Reserialize only now that a real shrink happened; `preserve_order`
        // (see Cargo.toml) keeps every other key in its original position.
        (serde_json::to_string(&value).ok(), outcomes)
    }
}

/// Replaces `field` (if it is a string, and not already a placeholder) with
/// `placeholder(original_len)`, recording the outcome.
fn shrink_string_field(
    field: &mut Value,
    category: &'static str,
    outcomes: &mut Vec<ShrinkOutcome>,
    placeholder: impl FnOnce(usize) -> String,
) {
    let Some(text) = field.as_str() else {
        return;
    };
    if is_already_pruned(text) {
        return;
    }
    let bytes_before = text.len();
    let replacement = placeholder(bytes_before);
    let bytes_after = replacement.len();
    outcomes.push(ShrinkOutcome {
        category,
        bytes_before,
        bytes_after,
    });
    *field = Value::String(replacement);
}

#[derive(Default)]
struct CategoryStats {
    lines: usize,
    bytes_before: usize,
    bytes_after: usize,
}

#[derive(Default)]
struct Summary {
    lines_total: usize,
    lines_touched: usize,
    by_category: BTreeMap<&'static str, CategoryStats>,
}

impl Summary {
    fn record(&mut self, outcomes: &[ShrinkOutcome]) {
        if !outcomes.is_empty() {
            self.lines_touched += 1;
        }
        for outcome in outcomes {
            let stats = self.by_category.entry(outcome.category).or_default();
            stats.lines += 1;
            stats.bytes_before += outcome.bytes_before;
            stats.bytes_after += outcome.bytes_after;
        }
    }

    fn print(&self) {
        println!(
            "prune-transcript: {} lines total, {} touched",
            self.lines_total, self.lines_touched
        );
        for (category, stats) in &self.by_category {
            println!(
                "  {category}: {} field(s), {} -> {} bytes",
                stats.lines, stats.bytes_before, stats.bytes_after
            );
        }
    }
}

/// Splits `bytes` on `\n`, reporting whether the file ended with a trailing
/// newline so it can be reproduced exactly rather than assumed. A line that
/// is not valid UTF-8 is treated exactly like a line that fails to parse as
/// JSON -- passed through untouched, never classified, never risked through
/// a lossy conversion.
fn split_lines(bytes: &[u8]) -> (Vec<&[u8]>, bool) {
    if bytes.is_empty() {
        return (Vec::new(), false);
    }
    let had_trailing_newline = bytes.last() == Some(&b'\n');
    let body = if had_trailing_newline {
        &bytes[..bytes.len() - 1]
    } else {
        bytes
    };
    (body.split(|&b| b == b'\n').collect(), had_trailing_newline)
}

/// Processes raw transcript bytes into (output bytes, summary). Every line
/// that is not one of the three known-safe categories -- including any
/// line that is not valid UTF-8, or that fails to parse as JSON -- is
/// copied through as its ORIGINAL bytes, never reserialized.
fn process(input: &[u8]) -> (Vec<u8>, Summary) {
    let (lines, had_trailing_newline) = split_lines(input);
    let mut summary = Summary {
        lines_total: lines.len(),
        ..Summary::default()
    };
    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    for (i, raw_line) in lines.iter().enumerate() {
        if i > 0 {
            out.push(b'\n');
        }
        match std::str::from_utf8(raw_line) {
            Ok(text) => {
                let (shrunk, outcomes) = shrink_line(text);
                summary.record(&outcomes);
                match shrunk {
                    Some(new_text) => out.extend_from_slice(new_text.as_bytes()),
                    None => out.extend_from_slice(raw_line),
                }
            }
            Err(_) => out.extend_from_slice(raw_line),
        }
    }
    if had_trailing_newline && !lines.is_empty() {
        out.push(b'\n');
    }
    (out, summary)
}

/// Confirms `output` differs from `original` only in the ways `process`
/// above is allowed to: same line count; any line that fails to parse as
/// JSON (on either side) is byte-identical; any line that does parse has
/// an identical top-level key set and identical `uuid`/`parentUuid`/`type`
/// values. Returns the first violation found, if any.
fn verify(original: &[u8], output: &[u8]) -> Result<(), String> {
    let (original_lines, _) = split_lines(original);
    let (output_lines, _) = split_lines(output);
    if original_lines.len() != output_lines.len() {
        return Err(format!(
            "line count differs: {} vs {}",
            original_lines.len(),
            output_lines.len()
        ));
    }
    for (i, (orig, out)) in original_lines.iter().zip(output_lines.iter()).enumerate() {
        let orig_value = std::str::from_utf8(orig)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(s).ok());
        let out_value = std::str::from_utf8(out)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(s).ok());
        match (orig_value, out_value) {
            (Some(ov), Some(nv)) => {
                let same_structure = match (ov.as_object(), nv.as_object()) {
                    (Some(oo), Some(no)) => {
                        let mut same = oo.keys().collect::<std::collections::BTreeSet<_>>()
                            == no.keys().collect::<std::collections::BTreeSet<_>>();
                        for key in ["uuid", "parentUuid", "type"] {
                            if oo.get(key) != no.get(key) {
                                same = false;
                            }
                        }
                        same
                    }
                    (None, None) => ov == nv, // both non-object JSON (rare, e.g. a bare literal)
                    _ => false,
                };
                if !same_structure {
                    return Err(format!("line {}: structural key mismatch", i + 1));
                }
            }
            (None, None) => {
                if orig != out {
                    return Err(format!("line {}: unparseable line was changed", i + 1));
                }
            }
            _ => {
                return Err(format!(
                    "line {}: parseability changed between original and output",
                    i + 1
                ));
            }
        }
    }
    Ok(())
}

fn write_atomic(destination: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = destination.with_extension(format!(
        "{}.tmp-{}",
        destination
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jsonl"),
        std::process::id()
    ));
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, destination)
}

const USAGE: &str = "\
prune-transcript -- shrink provably-static/superseded content in a Claude \
Code session transcript, in place.

Usage:
  prune-transcript INPUT.jsonl --out OUTPUT.jsonl   write a new file
  prune-transcript INPUT.jsonl --in-place           rewrite INPUT.jsonl,
                                                     atomically, only after
                                                     self-verification passes
  prune-transcript INPUT.jsonl --dry-run            report what would shrink;
                                                     writes nothing
  prune-transcript --verify OLD NEW                 standalone structural check
  prune-transcript --help
";

enum Mode {
    Out { input: PathBuf, output: PathBuf },
    InPlace { input: PathBuf },
    DryRun { input: PathBuf },
    Verify { original: PathBuf, output: PathBuf },
    Help,
}

fn parse_args(args: &[String]) -> Result<Mode, String> {
    match args {
        [] => Err("no arguments given".to_owned()),
        [flag] if flag == "--help" || flag == "-h" => Ok(Mode::Help),
        [flag, original, output] if flag == "--verify" => Ok(Mode::Verify {
            original: PathBuf::from(original),
            output: PathBuf::from(output),
        }),
        [input, flag] if flag == "--in-place" => Ok(Mode::InPlace {
            input: PathBuf::from(input),
        }),
        [input, flag] if flag == "--dry-run" => Ok(Mode::DryRun {
            input: PathBuf::from(input),
        }),
        [input, flag, output] if flag == "--out" => Ok(Mode::Out {
            input: PathBuf::from(input),
            output: PathBuf::from(output),
        }),
        _ => Err(format!("unrecognized arguments: {}", args.join(" "))),
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args)? {
        Mode::Help => {
            print!("{USAGE}");
            Ok(())
        }
        Mode::DryRun { input } => {
            let bytes =
                fs::read(&input).map_err(|e| format!("cannot read {}: {e}", input.display()))?;
            let (_, summary) = process(&bytes);
            summary.print();
            println!("(dry run: nothing written)");
            Ok(())
        }
        Mode::Out { input, output } => {
            let bytes =
                fs::read(&input).map_err(|e| format!("cannot read {}: {e}", input.display()))?;
            let (pruned, summary) = process(&bytes);
            verify(&bytes, &pruned)?;
            write_atomic(&output, &pruned)
                .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
            summary.print();
            println!("wrote {}", output.display());
            Ok(())
        }
        Mode::InPlace { input } => {
            let bytes =
                fs::read(&input).map_err(|e| format!("cannot read {}: {e}", input.display()))?;
            let (pruned, summary) = process(&bytes);
            verify(&bytes, &pruned)?;
            write_atomic(&input, &pruned)
                .map_err(|e| format!("cannot write {}: {e}", input.display()))?;
            summary.print();
            println!("rewrote {} in place", input.display());
            Ok(())
        }
        Mode::Verify { original, output } => {
            let original_bytes = fs::read(&original)
                .map_err(|e| format!("cannot read {}: {e}", original.display()))?;
            let output_bytes =
                fs::read(&output).map_err(|e| format!("cannot read {}: {e}", output.display()))?;
            verify(&original_bytes, &output_bytes)?;
            println!(
                "verify: OK ({} vs {})",
                original.display(),
                output.display()
            );
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("prune-transcript: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(json: serde_json::Value) -> String {
        serde_json::to_string(&json).unwrap()
    }

    #[test]
    fn skill_listing_is_shrunk() {
        let input = line(serde_json::json!({
            "uuid": "a", "parentUuid": null, "type": "attachment",
            "attachment": {"type": "skill_listing", "content": "x".repeat(500)}
        }));
        let (shrunk, outcomes) = shrink_line(&input);
        let shrunk = shrunk.expect("must shrink");
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].category, "skill_listing");
        assert!(shrunk.len() < input.len());
        let value: Value = serde_json::from_str(&shrunk).unwrap();
        let content = value["attachment"]["content"].as_str().unwrap();
        assert!(content.starts_with(PRUNE_MARKER));
        assert!(content.contains("500 bytes"));
    }

    #[test]
    fn prompt_snapshot_array_length_is_preserved() {
        let input = line(serde_json::json!({
            "uuid": "a", "type": "attachment",
            "attachment": {"type": "prompt_snapshot", "systemPrompt": ["one".repeat(100), "two".repeat(100), "three".repeat(100)]}
        }));
        let (shrunk, outcomes) = shrink_line(&input);
        let shrunk = shrunk.expect("must shrink");
        assert_eq!(outcomes.len(), 3);
        let value: Value = serde_json::from_str(&shrunk).unwrap();
        let arr = value["attachment"]["systemPrompt"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        for item in arr {
            assert!(item.as_str().unwrap().starts_with(PRUNE_MARKER));
        }
    }

    #[test]
    fn invoked_skills_keeps_name_and_path_shrinks_content() {
        let input = line(serde_json::json!({
            "uuid": "a", "type": "attachment",
            "attachment": {"type": "invoked_skills", "skills": [
                {"name": "todo", "path": "/skills/todo/SKILL.md", "content": "y".repeat(9000)}
            ]}
        }));
        let (shrunk, outcomes) = shrink_line(&input);
        let shrunk = shrunk.expect("must shrink");
        assert_eq!(outcomes.len(), 1);
        let value: Value = serde_json::from_str(&shrunk).unwrap();
        let skill = &value["attachment"]["skills"][0];
        assert_eq!(skill["name"], "todo");
        assert_eq!(skill["path"], "/skills/todo/SKILL.md");
        let content = skill["content"].as_str().unwrap();
        assert!(content.starts_with(PRUNE_MARKER));
        assert!(content.contains("/skills/todo/SKILL.md"));
    }

    #[test]
    fn an_already_pruned_line_is_left_alone_second_time() {
        let input = line(serde_json::json!({
            "uuid": "a", "type": "attachment",
            "attachment": {"type": "skill_listing", "content": "z".repeat(500)}
        }));
        let (once, _) = shrink_line(&input);
        let once = once.expect("first pass shrinks");
        let (twice, outcomes) = shrink_line(&once);
        assert!(twice.is_none(), "second pass must not touch it again");
        assert!(outcomes.is_empty());
    }

    #[test]
    fn a_malformed_line_is_passed_through() {
        let input = "{not valid json, but mentions skill_listing anyway";
        let (shrunk, outcomes) = shrink_line(input);
        assert!(shrunk.is_none());
        assert!(outcomes.is_empty());
    }

    #[test]
    fn an_unrelated_line_type_is_untouched() {
        let input = line(serde_json::json!({
            "uuid": "a", "type": "user", "message": {"role": "user", "content": "hello"}
        }));
        let (shrunk, outcomes) = shrink_line(&input);
        assert!(shrunk.is_none());
        assert!(outcomes.is_empty());
    }

    #[test]
    fn process_preserves_line_count_and_trailing_newline() {
        let a = line(serde_json::json!({"uuid": "a", "type": "user"}));
        let b = line(serde_json::json!({
            "uuid": "b", "type": "attachment",
            "attachment": {"type": "skill_listing", "content": "q".repeat(200)}
        }));
        let input = format!("{a}\n{b}\n");
        let (output, summary) = process(input.as_bytes());
        assert_eq!(summary.lines_total, 2);
        assert_eq!(summary.lines_touched, 1);
        assert!(output.ends_with(b"\n"));
        let (in_lines, _) = split_lines(input.as_bytes());
        let (out_lines, _) = split_lines(&output);
        assert_eq!(in_lines.len(), out_lines.len());
        assert_eq!(
            out_lines[0], in_lines[0],
            "untouched line must be byte-identical"
        );
    }

    #[test]
    fn process_without_trailing_newline_does_not_add_one() {
        let a = line(serde_json::json!({"uuid": "a", "type": "user"}));
        let (output, _) = process(a.as_bytes());
        assert!(!output.ends_with(b"\n"));
    }

    #[test]
    fn verify_passes_for_a_correct_prune() {
        let a = line(serde_json::json!({"uuid": "a", "type": "user"}));
        let b = line(serde_json::json!({
            "uuid": "b", "type": "attachment",
            "attachment": {"type": "skill_listing", "content": "q".repeat(200)}
        }));
        let input = format!("{a}\n{b}\n");
        let (output, _) = process(input.as_bytes());
        verify(input.as_bytes(), &output).expect("a correct prune must verify");
    }

    #[test]
    fn verify_catches_a_tampered_structural_key() {
        let a = line(serde_json::json!({"uuid": "a", "type": "user"}));
        let tampered = line(serde_json::json!({"uuid": "different", "type": "user"}));
        let original = format!("{a}\n");
        let output = format!("{tampered}\n");
        let error = verify(original.as_bytes(), output.as_bytes()).unwrap_err();
        assert!(error.contains("structural key mismatch"), "{error}");
    }

    #[test]
    fn verify_catches_a_line_count_mismatch() {
        let a = line(serde_json::json!({"uuid": "a", "type": "user"}));
        let original = format!("{a}\n");
        let output = format!("{a}\n{a}\n");
        let error = verify(original.as_bytes(), output.as_bytes()).unwrap_err();
        assert!(error.contains("line count differs"), "{error}");
    }
}
