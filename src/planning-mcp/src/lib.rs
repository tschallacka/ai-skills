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
            "create_adversarial_review",
            "Create the plan's adversarial-review.md. Unguarded: refuses outright if it already exists, so there is nothing to lose a concurrent write against.",
            &["plan_dir"],
            vec![("plan_dir", string("The plan directory."))],
        ),
        tool(
            "update_adversarial_review",
            "Rewrite the adversarial-review Findings table from CSV rows (ID, Missing or over-broad item, Required plan change, Status, Work unit), read from adversarial-review-incoming.md if present, else --file, else stdin. Revision-guarded on adversarial-review.md. args also accepts --set-rationale <text> as a standalone mode (no CSV/findings involved): sets the Verdict's own Rationale line, stamped with the review cycle it describes, so validate-plan can flag it once a later cycle is archived past that stamp (T56).",
            &["plan_dir"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("args", string_array("update-adversarial-review's own arguments verbatim, e.g. [\"--file\", \"rows.csv\"] or [\"--cycle\", \"2\"] or [\"--check\"] or [\"--set-rationale\", \"No unresolved findings remain.\"].")),
                ("revision", string("adversarial-review.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "add_adversarial_finding",
            "Append a finding to the adversarial review. Revision-guarded on adversarial-review.md; re-minting fix-keys.json when a work unit is named is not separately guarded.",
            &["plan_dir", "finding_id", "args"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("finding_id", string("The finding id, e.g. AR-01.")),
                ("args", string_array("The finding and resolution text, then optional flags, e.g. [\"Missing X\", \"Add X\", \"--status\", \"open\", \"--work-unit\", \"W05\"].")),
                ("revision", string("adversarial-review.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "resolve_finding",
            "Record a finding's status (and optionally who claimed it). Revision-guarded on adversarial-review.md.",
            &["plan_dir", "finding_id"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("finding_id", string("The finding id, e.g. AR-01.")),
                ("args", string_array("resolve-finding's own arguments verbatim, e.g. [\"--status\", \"resolved\", \"--claimed-by\", \"session-1\"].")),
                ("revision", string("adversarial-review.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "mint_fix_keys",
            "(Re)generate fix-keys.json from the plan's current findings. Unguarded: this fully regenerates the file every time, so a guard against its own previous bytes protects nothing a plain re-run does not already risk.",
            &["plan_dir"],
            vec![("plan_dir", string("The plan directory."))],
        ),
        tool(
            "verify_fix_keys",
            "Verify fixes.md's claims against fix-keys.json (read-only, unguarded, like validate_plan).",
            &["plan_dir"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("claimed_by", string("Optional: verify as this session id, refusing a claim made by the same session that minted the keys.")),
            ],
        ),
        tool(
            "add_fix_claim",
            "Record one fix claim in fixes.md. Unguarded: fixes.md is an append-only audit trail that may not exist yet before the first claim.",
            &["plan_dir", "finding_id", "work_unit", "key"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("finding_id", string("The adversarial-review finding the fix answers, e.g. AR-01.")),
                ("work_unit", string("The work unit that carried the fix, e.g. W05.")),
                ("key", string("The fix key minted for that pair (64 lowercase hex chars).")),
            ],
        ),
        tool(
            "add_coverage",
            "Append (or replace) a coverage row. Revision-guarded on work-unit-inventory.md (coverage rows live in the same file as work-unit rows).",
            &["plan_dir", "required_outcome", "work_units", "notes"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("required_outcome", string("The required outcome or proof text; this is the row's own key.")),
                ("work_units", string("Comma-separated work-unit ids that satisfy it, e.g. W01,W02.")),
                ("notes", string("How it is satisfied.")),
                ("replace", boolean("Replace an existing row with the same required_outcome instead of refusing. Defaults to false.")),
                ("revision", string("work-unit-inventory.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "remove_coverage",
            "Remove the coverage row whose required outcome matches exactly. Revision-guarded on work-unit-inventory.md.",
            &["plan_dir", "required_outcome"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("required_outcome", string("The required outcome or proof text to remove.")),
                ("revision", string("work-unit-inventory.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "create_work_unit_inventory",
            "Create the plan's work-unit-inventory.md. Unguarded: refuses outright if it already exists.",
            &["plan_dir"],
            vec![("plan_dir", string("The plan directory."))],
        ),
        tool(
            "create_plan_progress",
            "Create the plan's progress.md. Unguarded: refuses outright if it already exists.",
            &["plan_dir"],
            vec![("plan_dir", string("The plan directory."))],
        ),
        tool(
            "update_plan_progress",
            "Set a goal's status row in progress.md directly (rebuild_plan_progress and update_step both do this indirectly; this is the direct, single-goal form). Revision-guarded on progress.md.",
            &["plan_dir", "goal", "status"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("goal", string("The goal directory name.")),
                ("status", string("incomplete, in-progress, or completed.")),
                ("revision", string("progress.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "rebuild_plan_progress",
            "Fully regenerate progress.md from the goals' own progress files. Unguarded: this fully regenerates the file every time, so a guard against its own previous bytes protects nothing a plain re-run does not already risk.",
            &["plan_dir"],
            vec![("plan_dir", string("The plan directory."))],
        ),
        tool(
            "create_plan",
            "Scaffold a new plan directory. Unguarded: refuses outright if plan_dir already exists.",
            &["plan_dir", "title"],
            vec![
                ("plan_dir", string("The plan directory to create -- an explicit path, always (the CLI's own bare-name-under-the-plans-root shorthand is not offered here).")),
                ("title", string("The plan's title.")),
            ],
        ),
        tool(
            "remove_plan",
            "Permanently delete a whole plan directory. remove-plan itself has no confirmation flag; this tool refuses unless confirm is true, checked before anything runs.",
            &["plan_dir", "confirm"],
            vec![
                ("plan_dir", string("The plan directory to delete.")),
                ("confirm", boolean("Must be true, or the call is refused before running anything. There is no revision guard here -- confirm is the only safety check.")),
            ],
        ),
        tool(
            "cleanup_plans",
            "Bulk-remove completed plans under the whole plans root (not one plan_dir). list_only reports which plans cleanup-plans considers complete and changes nothing. The real removal mode refuses unless confirm is true, checked before anything runs, and then always passes --yes so cleanup-plans' own interactive prompt -- which would otherwise block forever with no terminal on the other end -- is never reached.",
            &[],
            vec![
                ("list_only", boolean("Report only; removes nothing. Defaults to false.")),
                ("plan_names", string_array("Specific plan names to consider, or empty for every plan under the root.")),
                ("confirm", boolean("Must be true to actually remove anything (ignored when list_only is true). Defaults to false.")),
            ],
        ),
        tool(
            "add_goal",
            "Add a new goal to a plan: its own directory (goal.md + steps/) and a row in progress.md. Revision-guarded on progress.md; the new goal directory is not separately guarded, since it does not exist yet at guard time.",
            &["plan_dir", "goal_name", "title", "outcome"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("goal_name", string("The new goal's directory name, e.g. 02-next-thing.")),
                ("title", string("The goal's title.")),
                ("outcome", string("The goal's outcome / definition of done.")),
                ("revision", string("progress.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "plan_root",
            "Resolve a project's plan-storage root directory (read-only). plan-root's OTHER subcommand, resolve, is not offered: on a project's first use it reads stdin interactively, which this adapter cannot safely forward.",
            &[],
            vec![("directory", string("The project directory to resolve from; omit for the current directory."))],
        ),
        tool(
            "register_read",
            "Query the shared bug or todo register (read-only). file is required, naming the exact BUGS.json/TODO.json to read -- there is no reliable ambient default this adapter can resolve.",
            &["kind", "mode", "file"],
            vec![
                ("kind", string("bug or todo.")),
                ("mode", string("show, list, report, count, or next-id.")),
                ("args", string_array("The mode's own trailing arguments, e.g. [\"B123\"] for show, or [\"--status\", \"open\"] for list/report/count.")),
                ("file", string("The exact register file to read, e.g. /path/to/BUGS.json.")),
            ],
        ),
        tool(
            "add_planning_bug",
            "Append a plan-scoped bug to planning-bugs.json, creating it on first use. Unguarded: the same \"nothing to guard against yet\" shape as add_fix_claim.",
            &["plan_dir", "id", "title", "reproduce", "observed", "expected"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("id", string("The plan-local bug id, PB-01 upward.")),
                ("title", string("The bug's title.")),
                ("reproduce", string("The command or steps, runnable rather than described.")),
                ("observed", string("What happened, quoted from the output.")),
                ("expected", string("What should have happened.")),
                ("args", string_array("Optional flags verbatim, e.g. [\"--severity\", \"blocking\", \"--priority\", \"urgent\", \"--status\", \"confirmed\", \"--found-by\", \"session-1\"].")),
            ],
        ),
        tool(
            "create_ui_validation",
            "Scaffold UI validation artifacts for a plan: inserts a \"## UI validation\" section into plan-description.md, and creates ui-user-stories.md and bugs.md fresh. Revision-guarded on plan-description.md. Refuses if UI validation artifacts already exist.",
            &["plan_dir", "browser_target"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("browser_target", string("The browser or discovery method the UI stories will be run against, e.g. \"chromium headless\".")),
                ("revision", string("plan-description.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "add_ui_story",
            "Append a UI user story row to ui-user-stories.md and create its per-story browser run cache under ui-story-runs/. Revision-guarded on ui-user-stories.md. actions/interaction together must name a direct user interaction (click, tap, type, keyboard, press, swipe, pinch, drag, or select).",
            &["plan_dir", "id", "persona", "actions", "interaction", "expected", "work_units"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("id", string("The new story id, e.g. US-01.")),
                ("persona", string("The persona and/or precondition.")),
                ("actions", string("The browser actions taken.")),
                ("interaction", string("The interaction evidence observed.")),
                ("expected", string("The expected observable result.")),
                ("work_units", string("Comma-separated related work-unit ids, e.g. W01,W02.")),
                ("revision", string("ui-user-stories.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "add_ui_story_links",
            "Rewrite a UI story row's own Related work units column. Revision-guarded on ui-user-stories.md; each work unit must already exist in work-unit-inventory.md.",
            &["plan_dir", "id", "work_units"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("id", string("The story id, e.g. US-01.")),
                ("work_units", string("Comma-separated work-unit ids, e.g. W01,W02.")),
                ("revision", string("ui-user-stories.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "update_ui_story",
            "Update any subset of a UI story's own fields (persona/actions/interaction/expected/status/evidence; at least one required). Revision-guarded on ui-user-stories.md. Setting status to ✅ passed, 🐛 bug found, or ⏭️ excluded requires evidence; excluded additionally requires evidence recording the user's approval. When a run result (status and/or evidence) is recorded and the row's cache column still points at the standard ui-story-runs/<id>.md path, that cache file's own Status/Evidence is mirrored too, not separately guarded.",
            &["plan_dir", "id"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("id", string("The story id, e.g. US-01.")),
                ("persona", string("New persona/precondition text.")),
                ("actions", string("New browser actions text.")),
                ("interaction", string("New interaction evidence text.")),
                ("expected", string("New expected observable result text.")),
                ("status", string("💤 untested, ⏳ in progress, ✅ passed, 🐛 bug found, or ⏭️ excluded.")),
                ("evidence", string("What was observed; required when status is a terminal status.")),
                ("revision", string("ui-user-stories.md's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "configure_ui_story_cache",
            "Fully regenerate a UI story's own browser run cache (its buffered interaction sequence and wait conditions). Revision-guarded on the cache file itself (ui-story-runs/<id>.md, which must already exist -- create it first with add_ui_story or create_ui_story_run_cache). revision is REQUIRED here, not optional: there is no read_plan_document id for this file, so the auto-read convenience other tools offer is not available -- pass the cache file's own last-reported revision (from add_ui_story's, create_ui_story_run_cache's, or this tool's own prior response).",
            &["plan_dir", "id", "starting_state", "input", "target", "readiness", "max_wait", "revision"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("id", string("The story id, e.g. US-01.")),
                ("starting_state", string("URL, persona, viewport/device, and visible initial condition.")),
                ("input", string("The single direct user interaction, e.g. \"click Sign up\".")),
                ("target", string("The target element and/or value.")),
                ("readiness", string("The expected readiness signal.")),
                ("max_wait", string("The maximum wait for that signal, e.g. \"5s\".")),
                ("revision", string("The cache file's own current revision. Required: there is no read_plan_document id for this file.")),
            ],
        ),
        tool(
            "create_ui_story_run_cache",
            "Create a fresh browser run cache for a story id. Unguarded: refuses outright if the cache file already exists. add_ui_story already creates this as a side effect; use this to recreate one that was deleted.",
            &["plan_dir", "id"],
            vec![
                ("plan_dir", string("The plan directory.")),
                ("id", string("The story id, e.g. US-01.")),
            ],
        ),
        tool(
            "read_register_file",
            "Read a register file's (TODO.json/BUGS.json) raw content and its current revision (read-only).",
            &["file"],
            vec![("file", string("The exact register file to read, e.g. /path/to/TODO.json."))],
        ),
        tool(
            "add_todo",
            "Append a new task to a TODO.json register. Revision-guarded on file directly -- file is also set as the TODO_JSON env var for the subprocess, never read from this adapter's own cwd or ambient environment.",
            &["file", "id", "title"],
            vec![
                ("file", string("The exact TODO.json to operate on.")),
                ("id", string("The new task id, e.g. T45.")),
                ("title", string("The task's title.")),
                ("parent", string("Optional parent task id.")),
                ("priority", string("urgent, high, normal, low, or someday. Defaults to normal.")),
                ("status", string("open, done, blocked, partly, decided, dropped, or obsolete. Defaults to open.")),
                ("blocked_on", string("Optional: what this task is blocked on.")),
                ("detail", string("Optional longer detail text.")),
                ("refs", string_array("Optional reference paths.")),
                ("revision", string("file's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "update_todo",
            "Update an existing task's status/priority/note/detail/blocked_on (any subset; at least one required). Revision-guarded on file directly, the same TODO_JSON-injection shape as add_todo.",
            &["file", "id"],
            vec![
                ("file", string("The exact TODO.json to operate on.")),
                ("id", string("The task id to update, e.g. T45.")),
                ("status", string("New status.")),
                ("priority", string("New priority.")),
                ("note", string("A note to set.")),
                ("detail", string("New detail text.")),
                ("blocked_on", string("New blocked_on text.")),
                ("revision", string("file's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "add_bug",
            "File a new defect in a BUGS.json register. Revision-guarded on file directly (BUGS_JSON is injected the same way add_todo injects TODO_JSON). bug-add mints its own B<N> id and takes no id argument at all -- the response reports the minted id back, since there is no other way to learn it.",
            &["file", "title", "reproduce", "observed", "expected"],
            vec![
                ("file", string("The exact BUGS.json to operate on.")),
                ("title", string("The bug's title.")),
                ("reproduce", string("The command or steps, runnable rather than described.")),
                ("observed", string("What happened, quoted from the output.")),
                ("expected", string("What should have happened.")),
                ("severity", string("blocking, major, minor, or cosmetic. Defaults to major.")),
                ("priority", string("urgent, high, normal, low, or someday. Defaults to normal.")),
                ("status", string("reported or confirmed. Defaults to reported.")),
                ("mechanism", string("Optional: why it happens, once known.")),
                ("parent", string("Optional parent bug id.")),
                ("found_by", string("Who found it.")),
                ("surfaces", string("Optional comma-separated list of files it surfaces in.")),
                ("revision", string("file's current revision; omit to read it fresh first.")),
            ],
        ),
        tool(
            "update_bug",
            "Update an existing bug's status/fix/verification/reason/priority/mechanism, or append a note (any subset; at least one required). Revision-guarded on file directly, the same BUGS_JSON-injection shape as add_bug. status fixed requires fix and verification; status wont-fix/not-a-defect/obsolete requires reason.",
            &["file", "id"],
            vec![
                ("file", string("The exact BUGS.json to operate on.")),
                ("id", string("The bug id to update, e.g. B12.")),
                ("status", string("reported, confirmed, fixed, not-a-defect, wont-fix, or obsolete.")),
                ("fix", string("What fixed it (required with status fixed).")),
                ("verification", string("How the fix was verified (required with status fixed).")),
                ("reason", string("Why it is wont-fix/not-a-defect/obsolete (required with those statuses).")),
                ("priority", string("New priority.")),
                ("mechanism", string("New mechanism text.")),
                ("append_note", string("A note to append.")),
                ("revision", string("file's current revision; omit to read it fresh first.")),
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

/// The same "omit revision, read it fresh first" convenience as
/// `revision_or_read`, but for a register file (TODO.json/BUGS.json)
/// instead of a plan document -- there is no plan-context document id for
/// these, so the read goes through ReadRegisterFile instead of
/// ReadPlanDocument.
fn register_revision_or_read(arguments: &Value, file: &str) -> Result<String, String> {
    match opt_str_arg(arguments, "revision") {
        Some(revision) => Ok(revision),
        None => match handlers::dispatch(Request::ReadRegisterFile {
            file: file.to_string(),
        }) {
            Response::Document { revision, .. } => Ok(revision),
            Response::Error { message } => Err(message),
            other => Err(format!(
                "expected a Document response while reading the current revision, got {other:?}"
            )),
        },
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
        "create_adversarial_review" => Ok(Request::CreateAdversarialReview {
            plan_dir: str_arg(arguments, "plan_dir")?,
        }),
        "update_adversarial_review" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let args = str_array_arg(arguments, "args")?;
            let revision = revision_or_read(arguments, &plan_dir, "adversarial-review")?;
            Ok(Request::UpdateAdversarialReview {
                plan_dir,
                args,
                revision,
            })
        }
        "add_adversarial_finding" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let finding_id = str_arg(arguments, "finding_id")?;
            let args = str_array_arg(arguments, "args")?;
            let revision = revision_or_read(arguments, &plan_dir, "adversarial-review")?;
            Ok(Request::AddAdversarialFinding {
                plan_dir,
                finding_id,
                args,
                revision,
            })
        }
        "resolve_finding" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let finding_id = str_arg(arguments, "finding_id")?;
            let args = str_array_arg(arguments, "args")?;
            let revision = revision_or_read(arguments, &plan_dir, "adversarial-review")?;
            Ok(Request::ResolveFinding {
                plan_dir,
                finding_id,
                args,
                revision,
            })
        }
        "mint_fix_keys" => Ok(Request::MintFixKeys {
            plan_dir: str_arg(arguments, "plan_dir")?,
        }),
        "verify_fix_keys" => Ok(Request::VerifyFixKeys {
            plan_dir: str_arg(arguments, "plan_dir")?,
            claimed_by: opt_str_arg(arguments, "claimed_by"),
        }),
        "add_fix_claim" => Ok(Request::AddFixClaim {
            plan_dir: str_arg(arguments, "plan_dir")?,
            finding_id: str_arg(arguments, "finding_id")?,
            work_unit: str_arg(arguments, "work_unit")?,
            key: str_arg(arguments, "key")?,
        }),
        "add_coverage" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let required_outcome = str_arg(arguments, "required_outcome")?;
            let work_units = str_arg(arguments, "work_units")?;
            let notes = str_arg(arguments, "notes")?;
            let replace = opt_bool_arg(arguments, "replace");
            let revision = revision_or_read(arguments, &plan_dir, "inventory")?;
            Ok(Request::AddCoverage {
                plan_dir,
                required_outcome,
                work_units,
                notes,
                replace,
                revision,
            })
        }
        "remove_coverage" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let required_outcome = str_arg(arguments, "required_outcome")?;
            let revision = revision_or_read(arguments, &plan_dir, "inventory")?;
            Ok(Request::RemoveCoverage {
                plan_dir,
                required_outcome,
                revision,
            })
        }
        "create_work_unit_inventory" => Ok(Request::CreateWorkUnitInventory {
            plan_dir: str_arg(arguments, "plan_dir")?,
        }),
        "create_plan_progress" => Ok(Request::CreatePlanProgress {
            plan_dir: str_arg(arguments, "plan_dir")?,
        }),
        "update_plan_progress" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let goal = str_arg(arguments, "goal")?;
            let status = str_arg(arguments, "status")?;
            let revision = revision_or_read(arguments, &plan_dir, "progress")?;
            Ok(Request::UpdatePlanProgress {
                plan_dir,
                goal,
                status,
                revision,
            })
        }
        "rebuild_plan_progress" => Ok(Request::RebuildPlanProgress {
            plan_dir: str_arg(arguments, "plan_dir")?,
        }),
        "create_plan" => Ok(Request::CreatePlan {
            plan_dir: str_arg(arguments, "plan_dir")?,
            title: str_arg(arguments, "title")?,
        }),
        "remove_plan" => Ok(Request::RemovePlan {
            plan_dir: str_arg(arguments, "plan_dir")?,
            confirm: opt_bool_arg(arguments, "confirm"),
        }),
        "cleanup_plans" => Ok(Request::CleanupPlans {
            list_only: opt_bool_arg(arguments, "list_only"),
            plan_names: str_array_arg(arguments, "plan_names")?,
            confirm: opt_bool_arg(arguments, "confirm"),
        }),
        "add_goal" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let goal_name = str_arg(arguments, "goal_name")?;
            let title = str_arg(arguments, "title")?;
            let outcome = str_arg(arguments, "outcome")?;
            let revision = revision_or_read(arguments, &plan_dir, "progress")?;
            Ok(Request::AddGoal {
                plan_dir,
                goal_name,
                title,
                outcome,
                revision,
            })
        }
        "plan_root" => Ok(Request::PlanRoot {
            directory: opt_str_arg(arguments, "directory"),
        }),
        "register_read" => Ok(Request::RegisterRead {
            kind: str_arg(arguments, "kind")?,
            mode: str_arg(arguments, "mode")?,
            args: str_array_arg(arguments, "args")?,
            file: str_arg(arguments, "file")?,
        }),
        "add_planning_bug" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let id = str_arg(arguments, "id")?;
            let title = str_arg(arguments, "title")?;
            let reproduce = str_arg(arguments, "reproduce")?;
            let observed = str_arg(arguments, "observed")?;
            let expected = str_arg(arguments, "expected")?;
            let args = str_array_arg(arguments, "args")?;
            Ok(Request::AddPlanningBug {
                plan_dir,
                id,
                title,
                reproduce,
                observed,
                expected,
                args,
            })
        }
        "create_ui_validation" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let browser_target = str_arg(arguments, "browser_target")?;
            let revision = revision_or_read(arguments, &plan_dir, "plan")?;
            Ok(Request::CreateUiValidation {
                plan_dir,
                browser_target,
                revision,
            })
        }
        "add_ui_story" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let id = str_arg(arguments, "id")?;
            let persona = str_arg(arguments, "persona")?;
            let actions = str_arg(arguments, "actions")?;
            let interaction = str_arg(arguments, "interaction")?;
            let expected = str_arg(arguments, "expected")?;
            let work_units = str_arg(arguments, "work_units")?;
            let revision = revision_or_read(arguments, &plan_dir, "stories")?;
            Ok(Request::AddUiStory {
                plan_dir,
                id,
                persona,
                actions,
                interaction,
                expected,
                work_units,
                revision,
            })
        }
        "add_ui_story_links" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let id = str_arg(arguments, "id")?;
            let work_units = str_arg(arguments, "work_units")?;
            let revision = revision_or_read(arguments, &plan_dir, "stories")?;
            Ok(Request::AddUiStoryLinks {
                plan_dir,
                id,
                work_units,
                revision,
            })
        }
        "update_ui_story" => {
            let plan_dir = str_arg(arguments, "plan_dir")?;
            let id = str_arg(arguments, "id")?;
            let persona = opt_str_arg(arguments, "persona");
            let actions = opt_str_arg(arguments, "actions");
            let interaction = opt_str_arg(arguments, "interaction");
            let expected = opt_str_arg(arguments, "expected");
            let status = opt_str_arg(arguments, "status");
            let evidence = opt_str_arg(arguments, "evidence");
            let revision = revision_or_read(arguments, &plan_dir, "stories")?;
            Ok(Request::UpdateUiStory {
                plan_dir,
                id,
                persona,
                actions,
                interaction,
                expected,
                status,
                evidence,
                revision,
            })
        }
        "configure_ui_story_cache" => Ok(Request::ConfigureUiStoryCache {
            plan_dir: str_arg(arguments, "plan_dir")?,
            id: str_arg(arguments, "id")?,
            starting_state: str_arg(arguments, "starting_state")?,
            input: str_arg(arguments, "input")?,
            target: str_arg(arguments, "target")?,
            readiness: str_arg(arguments, "readiness")?,
            max_wait: str_arg(arguments, "max_wait")?,
            revision: str_arg(arguments, "revision")?,
        }),
        "create_ui_story_run_cache" => Ok(Request::CreateUiStoryRunCache {
            plan_dir: str_arg(arguments, "plan_dir")?,
            id: str_arg(arguments, "id")?,
        }),
        "read_register_file" => Ok(Request::ReadRegisterFile {
            file: str_arg(arguments, "file")?,
        }),
        "add_todo" => {
            let file = str_arg(arguments, "file")?;
            let id = str_arg(arguments, "id")?;
            let title = str_arg(arguments, "title")?;
            let parent = opt_str_arg(arguments, "parent");
            let priority = opt_str_arg(arguments, "priority");
            let status = opt_str_arg(arguments, "status");
            let blocked_on = opt_str_arg(arguments, "blocked_on");
            let detail = opt_str_arg(arguments, "detail");
            let refs = str_array_arg(arguments, "refs")?;
            let revision = register_revision_or_read(arguments, &file)?;
            Ok(Request::AddTodo {
                file,
                id,
                title,
                parent,
                priority,
                status,
                blocked_on,
                detail,
                refs,
                revision,
            })
        }
        "update_todo" => {
            let file = str_arg(arguments, "file")?;
            let id = str_arg(arguments, "id")?;
            let status = opt_str_arg(arguments, "status");
            let priority = opt_str_arg(arguments, "priority");
            let note = opt_str_arg(arguments, "note");
            let detail = opt_str_arg(arguments, "detail");
            let blocked_on = opt_str_arg(arguments, "blocked_on");
            let revision = register_revision_or_read(arguments, &file)?;
            Ok(Request::UpdateTodo {
                file,
                id,
                status,
                priority,
                note,
                detail,
                blocked_on,
                revision,
            })
        }
        "add_bug" => {
            let file = str_arg(arguments, "file")?;
            let title = str_arg(arguments, "title")?;
            let reproduce = str_arg(arguments, "reproduce")?;
            let observed = str_arg(arguments, "observed")?;
            let expected = str_arg(arguments, "expected")?;
            let severity = opt_str_arg(arguments, "severity");
            let priority = opt_str_arg(arguments, "priority");
            let status = opt_str_arg(arguments, "status");
            let mechanism = opt_str_arg(arguments, "mechanism");
            let parent = opt_str_arg(arguments, "parent");
            let found_by = opt_str_arg(arguments, "found_by");
            let surfaces = opt_str_arg(arguments, "surfaces");
            let revision = register_revision_or_read(arguments, &file)?;
            Ok(Request::AddBug {
                file,
                title,
                reproduce,
                observed,
                expected,
                severity,
                priority,
                status,
                mechanism,
                parent,
                found_by,
                surfaces,
                revision,
            })
        }
        "update_bug" => {
            let file = str_arg(arguments, "file")?;
            let id = str_arg(arguments, "id")?;
            let status = opt_str_arg(arguments, "status");
            let fix = opt_str_arg(arguments, "fix");
            let verification = opt_str_arg(arguments, "verification");
            let reason = opt_str_arg(arguments, "reason");
            let priority = opt_str_arg(arguments, "priority");
            let mechanism = opt_str_arg(arguments, "mechanism");
            let append_note = opt_str_arg(arguments, "append_note");
            let revision = register_revision_or_read(arguments, &file)?;
            Ok(Request::UpdateBug {
                file,
                id,
                status,
                fix,
                verification,
                reason,
                priority,
                mechanism,
                append_note,
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
        // `id` here is the destructured minted register id (e.g. bug-add's
        // own B12), not the JSON-RPC message id -- given the same name as
        // that outer `id: Value` parameter, so it is bound under its own
        // name to avoid shadowing the one `ok_result` still needs.
        Response::WrittenWithId {
            revision,
            id: minted_id,
        } => ok_result(
            id,
            format!("written; new id: {minted_id}; new revision: {revision}"),
        ),
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
                "create_adversarial_review",
                "update_adversarial_review",
                "add_adversarial_finding",
                "resolve_finding",
                "mint_fix_keys",
                "verify_fix_keys",
                "add_fix_claim",
                "add_coverage",
                "remove_coverage",
                "create_work_unit_inventory",
                "create_plan_progress",
                "update_plan_progress",
                "rebuild_plan_progress",
                "create_plan",
                "remove_plan",
                "cleanup_plans",
                "add_goal",
                "plan_root",
                "register_read",
                "add_planning_bug",
                "create_ui_validation",
                "add_ui_story",
                "add_ui_story_links",
                "update_ui_story",
                "configure_ui_story_cache",
                "create_ui_story_run_cache",
                "read_register_file",
                "add_todo",
                "update_todo",
                "add_bug",
                "update_bug",
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
    fn create_adversarial_review_reports_a_missing_argument_by_name() {
        let response = call("create_adversarial_review", json!({}));
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn add_adversarial_finding_reports_a_missing_argument_before_ever_reading_a_plan() {
        // finding_id is missing, so this must fail there, never at reading
        // a (nonexistent) plan for the revision.
        let response = call(
            "add_adversarial_finding",
            json!({"plan_dir": "/definitely/does/not/exist"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn add_fix_claim_reports_each_missing_argument_by_name() {
        for (present, missing) in [
            (json!({"plan_dir": "/x"}), "finding_id"),
            (
                json!({"plan_dir": "/x", "finding_id": "AR-01"}),
                "work_unit",
            ),
            (
                json!({"plan_dir": "/x", "finding_id": "AR-01", "work_unit": "W01"}),
                "key",
            ),
        ] {
            let response = call("add_fix_claim", present);
            assert_eq!(response["result"]["isError"], true);
            let text = response["result"]["content"][0]["text"].as_str().unwrap();
            assert!(
                text.contains(missing),
                "expected the refusal to name {missing}: {text}"
            );
        }
    }

    #[test]
    fn add_coverage_reports_each_missing_argument_by_name() {
        for (present, missing) in [
            (json!({"plan_dir": "/x"}), "required_outcome"),
            (
                json!({"plan_dir": "/x", "required_outcome": "It works"}),
                "work_units",
            ),
            (
                json!({"plan_dir": "/x", "required_outcome": "It works", "work_units": "W01"}),
                "notes",
            ),
        ] {
            let response = call("add_coverage", present);
            assert_eq!(response["result"]["isError"], true);
            let text = response["result"]["content"][0]["text"].as_str().unwrap();
            assert!(
                text.contains(missing),
                "expected the refusal to name {missing}: {text}"
            );
        }
    }

    #[test]
    fn create_work_unit_inventory_reports_a_missing_argument_by_name() {
        let response = call("create_work_unit_inventory", json!({}));
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn update_plan_progress_reports_a_missing_argument_before_ever_reading_a_plan() {
        let response = call(
            "update_plan_progress",
            json!({"plan_dir": "/definitely/does/not/exist"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn remove_plan_refuses_without_confirm_and_never_touches_disk() {
        let response = call(
            "remove_plan",
            json!({"plan_dir": "/definitely/does/not/exist"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("confirm"));
    }

    #[test]
    fn cleanup_plans_refuses_without_confirm_unless_list_only() {
        let response = call("cleanup_plans", json!({}));
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("confirm"));
    }

    #[test]
    fn create_plan_reports_a_missing_argument_by_name() {
        let response = call("create_plan", json!({"plan_dir": "/x"}));
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn add_goal_reports_a_missing_argument_before_ever_reading_a_plan() {
        let response = call(
            "add_goal",
            json!({"plan_dir": "/definitely/does/not/exist"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn register_read_reports_each_missing_argument_by_name() {
        for (present, missing) in [
            (json!({}), "kind"),
            (json!({"kind": "bug"}), "mode"),
            (json!({"kind": "bug", "mode": "show"}), "file"),
        ] {
            let response = call("register_read", present);
            assert_eq!(response["result"]["isError"], true);
            let text = response["result"]["content"][0]["text"].as_str().unwrap();
            assert!(
                text.contains(missing),
                "expected the refusal to name {missing}: {text}"
            );
        }
    }

    #[test]
    fn add_planning_bug_reports_a_missing_argument_by_name() {
        let response = call("add_planning_bug", json!({"plan_dir": "/x"}));
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn create_ui_validation_reports_a_missing_argument_before_ever_reading_a_plan() {
        // browser_target is missing, so this must fail there, never at
        // reading a (nonexistent) plan for the revision.
        let response = call(
            "create_ui_validation",
            json!({"plan_dir": "/definitely/does/not/exist"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn add_ui_story_reports_a_missing_argument_before_ever_reading_a_plan() {
        let response = call(
            "add_ui_story",
            json!({"plan_dir": "/definitely/does/not/exist"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn add_ui_story_links_reports_a_missing_argument_before_ever_reading_a_plan() {
        let response = call(
            "add_ui_story_links",
            json!({"plan_dir": "/definitely/does/not/exist"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn update_ui_story_reports_a_missing_argument_before_ever_reading_a_plan() {
        // id is missing, so this must fail there, never at reading a
        // (nonexistent) plan for the revision. Every field beyond plan_dir
        // and id is optional (at least one is required, but that is
        // update-ui-story's own refusal once it actually runs, not this
        // adapter's).
        let response = call(
            "update_ui_story",
            json!({"plan_dir": "/definitely/does/not/exist"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn configure_ui_story_cache_reports_each_missing_argument_by_name() {
        // Unlike every other guarded tool, revision itself is required here
        // (there is no read_plan_document id for a per-story run cache), so
        // it is included in this cumulative check like every other field.
        for (present, missing) in [
            (json!({}), "plan_dir"),
            (json!({"plan_dir": "/x"}), "id"),
            (json!({"plan_dir": "/x", "id": "US-01"}), "starting_state"),
            (
                json!({"plan_dir": "/x", "id": "US-01", "starting_state": "logged out"}),
                "input",
            ),
            (
                json!({"plan_dir": "/x", "id": "US-01", "starting_state": "logged out", "input": "click Sign up"}),
                "target",
            ),
            (
                json!({"plan_dir": "/x", "id": "US-01", "starting_state": "logged out", "input": "click Sign up", "target": "#signup-button"}),
                "readiness",
            ),
            (
                json!({"plan_dir": "/x", "id": "US-01", "starting_state": "logged out", "input": "click Sign up", "target": "#signup-button", "readiness": "the form is visible"}),
                "max_wait",
            ),
            (
                json!({"plan_dir": "/x", "id": "US-01", "starting_state": "logged out", "input": "click Sign up", "target": "#signup-button", "readiness": "the form is visible", "max_wait": "5s"}),
                "revision",
            ),
        ] {
            let response = call("configure_ui_story_cache", present);
            assert_eq!(response["result"]["isError"], true);
            let text = response["result"]["content"][0]["text"].as_str().unwrap();
            assert!(
                text.contains(missing),
                "expected the refusal to name {missing}: {text}"
            );
        }
    }

    #[test]
    fn create_ui_story_run_cache_reports_a_missing_argument_by_name() {
        let response = call("create_ui_story_run_cache", json!({"plan_dir": "/x"}));
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn read_register_file_reports_a_missing_argument_by_name() {
        let response = call("read_register_file", json!({}));
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn add_todo_reports_a_missing_argument_before_ever_reading_a_register() {
        // title is missing, so this must fail there, never at reading a
        // (nonexistent) register file for the revision.
        let response = call(
            "add_todo",
            json!({"file": "/definitely/does/not/exist/TODO.json", "id": "T45"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn update_todo_reports_a_missing_argument_before_ever_reading_a_register() {
        let response = call(
            "update_todo",
            json!({"file": "/definitely/does/not/exist/TODO.json"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
    }

    #[test]
    fn add_bug_reports_each_missing_argument_by_name() {
        for (present, missing) in [
            (json!({"file": "/x"}), "title"),
            (json!({"file": "/x", "title": "t"}), "reproduce"),
            (
                json!({"file": "/x", "title": "t", "reproduce": "r"}),
                "observed",
            ),
            (
                json!({"file": "/x", "title": "t", "reproduce": "r", "observed": "o"}),
                "expected",
            ),
        ] {
            let response = call("add_bug", present);
            assert_eq!(response["result"]["isError"], true);
            let text = response["result"]["content"][0]["text"].as_str().unwrap();
            assert!(
                text.contains(missing),
                "expected the refusal to name {missing}: {text}"
            );
        }
    }

    #[test]
    fn update_bug_reports_a_missing_argument_before_ever_reading_a_register() {
        let response = call(
            "update_bug",
            json!({"file": "/definitely/does/not/exist/BUGS.json"}),
        );
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required argument"));
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
