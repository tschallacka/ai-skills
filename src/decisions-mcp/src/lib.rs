// MODE: DEV
// PACKAGE: PROD
//! MCP adapter for the decisions register: typed tool calls over the
//! decisions crate directly, no network client and no server. Every call
//! opens DECISIONS.json, does its work, and writes it back -- the same
//! round trip the CLI makes, just without a process per call.

use decisions::{
    add, answer, answer_with_text, implement, list, migrate, remove_options, reorder_options,
    set_option, stub, Filter, NewQuestion, Priority, Status,
};
use serde_json::{json, Value};

const INSTRUCTIONS: &str = "The question register (DECISIONS.json): non-blocking questions an agent \
raised mid-work, with lettered options, a priority, and the branch they came from as context. A \
question's lifecycle is open -> decided -> implemented. Use list_open/list_decided/list_implemented/ \
list_closed/list_urgent to glean questions; add to raise one and keep working with a stubbed \
assumption recorded via stub; answer records the user's pick (moves it to decided) -- pass letter \
for one of the question's own options, or text for the user's own words when none of them fit; once \
a decided pick has actually been carried out in the code, call implement to record that and move it \
to implemented -- a decided question is outstanding work for the agent, not just the user's to \
answer. option_set/option_remove/option_reorder edit what a question offers before anyone has \
picked from it (only while it is still open): option_set adds a new letter or relabels an existing \
one, option_remove takes any number of letters at once atomically, option_reorder takes the full \
list of current letters in the wanted order. This never replaces a harness's own blocking question \
or confirmation mechanism -- it is for the one that does not have to be answered right now.";

pub fn handle(message: Value) -> Value {
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    match method {
        "initialize" => json!({"jsonrpc":"2.0","id":id,"result":{
            "protocolVersion":"2025-06-18",
            "capabilities":{"tools":{}},
            "serverInfo":{"name":"decisions","version":"0.1.0"},
            "instructions": INSTRUCTIONS}}),
        "notifications/initialized" => Value::Null,
        "tools/list" => json!({"jsonrpc":"2.0","id":id,"result":{"tools": tool_definitions()}}),
        "tools/call" => call_tool(id, message.get("params").cloned().unwrap_or_default()),
        _ => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"method not found"}}),
    }
}

fn tool_definitions() -> Vec<Value> {
    let status_filter = json!({"type":"object","properties":{
        "priority":{"type":"string","description":"Narrow further to this priority: urgent, high, normal, low, or someday.","enum":["urgent","high","normal","low","someday"]},
        "branch":{"type":"string","description":"Narrow further to questions raised on this branch."}
    }});
    vec![
        json!({"name":"list_open","description":"Open questions: raised, not yet answered.","inputSchema":status_filter}),
        json!({"name":"list_decided","description":"Questions the user has picked an option for, not yet implemented -- outstanding work for the agent, not the user.","inputSchema":status_filter}),
        json!({"name":"list_implemented","description":"Questions whose decided pick has already been carried out in the code.","inputSchema":status_filter}),
        json!({"name":"list_closed","description":"Questions with a recorded resolution.","inputSchema":status_filter}),
        json!({"name":"list_urgent","description":"Every question of urgent priority, regardless of status.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"add","description":"Raise a non-blocking question: lettered options, a priority, and context (what you stubbed while it stays open). Records the current git branch automatically.","inputSchema":{
            "type":"object",
            "properties":{
                "title":{"type":"string","description":"The question, stated as a question."},
                "options":{"type":"array","items":{"type":"object","properties":{"letter":{"type":"string"},"label":{"type":"string"}},"required":["letter","label"]},"description":"At least one lettered option."},
                "priority":{"type":"string","enum":["urgent","high","normal","low","someday"],"description":"Defaults to normal."},
                "context":{"type":"string","description":"Why the question was raised, and what was stubbed or assumed while it stays open."}
            },
            "required":["title","options"]
        }}),
        json!({"name":"answer","description":"Record the user's pick for a question: sets it to decided. Give letter for one of the question's own options, or text for the user's own words when none of them fit -- exactly one of the two.","inputSchema":{
            "type":"object",
            "properties":{
                "id":{"type":"string"},
                "letter":{"type":"string","description":"One of the question's own option letters."},
                "text":{"type":"string","description":"The user's own words, when the pick is not one of the offered letters."}
            },
            "required":["id"]
        }}),
        json!({"name":"stub","description":"Record what was assumed or stubbed on an open question, without changing its status.","inputSchema":{
            "type":"object",
            "properties":{"id":{"type":"string"},"assumption":{"type":"string"}},
            "required":["id","assumption"]
        }}),
        json!({"name":"implement","description":"Mark a decided question as carried out in the code, optionally recording what was done. Refused unless the question is currently decided.","inputSchema":{
            "type":"object",
            "properties":{"id":{"type":"string"},"note":{"type":"string","description":"What was implemented, for the record. Optional."}},
            "required":["id"]
        }}),
        json!({"name":"option_set","description":"Add a new lettered option, or relabel an existing one if the letter is already offered. Refused once the question is no longer open.","inputSchema":{
            "type":"object",
            "properties":{"id":{"type":"string"},"letter":{"type":"string"},"label":{"type":"string"}},
            "required":["id","letter","label"]
        }}),
        json!({"name":"option_remove","description":"Remove one or more of a question's options at once, atomically: every named letter must exist and at least one option must remain, or nothing changes. Refused once the question is no longer open.","inputSchema":{
            "type":"object",
            "properties":{"id":{"type":"string"},"letters":{"type":"array","items":{"type":"string"},"description":"The option letters to remove."}},
            "required":["id","letters"]
        }}),
        json!({"name":"option_reorder","description":"Resort a question's options into exactly the given letter order -- a permutation of its current letters, never a way to add, drop, or rename one. Refused once the question is no longer open.","inputSchema":{
            "type":"object",
            "properties":{"id":{"type":"string"},"order":{"type":"array","items":{"type":"string"},"description":"Every current option letter, in the wanted order."}},
            "required":["id","order"]
        }}),
    ]
}

