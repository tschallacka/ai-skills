// MODE: DEV
// PACKAGE: PROD
//! MCP adapter for planning-server, mirroring ai-text-editor-mcp's own
//! thin-adapter shape: a stdio JSON-RPC surface exposing the operations
//! planning_server::handlers dispatches, growing past the original seven-
//! operation MVP, each implemented as a direct, in-process call into
//! planning_server::handlers::dispatch -- no operation's own logic is
//! reimplemented here. Calling the handler in-process rather than through
//! the socket needs no running planning-server daemon at all; it is the
//! plan-file-level revision guard, not any particular process, that makes a
//! write safe, so this is not a second implementation of that guard, only a
//! second caller of it.

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

fn string_array(description: &str) -> Value {
    json!({"type": "array", "items": {"type": "string"}, "description": description})
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
                ("type", string("source, test, verification, docs, config, data, generated, discovery, markup, style, or relocation.")),
                ("file", string("The target file, or N/A. relocation alone may name a directory (ending in /): the source path moved wholesale, contents unchanged.")),
                ("scope", string("The primary symbol or file scope. For relocation, the destination path instead.")),
                ("subscope", string("The subscope, or N/A.")),
                ("change", string("The intended change.")),
                ("depends_on", string("Comma-separated dependency work-unit ids, or --.")),
                ("goal", string("The owning goal.")),
                ("step", string("The owning step.")),
                ("revision", string("work-unit-inventory.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "update_work_unit",
            "Change an existing work unit's scope/file/type/depends-on/description, or move it to a different goal/step, revision-guarded on work-unit-inventory.md. A move also rewrites the unit's step file and both goals' progress trackers; only the inventory row is guarded.",
            &["plan_dir", "unit_id", "args"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("unit_id", string("The work unit id, e.g. W05.")),
                ("args", string_array("The rest of update-work-unit's own arguments verbatim, e.g. [\"--scope\", \"new scope\"] or [\"--goal\", \"02-next\", \"--step\", \"03-step\"] to move it.")),
                ("revision", string("work-unit-inventory.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "remove_work_unit",
            "Remove a work unit: its inventory row, its id from coverage rows, its goal's Owned work units entry, its step file and testing twin, then rebuilds both progress trackers. Revision-guarded on work-unit-inventory.md; other rewritten files are not separately guarded. Refuses when another unit depends on this one unless confirm_cascade is set.",
            &["plan_dir", "unit_id"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("unit_id", string("The work unit id, e.g. W05.")),
                ("confirm_cascade", boolean("Prune dependency links from other units onto this one. Defaults to false.")),
                ("revision", string("work-unit-inventory.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "update_plan_content",
            "Edit plan prose: one paragraph, a whole section, a title, a table cell, or a decomposition-review flag, revision-guarded on whichever document the mode targets.",
            &["plan_dir", "mode", "args"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("mode", string("description-paragraph, description-section, goal-paragraph, goal-section, step-paragraph, step-section, review-paragraph, review-section, append-paragraph, table-paragraph, insert-after, insert-before, delete-paragraph, title, field, or decomposition-review.")),
                ("args", string_array("The mode's own arguments verbatim, e.g. [\"1.2\", \"new text\"] for description-paragraph, or [\"goal:01-example\", \"New title\"] for title.")),
                ("revision", string("The target document's current revision; omit to read it fresh first.")),
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

fn opt_bool_arg(arguments: &Value, key: &str) -> bool {
    arguments.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// `args` on update_work_unit/update_plan_content: an array of strings, or
/// simply absent (some modes/calls need none) -- absent is `[]`, not a
/// missing-argument refusal, since every other tool's optional array would
/// otherwise need its own caller-side `[]` default.
fn str_array_arg(arguments: &Value, key: &str) -> Result<Vec<String>, String> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{key} must be an array of strings"))
            })
            .collect(),
        Some(_) => Err(format!("{key} must be an array of strings")),
    }
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
        "update_work_unit" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let unit_id = str_arg(arguments, "unit_id")?;
            let args = str_array_arg(arguments, "args")?;
            let revision = revision_or_read(arguments, &plan_dir, "inventory")?;
            Ok(Request::UpdateWorkUnit {
                plan_dir,
                unit_id,
                args,
                revision,
            })
        }
        "remove_work_unit" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let unit_id = str_arg(arguments, "unit_id")?;
            let confirm_cascade = opt_bool_arg(arguments, "confirm_cascade");
            let revision = revision_or_read(arguments, &plan_dir, "inventory")?;
            Ok(Request::RemoveWorkUnit {
                plan_dir,
                unit_id,
                confirm_cascade,
                revision,
            })
        }
        "update_plan_content" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let mode = str_arg(arguments, "mode")?;
            let args = str_array_arg(arguments, "args")?;
            let document_id = handlers::update_plan_content_document_id(&mode, &args)?;
            let revision = revision_or_read(arguments, &plan_dir, &document_id)?;
            Ok(Request::UpdatePlanContent {
                plan_dir,
                mode,
                args,
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
    fn tools_list_names_every_routed_operation() {
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
                "update_work_unit",
                "remove_work_unit",
                "update_plan_content",
                "set_review_status",
                "set_testing_requirement",
                "validate_plan",
            ]
        );
    }

    #[test]
    fn update_work_unit_reports_a_missing_argument_before_ever_reading_a_plan() {
        // unit_id is missing, so this must fail at that check, never at
        // reading a (nonexistent) plan for the revision -- same contract
        // update_step's own equivalent test pins for the existing tools.
        let response = call(
            "update_work_unit",
            json!({"plan_dir": "/definitely/does/not/exist"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn update_work_unit_accepts_an_args_array_and_defaults_it_to_empty() {
        // args is optional; when it is present it must be an array of
        // strings, and both calls fail on the (deliberately nonexistent)
        // plan_dir at the revision read, not on parsing args itself.
        for arguments in [
            json!({"plan_dir": "/definitely/does/not/exist", "unit_id": "W01"}),
            json!({"plan_dir": "/definitely/does/not/exist", "unit_id": "W01", "args": ["--scope", "x"]}),
        ] {
            let response = call("update_work_unit", arguments);
            assert_eq!(response["result"]["isError"], true);
            assert!(
                !response["result"]["content"][0]["text"]
                    .as_str()
                    .unwrap()
                    .contains("must be an array of strings"),
                "a valid or absent args must not be reported as malformed"
            );
        }
    }

    #[test]
    fn update_work_unit_refuses_a_non_array_args() {
        let response = call(
            "update_work_unit",
            json!({"plan_dir": "/x", "unit_id": "W01", "args": "not an array"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("must be an array of strings"));
    }

    #[test]
    fn remove_work_unit_reports_a_missing_argument_before_ever_reading_a_plan() {
        let response = call(
            "remove_work_unit",
            json!({"plan_dir": "/definitely/does/not/exist"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn update_plan_content_refuses_an_unknown_mode_before_ever_reading_a_plan() {
        let response = call(
            "update_plan_content",
            json!({"plan_dir": "/definitely/does/not/exist", "mode": "not-a-real-mode", "args": []}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("unknown update-plan-content mode"));
    }

    #[test]
    fn update_plan_content_reports_a_missing_positional_argument_by_name() {
        // "goal-paragraph" needs a goal name as args[0]; an empty args must
        // be refused by name rather than panicking on an out-of-bounds index.
        let response = call(
            "update_plan_content",
            json!({"plan_dir": "/definitely/does/not/exist", "mode": "goal-paragraph", "args": []}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("needs a goal name"));
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
