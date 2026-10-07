// MODE: DEV
// PACKAGE: PROD
//! MCP adapter for the decisions register: typed tool calls over the
//! decisions crate directly, no network client and no server. Every call
//! opens DECISIONS.json, does its work, and writes it back -- the same
//! round trip the CLI makes, just without a process per call.

use decisions::{
    add, answer, implement, list, migrate, stub, Filter, NewQuestion, Priority, Status,
};
use serde_json::{json, Value};

const INSTRUCTIONS: &str = "The question register (DECISIONS.json): non-blocking questions an agent \
raised mid-work, with lettered options, a priority, and the branch they came from as context. A \
question's lifecycle is open -> decided -> implemented. Use list_open/list_decided/list_implemented/ \
list_closed/list_urgent to glean questions; add to raise one and keep working with a stubbed \
assumption recorded via stub; answer records the user's pick (moves it to decided); once a decided \
pick has actually been carried out in the code, call implement to record that and move it to \
implemented -- a decided question is outstanding work for the agent, not just the user's to answer. \
This never replaces a harness's own blocking question or confirmation mechanism -- it is for the one \
that does not have to be answered right now.";

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
        json!({"name":"answer","description":"Record the user's pick for a question: sets it to decided.","inputSchema":{
            "type":"object",
            "properties":{"id":{"type":"string"},"letter":{"type":"string","description":"One of the question's own option letters."}},
            "required":["id","letter"]
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
    ]
}

fn tool_error(id: Value, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":message}})
}

fn load_register() -> Result<decisions::Register, String> {
    let path = std::env::var("DECISIONS_JSON").unwrap_or_else(|_| "DECISIONS.json".to_string());
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
    let path = std::env::var("DECISIONS_JSON").unwrap_or_else(|_| "DECISIONS.json".to_string());
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
        "answer" => mutate_tool(register, &arguments, "id", "letter", answer),
        "stub" => mutate_tool(register, &arguments, "id", "assumption", stub),
        "implement" => implement_tool(register, &arguments),
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
            r#"{"skill":"decisions","skill_version":"2.0.0-alpha.5","comment":"t","questions":[]}"#,
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
}