fn tool_error(id: Value, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":message}})
}

/// `DECISIONS_JSON` beats everything, exactly as today. Otherwise, the
/// shared registers worktree is used silently once ALREADY recognized, or
/// an explicit `registers_access = "main-checkout"` opinion always wins even
/// over a stray already-existing worktree. Nothing here ever prompts or
/// creates a worktree: an MCP adapter has no attachable terminal, so
/// `is_terminal()` is always false and a prompt would silently resolve to
/// its own non-interactive default anyway -- the call is skipped entirely
/// rather than invoked for nothing. The first-use prompt, and any worktree
/// creation it leads to, is left to whichever CLI the person runs directly.
fn resolved_register_path() -> String {
    if let Ok(from_env) = std::env::var("DECISIONS_JSON") {
        if !from_env.is_empty() {
            return from_env;
        }
    }
    if let Ok(project) = planning_core::project_root_for(None) {
        let cfg = planning_core::read_tsch_config(&project);
        if cfg.registers_access.as_deref() == Some("main-checkout") {
            return "DECISIONS.json".to_string();
        }
        if let Ok(candidate) = planning_core::registers_scoped_root(&project) {
            if planning_core::worktree_recognized(&candidate) {
                return candidate.join("DECISIONS.json").display().to_string();
            }
        }
    }
    "DECISIONS.json".to_string()
}

fn load_register() -> Result<decisions::Register, String> {
    let path = resolved_register_path();
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let loose: Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
    let claimed = migrate::claimed_version(&loose);
    if migrate::is_current(&claimed) {
        return serde_json::from_value(loose).map_err(|e| format!("{path}: {e}"));
    }
    let (carried, _archived, _unconvertible) = migrate::attempt(&loose);
    Ok(migrate::rebuilt(&loose, carried))
}

fn save_register(register: &decisions::Register) -> Result<(), String> {
    let path = resolved_register_path();
    let mut text = serde_json::to_string_pretty(register).map_err(|e| e.to_string())?;
    text.push('\n');
    std::fs::write(&path, text).map_err(|e| format!("{path}: {e}"))
}

