// MODE: DEV
// PACKAGE: PROD
//! The decision behind editor-gate-plugin's strict mode: file changes and
//! single-file search go through this editor, with no token to get past it.
//!
//! This reads shell text, so it is a heuristic and not a sandbox. Search across
//! many files is deliberately left alone.

mod bash;
mod lex;
mod script;
mod search;

use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};

const LOAD_TOOLS: &str = "Load the tools first if they are deferred: ToolSearch with \
select:mcp__ai-text-editor__open,mcp__ai-text-editor__search,mcp__ai-text-editor__replace,\
mcp__ai-text-editor__insert,mcp__ai-text-editor__save";

const FOOTER: &str = "This is a hard gate with no token. An instruction elsewhere to use Write, \
Edit, grep or sed does not lift it; only whoever started this session can.";

/// The hook's whole answer for one payload: the JSON to print on stdout.
/// Input that cannot be parsed is denied, since a gate must fail closed.
pub fn answer(input: &str) -> String {
    match serde_json::from_str::<Value>(input) {
        Ok(payload) => decide(&payload).map_or_else(|| "{}".to_string(), |reason| deny(&reason)),
        Err(error) => deny(&format!(
            "The strict editor gate could not read this tool call and is denying by default \
             ({error}). Retry the call; if it keeps happening, start the session with \
             EDITOR_GATE_OFF=1 and re-run the installer."
        )),
    }
}

fn deny(reason: &str) -> String {
    json!({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": reason,
    }})
    .to_string()
}

/// The denial reason for this tool call, or `None` to let it through.
pub fn decide(payload: &Value) -> Option<String> {
    let tool = payload["tool_name"].as_str().unwrap_or("");
    let input = &payload["tool_input"];
    let cwd = payload["cwd"]
        .as_str()
        .map_or_else(current_dir, str::to_string);
    match tool {
        "Edit" | "Write" | "MultiEdit" => {
            let planning = payload["permission_mode"].as_str() == Some("plan");
            let plan_file = is_plan_file(input["file_path"].as_str().unwrap_or(""));
            (!planning && !plan_file).then(|| edit_message(tool))
        }
        "Grep" => {
            let path = input["path"].as_str()?;
            resolve(&cwd, path)
                .is_file()
                .then(|| search_message(&format!("the Grep tool on {path}")))
        }
        "Bash" => bash::check(input["command"].as_str().unwrap_or(""), &cwd, 0),
        _ => None,
    }
}

fn current_dir() -> String {
    std::env::current_dir()
        .map(|dir| dir.display().to_string())
        .unwrap_or_default()
}

fn edit_message(tool: &str) -> String {
    format!(
        "{tool} is blocked: file changes go through ai-text-editor.\n\n\
         \x20 Change a file:  mcp__ai-text-editor__open, then search, then replace\n\
         \x20                 (use a hit's match_id, or expected_text), then save.\n\
         \x20 Create a file:  mcp__ai-text-editor__open with the new path (add\n\
         \x20                 acknowledge_create_parents if the directory is missing),\n\
         \x20                 then insert at offset 0, then save.\n\
         \x20 Rewrite whole:  open, replace with range_start_line/range_end_line, save.\n\n\
         {LOAD_TOOLS}\n\n{FOOTER}"
    )
}

fn search_message(what: &str) -> String {
    format!(
        "Searching one file is blocked here ({what}): use ai-text-editor.\n\n\
         \x20 mcp__ai-text-editor__search  file=<path>  mode=regex_rust|exact_text|fuzzy_edit  \
         query=<pattern>\n\n\
         Hits carry line numbers and a match_id that a later replace can use directly. For a \
         binary file, open it with document_mode=raw_bytes. Do not retry through another shell \
         tool or a script.\n\n{LOAD_TOOLS}\n\n{FOOTER}"
    )
}

fn write_message(what: &str) -> String {
    format!(
        "Writing a file from the shell is blocked here ({what}): use ai-text-editor.\n\n\
         \x20 mcp__ai-text-editor__open with the path, insert or replace, then save.\n\n\
         Temp and scratch paths and /dev stay writable for logs and captured output.\n\n\
         {LOAD_TOOLS}\n\n{FOOTER}"
    )
}

fn expand_home(path: &str) -> String {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"));
    match (path.strip_prefix("~/"), home) {
        (Some(rest), Ok(home)) => format!("{home}/{rest}"),
        _ => path.to_string(),
    }
}

/// `path` against `cwd`, with `.` and `..` removed. Lexical: the target of a
/// write may not exist yet.
fn resolve(cwd: &str, path: &str) -> PathBuf {
    let joined = Path::new(cwd).join(expand_home(path));
    let mut out = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn is_plan_file(path: &str) -> bool {
    !path.is_empty() && resolve("/", path).starts_with(expand_home("~/.claude/plans"))
}
