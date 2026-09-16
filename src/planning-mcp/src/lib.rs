// MODE: DEV
// PACKAGE: PROD
//! MCP adapter for planning-server, mirroring ai-text-editor-mcp's own
//! thin-adapter shape: a stdio JSON-RPC surface exposing the same seven MVP
//! operations planning-client's own subcommands expose, each implemented as
//! a direct, in-process call into planning_server::handlers::dispatch --
//! no operation's own logic is reimplemented here. Calling the handler
//! in-process rather than through the socket needs no running
//! planning-server daemon at all; it is the plan-file-level revision guard,
//! not any particular process, that makes a write safe, so this is not a
//! second implementation of that guard, only a second caller of it.

use planning_server::handlers;
use planning_server::protocol::{Request, Response};
use serde_json::{json, Map, Value};

pub fn handle(message: Value) -> Value {
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2025-06-18",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "planning-server", "version": "0.1.0"}
            }
        }),
        "notifications/initialized" => Value::Null,
        "tools/list" => {
            json!({"jsonrpc": "2.0", "id": id, "result": {"tools": tool_definitions()}})
        }
        "tools/call" => call_tool(id, message.get("params").cloned().unwrap_or_default()),
        _ => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "method not found"}})
        }
    }
}

fn string(description: &str) -> Value {
    json!({"type": "string", "description": description})
}

fn boolean(description: &str) -> Value {
    json!({"type": "boolean", "description": description})
}

fn tool(name: &str, description: &str, required: &[&str], properties: Vec<(&str, Value)>) -> Value {
    let mut property_map = Map::new();
    for (key, schema) in properties {
        property_map.insert(key.to_string(), schema);
    }
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": Value::Object(property_map),
            "required": required,
        }
    })
}

const DOCUMENT_ID_HELP: &str = "plan, review, goal:<goal>, step:<goal>/<step>, unit:<WNN>, inventory, coverage, progress, goal-progress:<goal>, adversarial-review, stories, bugs, fixes, fix-keys, approval";
const REVISION_HELP: &str = "The document's current revision, from a prior read; omit to have this tool read it fresh first (one extra call, but never a stale guess).";

fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "read_plan_document",
            "Read a bounded view of a plan document.",
            &["plan_dir", "document_id"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("document_id", string(DOCUMENT_ID_HELP)),
                ("view", string("Optional view name; defaults to full.")),
            ],
        ),
        tool(
            "read_work_unit",
            "Read a work unit's own step document (sugar for read_plan_document's unit:<WNN> case).",
            &["plan_dir", "unit_id"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("unit_id", string("The work unit id, e.g. W01.")),
            ],
        ),
        tool(
            "update_step",
            "Update a step's completion status, revision-guarded.",
            &["plan_dir", "goal", "step", "status"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("goal", string("The goal directory name.")),
                ("step", string("The step name.")),
                ("status", string("incomplete, in-progress, or completed.")),
                ("revision", string(REVISION_HELP)),
            ],
        ),
        tool(
            "add_work_unit",
            "Append a new work-unit inventory row, revision-guarded.",
            &["plan_dir", "id", "type", "file", "scope", "subscope", "change", "depends_on", "goal", "step"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("id", string("The new work unit id, e.g. W05.")),
                ("type", string("source, test, verification, docs, config, data, generated, discovery, markup, or style.")),
                ("file", string("The target file, or N/A.")),
                ("scope", string("The primary symbol or file scope.")),
                ("subscope", string("The subscope, or N/A.")),
                ("change", string("The intended change.")),
                ("depends_on", string("Comma-separated dependency work-unit ids, or --.")),
                ("goal", string("The owning goal.")),
                ("step", string("The owning step.")),
                ("revision", string("work-unit-inventory.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "set_review_status",
            "Set the plan's adversarial-review status, revision-guarded.",
            &["plan_dir", "status"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("status", string("pending or approved.")),
                ("revision", string("plan-description.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "set_testing_requirement",
            "Set a goal's testing requirement, revision-guarded.",
            &["plan_dir", "goal", "required", "rationale"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("goal", string("The goal directory name.")),
                ("required", boolean("Whether this goal requires testing.")),
                ("rationale", string("Why testing is or is not meaningful for this goal.")),
                ("revision", string("The goal document's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "validate_plan",
            "Run the plan validator (read-only, unguarded).",
            &["plan_dir"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("complete", boolean("Whether to run the strict --complete gate. Defaults to false.")),
            ],
        ),
    ]
}

fn str_arg(arguments: &Value, key: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

fn opt_str_arg(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn bool_arg(arguments: &Value, key: &str) -> Result<bool, String> {
    arguments
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

fn revision_or_read(
    arguments: &Value,
    plan_dir: &str,
    document_id: &str,
) -> Result<String, String> {
    match opt_str_arg(arguments, "revision") {
        Some(revision) => Ok(revision),
        None => current_revision(plan_dir, document_id),
    }
}

fn current_revision(plan_dir: &str, document_id: &str) -> Result<String, String> {
    match handlers::dispatch(Request::ReadPlanDocument {
        plan_dir: plan_dir.to_string(),
        document_id: document_id.to_string(),
        view: Some("full".to_string()),
    }) {
        Response::Document { revision, .. } => Ok(revision),
        Response::Error { message } => Err(message),
        other => Err(format!(
            "expected a Document response while reading the current revision, got {other:?}"
        )),
    }
}

fn build_request(name: &str, arguments: &Value) -> Result<Request, String> {
    match name {
        "read_plan_document" => Ok(Request::ReadPlanDocument {
            plan_dir: str_arg(arguments, "plan_dir")?,
            document_id: str_arg(arguments, "document_id")?,
            view: opt_str_arg(arguments, "view"),
        }),
        "read_work_unit" => Ok(Request::ReadWorkUnit {
            plan_dir: str_arg(arguments, "plan_dir")?,
            unit_id: str_arg(arguments, "unit_id")?,
        }),
        "update_step" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let goal = str_arg(arguments, "goal")?;
            let step = str_arg(arguments, "step")?;
            let status = str_arg(arguments, "status")?;
            // update-step's own write target is the goal's progress.md, not
            // the step document itself (confirmed directly against the real
            // standalone binary) -- the guard must read/check that file.
            let document_id = format!("goal-progress:{goal}");
            let revision = revision_or_read(arguments, &plan_dir, &document_id)?;
            Ok(Request::UpdateStep {
                plan_dir,
                goal,
                step,
                status,
                revision,
            })
        }
        "add_work_unit" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let revision = revision_or_read(arguments, &plan_dir, "inventory")?;
            Ok(Request::AddWorkUnit {
                plan_dir,
                id: str_arg(arguments, "id")?,
                unit_type: str_arg(arguments, "type")?,
                file: str_arg(arguments, "file")?,
                scope: str_arg(arguments, "scope")?,
                subscope: str_arg(arguments, "subscope")?,
                change: str_arg(arguments, "change")?,
                depends_on: str_arg(arguments, "depends_on")?,
                goal: str_arg(arguments, "goal")?,
                step: str_arg(arguments, "step")?,
                revision,
            })
        }
        "set_review_status" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let status = str_arg(arguments, "status")?;
            let revision = revision_or_read(arguments, &plan_dir, "plan")?;
            Ok(Request::SetReviewStatus {
                plan_dir,
                status,
                revision,
            })
        }
        "set_testing_requirement" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let goal = str_arg(arguments, "goal")?;
            let required = bool_arg(arguments, "required")?;
            let rationale = str_arg(arguments, "rationale")?;
            let document_id = format!("goal:{goal}");
            let revision = revision_or_read(arguments, &plan_dir, &document_id)?;
            Ok(Request::SetTestingRequirement {
                plan_dir,
                goal,
                required,
                rationale,
                revision,
            })
        }
        "validate_plan" => Ok(Request::ValidatePlan {
            plan_dir: str_arg(arguments, "plan_dir")?,
            complete: arguments
                .get("complete")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        other => Err(format!("unknown tool: {other}")),
    }
}

fn call_tool(id: Value, params: Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let request = match build_request(name, &arguments) {
        Ok(request) => request,
        Err(message) => return tool_error(id, &message),
    };
    respond(id, handlers::dispatch(request))
}

fn respond(id: Value, response: Response) -> Value {
    match response {
        Response::Document { content, revision } => ok_result(id, format!("{content}\n\nrevision: {revision}")),
        Response::Written { revision } => ok_result(id, format!("written; new revision: {revision}")),
        Response::Validated { passed, report } => {
            let mut result = json!({"content": [{"type": "text", "text": report}]});
            if !passed {
                result["isError"] = json!(true);
            }
            json!({"jsonrpc": "2.0", "id": id, "result": result})
        }
        // A guarded tool call refused as stale is a structured MCP tool
        // error (isError:true), not a generic JSON-RPC failure -- the
        // caller's own retry logic reads the SAME message a CLI caller
        // would see, telling it to re-read and resubmit.
        Response::Stale { expected, actual } => tool_error(
            id,
            &format!(
                "stale revision: the document is at revision {actual}; this request supplied {expected} - re-read and retry with the current revision"
            ),
        ),
        Response::Error { message } => tool_error(id, &message),
        Response::Unimplemented => tool_error(id, "the server has no handler for this operation yet"),
    }
}

fn ok_result(id: Value, text: String) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": {"content": [{"type": "text", "text": text}]}})
}

fn tool_error(id: Value, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": {"isError": true, "content": [{"type": "text", "text": message}]}})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(name: &str, arguments: Value) -> Value {
        handle(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        }))
    }

    #[test]
    fn initialize_reports_a_reasonable_server_info() {
        let response = handle(json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"}));
        assert_eq!(response["result"]["serverInfo"]["name"], "planning-server");
    }

    #[test]
    fn tools_list_names_all_seven_operations() {
        let response = handle(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
        let names: Vec<&str> = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![
                "read_plan_document",
                "read_work_unit",
                "update_step",
                "add_work_unit",
                "set_review_status",
                "set_testing_requirement",
                "validate_plan",
            ]
        );
    }

    #[test]
    fn an_unknown_method_is_a_jsonrpc_error() {
        let response = handle(json!({"jsonrpc": "2.0", "id": 1, "method": "bogus"}));
        assert_eq!(response["error"]["code"], -32601);
    }

    #[test]
    fn an_unknown_tool_is_a_structured_tool_error_not_a_panic() {
        let response = call("bogus_tool", json!({}));
        assert_eq!(response["result"]["isError"], true);
    }

    #[test]
    fn a_missing_required_argument_is_a_structured_tool_error() {
        let response = call("update_step", json!({"plan_dir": "/nonexistent"}));
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn a_read_plan_document_call_against_a_missing_plan_is_a_structured_error() {
        let response = call(
            "read_plan_document",
            json!({"plan_dir": "/definitely/does/not/exist", "document_id": "plan"}),
        );
        assert_eq!(response["result"]["isError"], true);
    }
}