fn string_argument(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn parse_priority(value: &str) -> Option<Priority> {
    serde_json::from_value(Value::String(value.to_lowercase())).ok()
}

fn tool_result(text: String) -> Value {
    json!({"content":[{"type":"text","text":text}]})
}

fn list_result(register: &decisions::Register, filter: &Filter) -> Value {
    let matching: Vec<&decisions::Question> = list(register, filter);
    tool_result(serde_json::to_string(&matching).unwrap_or_default())
}

const TOOL_NAMES: &[&str] = &[
    "list_open",
    "list_decided",
    "list_implemented",
    "list_closed",
    "list_urgent",
    "add",
    "answer",
    "stub",
    "implement",
    "option_set",
    "option_remove",
    "option_reorder",
];

fn call_tool(id: Value, params: Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    if !TOOL_NAMES.contains(&name) {
        return tool_error(id, &format!("unknown tool: {name}"));
    }
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let register = match load_register() {
        Ok(register) => register,
        Err(message) => return tool_error(id, &message),
    };

    let result = match name {
        "list_open" | "list_decided" | "list_implemented" | "list_closed" => {
            let mut filter = Filter {
                status: Some(match name {
                    "list_open" => Status::Open,
                    "list_decided" => Status::Decided,
                    "list_implemented" => Status::Implemented,
                    _ => Status::Closed,
                }),
                priority: None,
                branch: None,
            };
            if let Some(value) = string_argument(&arguments, "priority") {
                match parse_priority(&value) {
                    Some(priority) => filter.priority = Some(priority),
                    None => return tool_error(id, &format!("unknown priority: {value}")),
                }
            }
            filter.branch = string_argument(&arguments, "branch");
            Ok(list_result(&register, &filter))
        }
        "list_urgent" => Ok(list_result(
            &register,
            &Filter {
                status: None,
                priority: Some(Priority::Urgent),
                branch: None,
            },
        )),
        "add" => add_tool(register, &arguments),
        "answer" => answer_tool(register, &arguments),
        "stub" => mutate_tool(register, &arguments, "id", "assumption", stub),
        "implement" => implement_tool(register, &arguments),
        "option_set" => option_set_tool(register, &arguments),
        "option_remove" => option_remove_tool(register, &arguments),
        "option_reorder" => option_reorder_tool(register, &arguments),
        _ => unreachable!("checked against TOOL_NAMES above"),
    };

    match result {
        Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":value}),
        Err(message) => tool_error(id, &message),
    }
}

fn add_tool(mut register: decisions::Register, arguments: &Value) -> Result<Value, String> {
    let title = string_argument(arguments, "title").ok_or("title is required")?;
    let options: Vec<decisions::Choice> = arguments
        .get("options")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| format!("options: {e}"))?
        .unwrap_or_default();
    let priority = match string_argument(arguments, "priority") {
        Some(value) => parse_priority(&value).ok_or(format!("unknown priority: {value}"))?,
        None => Priority::Normal,
    };
    let context = string_argument(arguments, "context").unwrap_or_default();
    let branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|branch| branch.trim().to_string())
        .unwrap_or_default();

    let id = add(
        &mut register,
        NewQuestion {
            title,
            options,
            priority,
            branch,
            context,
        },
    )?;
    save_register(&register)?;
    Ok(tool_result(id))
}

/// `letter` and `text` are mutually exclusive: a lettered pick goes through
/// `answer` (validated against the question's own options), the user's own
/// words go through `answer_with_text` (nothing to validate against).
fn answer_tool(mut register: decisions::Register, arguments: &Value) -> Result<Value, String> {
    let question_id = string_argument(arguments, "id").ok_or("id is required")?;
    let letter = string_argument(arguments, "letter");
    let text = string_argument(arguments, "text");
    match (letter, text) {
        (Some(_), Some(_)) => Err("give letter or text, not both".to_string()),
        (None, None) => Err("either letter or text is required".to_string()),
        (Some(letter), None) => {
            answer(&mut register, &question_id, &letter)?;
            save_register(&register)?;
            Ok(tool_result(question_id))
        }
        (None, Some(text)) => {
            answer_with_text(&mut register, &question_id, &text)?;
            save_register(&register)?;
            Ok(tool_result(question_id))
        }
    }
}

fn option_set_tool(mut register: decisions::Register, arguments: &Value) -> Result<Value, String> {
    let question_id = string_argument(arguments, "id").ok_or("id is required")?;
    let letter = string_argument(arguments, "letter").ok_or("letter is required")?;
    let label = string_argument(arguments, "label").ok_or("label is required")?;
    set_option(&mut register, &question_id, &letter, &label)?;
    save_register(&register)?;
    Ok(tool_result(question_id))
}

fn option_remove_tool(
    mut register: decisions::Register,
    arguments: &Value,
) -> Result<Value, String> {
    let question_id = string_argument(arguments, "id").ok_or("id is required")?;
    let letters: Vec<String> = arguments
        .get("letters")
        .and_then(Value::as_array)
        .ok_or("letters is required")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or("letters must all be strings")
        })
        .collect::<Result<_, _>>()?;
    remove_options(&mut register, &question_id, &letters)?;
    save_register(&register)?;
    Ok(tool_result(question_id))
}

fn option_reorder_tool(
    mut register: decisions::Register,
    arguments: &Value,
) -> Result<Value, String> {
    let question_id = string_argument(arguments, "id").ok_or("id is required")?;
    let order: Vec<String> = arguments
        .get("order")
        .and_then(Value::as_array)
        .ok_or("order is required")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or("order must all be strings")
        })
        .collect::<Result<_, _>>()?;
    reorder_options(&mut register, &question_id, &order)?;
    save_register(&register)?;
    Ok(tool_result(question_id))
}

fn mutate_tool(
    mut register: decisions::Register,
    arguments: &Value,
    id_key: &str,
    text_key: &str,
    op: impl FnOnce(&mut decisions::Register, &str, &str) -> Result<(), String>,
) -> Result<Value, String> {
    let question_id = string_argument(arguments, id_key).ok_or(format!("{id_key} is required"))?;
    let text = string_argument(arguments, text_key).ok_or(format!("{text_key} is required"))?;
    op(&mut register, &question_id, &text)?;
    save_register(&register)?;
    Ok(tool_result(question_id))
}

/// `note` is optional, unlike `answer`'s letter and `stub`'s assumption, so
/// this does not go through `mutate_tool`, which requires its text key.
fn implement_tool(mut register: decisions::Register, arguments: &Value) -> Result<Value, String> {
    let question_id = string_argument(arguments, "id").ok_or("id is required")?;
    let note = string_argument(arguments, "note").unwrap_or_default();
    implement(&mut register, &question_id, &note)?;
    save_register(&register)?;
    Ok(tool_result(question_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    // `DECISIONS_JSON` is process-wide state, so two tests setting it at once
    // would race (B271-shaped: cargo test runs this module's tests on
    // separate threads by default). Every test that touches the env var holds
    // this lock for its whole body, serializing just those tests rather than
    // the whole suite.
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn scratch_register(dir: &std::path::Path) -> MutexGuard<'static, ()> {
        let guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::env::set_var("DECISIONS_JSON", dir.join("DECISIONS.json"));
        std::fs::write(
            dir.join("DECISIONS.json"),
            r#"{"skill":"decisions","skill_version":"2.0.0-alpha.9","comment":"t","questions":[]}"#,
        )
        .unwrap();
        guard
    }

    #[test]
    fn initialize_reports_the_server_name() {
        let response = handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize"}));
        assert_eq!(response["result"]["serverInfo"]["name"], "decisions");
    }

    #[test]
    fn tools_list_names_every_tool() {
        let response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}));
        let names: Vec<&str> = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        for expected in [
            "list_open",
            "list_decided",
            "list_implemented",
            "list_closed",
            "list_urgent",
            "add",
            "answer",
            "stub",
            "implement",
            "option_set",
            "option_remove",
            "option_reorder",
        ] {
            assert!(names.contains(&expected), "missing {expected}: {names:?}");
        }
    }

    #[test]
    fn add_then_list_open_then_answer_then_list_decided() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = scratch_register(dir.path());

        let add_response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"add","arguments":{"title":"Pick one","options":[{"letter":"a","label":"Yes"},{"letter":"b","label":"No"}],"priority":"urgent"}}}));
        assert!(add_response.get("error").is_none(), "{add_response}");
        let id = add_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();

        let open = handle(
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_open","arguments":{}}}),
        );
        assert!(open["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains(&id));

        let decided = handle(json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"answer","arguments":{"id": id, "letter": "a"}}}));
        assert!(decided.get("error").is_none(), "{decided}");

        let open_after = handle(
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"list_open","arguments":{}}}),
        );
        assert!(!open_after["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains(&id));

        let decided_list = handle(
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"list_decided","arguments":{}}}),
        );
        assert!(decided_list["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains(&id));
    }

    #[test]
    fn implement_requires_decided_then_moves_to_list_implemented() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = scratch_register(dir.path());

        let add_response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"add","arguments":{"title":"Pick one","options":[{"letter":"a","label":"Yes"}],"priority":"normal"}}}));
        let id = add_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();

        let too_early = handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"implement","arguments":{"id": id, "note": "too early"}}}));
        assert!(too_early.get("error").is_some(), "{too_early}");

        handle(json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"answer","arguments":{"id": id, "letter": "a"}}}));

        let implemented = handle(json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"implement","arguments":{"id": id, "note": "landed in src/thing.rs"}}}));
        assert!(implemented.get("error").is_none(), "{implemented}");

        let decided_after = handle(
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"list_decided","arguments":{}}}),
        );
        assert!(!decided_after["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains(&id));

        let implemented_list = handle(
            json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"list_implemented","arguments":{}}}),
        );
        let implemented_text = implemented_list["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        assert!(implemented_text.contains(&id));
        assert!(implemented_text.contains("landed in src/thing.rs"));
    }

    #[test]
    fn list_urgent_ignores_status() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = scratch_register(dir.path());
        handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"add","arguments":{"title":"Urgent one","options":[{"letter":"a","label":"Yes"}],"priority":"urgent"}}}));
        let urgent = handle(
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_urgent","arguments":{}}}),
        );
        assert!(urgent["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Urgent one"));
    }

    #[test]
    fn an_unknown_tool_is_refused() {
        let response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"bogus","arguments":{}}}));
        assert!(response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("bogus"));
    }

    #[test]
    fn answer_with_text_decides_on_the_users_own_words() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = scratch_register(dir.path());

        let add_response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"add","arguments":{"title":"Pick one","options":[{"letter":"a","label":"Yes"}],"priority":"normal"}}}));
        let id = add_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();

        let decided = handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"answer","arguments":{"id": id, "text": "Neither, do something else"}}}));
        assert!(decided.get("error").is_none(), "{decided}");

        let decided_list = handle(
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_decided","arguments":{}}}),
        );
        assert!(decided_list["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Neither, do something else"));
    }

    #[test]
    fn answer_with_both_letter_and_text_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = scratch_register(dir.path());

        let add_response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"add","arguments":{"title":"Pick one","options":[{"letter":"a","label":"Yes"}],"priority":"normal"}}}));
        let id = add_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();

        let response = handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"answer","arguments":{"id": id, "letter": "a", "text": "no"}}}));
        assert!(response.get("error").is_some(), "{response}");
    }

    #[test]
    fn option_set_adds_and_relabels_then_option_remove_and_reorder_apply() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = scratch_register(dir.path());

        let add_response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"add","arguments":{"title":"Pick one","options":[{"letter":"a","label":"Yes"},{"letter":"b","label":"No"}],"priority":"normal"}}}));
        let id = add_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();

        let set = handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"option_set","arguments":{"id": id, "letter": "c", "label": "Maybe"}}}));
        assert!(set.get("error").is_none(), "{set}");

        let reordered = handle(json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"option_reorder","arguments":{"id": id, "order": ["c", "a", "b"]}}}));
        assert!(reordered.get("error").is_none(), "{reordered}");

        let removed = handle(json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"option_remove","arguments":{"id": id, "letters": ["b"]}}}));
        assert!(removed.get("error").is_none(), "{removed}");

        let open = handle(
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"list_open","arguments":{}}}),
        );
        let text = open["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Maybe"));
        assert!(!text.contains("\"letter\":\"b\""));
    }

    #[test]
    fn option_remove_with_a_non_string_letter_is_refused_not_coerced_to_blank() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = scratch_register(dir.path());

        let add_response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"add","arguments":{"title":"Pick one","options":[{"letter":"a","label":"Yes"}],"priority":"normal"}}}));
        let id = add_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();

        let removed = handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"option_remove","arguments":{"id": id, "letters": [1]}}}));
        assert!(removed.get("error").is_some(), "{removed}");
    }

    #[test]
    fn option_reorder_with_a_non_string_entry_is_refused_not_coerced_to_blank() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = scratch_register(dir.path());

        let add_response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"add","arguments":{"title":"Pick one","options":[{"letter":"a","label":"Yes"}],"priority":"normal"}}}));
        let id = add_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();

        let reordered = handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"option_reorder","arguments":{"id": id, "order": [1]}}}));
        assert!(reordered.get("error").is_some(), "{reordered}");
    }

    #[test]
    fn option_set_is_refused_once_the_question_is_decided() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = scratch_register(dir.path());

        let add_response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"add","arguments":{"title":"Pick one","options":[{"letter":"a","label":"Yes"}],"priority":"normal"}}}));
        let id = add_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();

        handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"answer","arguments":{"id": id, "letter": "a"}}}));

        let response = handle(json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"option_set","arguments":{"id": id, "letter": "b", "label": "No"}}}));
        assert!(response.get("error").is_some(), "{response}");
    }
}
