// MODE: DEV
// PACKAGE: PROD
//! The request handlers, growing past the original seven-operation MVP.
//! The two reads call directly into plan-context-core's own bounded-read
//! machinery (the same implementation plan-context.sh is already wired
//! onto). Every write DELEGATES to the existing standalone commands
//! (update-step, add-work-unit, update-work-unit, remove-work-unit,
//! update-plan-content) as real subprocesses rather than reimplementing
//! their own logic -- all `main.rs`-only binaries with no `[lib]` target, so
//! there is no library to call instead, and delegation guarantees
//! byte-identical output by construction rather than by separately-verified
//! parity. The guard gates whether the subprocess runs at all: a stale
//! guard means the subprocess is never invoked. ValidatePlan is unguarded
//! and read-only.
//!
//! Known MVP simplification, recorded rather than hidden: add-work-unit
//! touches TWO files (it inserts a row into work-unit-inventory.md and
//! creates a brand-new step .md file); only the inventory row is guarded
//! here, since the new step file does not exist yet at guard time and a
//! creation has nothing to check a hash against. update-work-unit and
//! remove-work-unit inherit the same simplification -- a move or a cascade
//! removal rewrites the unit's step file, coverage rows, both goals'
//! rosters and both progress trackers, and only the inventory row's own
//! guard is checked. update-step, in fact, writes only ONE file --
//! confirmed directly against the real standalone binary: it rewrites the
//! goal's own progress.md status row and leaves the step document itself
//! completely untouched, so progress.md, not the step file, is what
//! update-step's own guard must check (an earlier version of this handler
//! guarded the step file instead, which never changed, so the guard never
//! actually detected anything -- every call, racing or sequential, correct
//! revision or stale, silently passed). Closing the add-work-unit gap
//! (covering its new step file too) is exactly the "migrate every other
//! command onto this mechanism" follow-on this goal's own scope section
//! defers.

use crate::protocol::{Request, Response};
use crate::revision::{guarded_call, read_with_revision, PlanRevision, RevisionError};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn dispatch(request: Request) -> Response {
    dispatch_with_bin_dir(request, None)
}

/// Same as `dispatch`, but resolves each delegated command under `bin_dir`
/// instead of bare-name-on-PATH when given -- how tests point this crate at
/// the workspace's own freshly-built sibling binaries without mutating the
/// process environment (`std::env::set_var` is unsafe as of this
/// toolchain's own std, and PATH is process-global and therefore unsafe to
/// mutate from a test that may run alongside others on the same thread
/// pool).
pub fn dispatch_with_bin_dir(request: Request, bin_dir: Option<&Path>) -> Response {
    match request {
        Request::ReadPlanDocument {
            plan_dir,
            document_id,
            view,
        } => read_plan_document(&plan_dir, &document_id, view.as_deref()),
        Request::ReadWorkUnit { plan_dir, unit_id } => {
            read_plan_document(&plan_dir, &format!("unit:{unit_id}"), None)
        }
        Request::UpdateStep {
            plan_dir,
            goal,
            step,
            status,
            revision,
        } => update_step(bin_dir, &plan_dir, &goal, &step, &status, &revision),
        Request::AddWorkUnit {
            plan_dir,
            id,
            unit_type,
            file,
            scope,
            subscope,
            change,
            depends_on,
            goal,
            step,
            revision,
        } => add_work_unit(
            bin_dir,
            &plan_dir,
            &id,
            &unit_type,
            &file,
            &scope,
            &subscope,
            &change,
            &depends_on,
            &goal,
            &step,
            &revision,
        ),
        Request::UpdateWorkUnit {
            plan_dir,
            unit_id,
            args,
            revision,
        } => update_work_unit(bin_dir, &plan_dir, &unit_id, &args, &revision),
        Request::RemoveWorkUnit {
            plan_dir,
            unit_id,
            confirm_cascade,
            revision,
        } => remove_work_unit(bin_dir, &plan_dir, &unit_id, confirm_cascade, &revision),
        Request::UpdatePlanContent {
            plan_dir,
            mode,
            args,
            revision,
        } => update_plan_content(bin_dir, &plan_dir, &mode, &args, &revision),
        Request::SetReviewStatus {
            plan_dir,
            status,
            revision,
        } => set_review_status(bin_dir, &plan_dir, &status, &revision),
        Request::SetTestingRequirement {
            plan_dir,
            goal,
            required,
            rationale,
            revision,
        } => set_testing_requirement(bin_dir, &plan_dir, &goal, required, &rationale, &revision),
        Request::CreateAdversarialReview { plan_dir } => {
            create_adversarial_review(bin_dir, &plan_dir)
        }
        Request::UpdateAdversarialReview {
            plan_dir,
            args,
            revision,
        } => update_adversarial_review(bin_dir, &plan_dir, &args, &revision),
        Request::AddAdversarialFinding {
            plan_dir,
            finding_id,
            args,
            revision,
        } => add_adversarial_finding(bin_dir, &plan_dir, &finding_id, &args, &revision),
        Request::ResolveFinding {
            plan_dir,
            finding_id,
            args,
            revision,
        } => resolve_finding(bin_dir, &plan_dir, &finding_id, &args, &revision),
        Request::MintFixKeys { plan_dir } => mint_fix_keys(bin_dir, &plan_dir),
        Request::VerifyFixKeys {
            plan_dir,
            claimed_by,
        } => verify_fix_keys(bin_dir, &plan_dir, claimed_by.as_deref()),
        Request::AddFixClaim {
            plan_dir,
            finding_id,
            work_unit,
            key,
        } => add_fix_claim(bin_dir, &plan_dir, &finding_id, &work_unit, &key),
        Request::AddCoverage {
            plan_dir,
            required_outcome,
            work_units,
            notes,
            replace,
            revision,
        } => add_coverage(
            bin_dir,
            &plan_dir,
            &required_outcome,
            &work_units,
            &notes,
            replace,
            &revision,
        ),
        Request::RemoveCoverage {
            plan_dir,
            required_outcome,
            revision,
        } => remove_coverage(bin_dir, &plan_dir, &required_outcome, &revision),
        Request::CreateWorkUnitInventory { plan_dir } => {
            create_work_unit_inventory(bin_dir, &plan_dir)
        }
        Request::CreatePlanProgress { plan_dir } => create_plan_progress(bin_dir, &plan_dir),
        Request::UpdatePlanProgress {
            plan_dir,
            goal,
            status,
            revision,
        } => update_plan_progress(bin_dir, &plan_dir, &goal, &status, &revision),
        Request::RebuildPlanProgress { plan_dir } => rebuild_plan_progress(bin_dir, &plan_dir),
        Request::CreatePlan { plan_dir, title } => create_plan(bin_dir, &plan_dir, &title),
        Request::RemovePlan { plan_dir, confirm } => remove_plan(bin_dir, &plan_dir, confirm),
        Request::CleanupPlans {
            list_only,
            plan_names,
            confirm,
        } => cleanup_plans(bin_dir, list_only, &plan_names, confirm),
        Request::AddGoal {
            plan_dir,
            goal_name,
            title,
            outcome,
            revision,
        } => add_goal(bin_dir, &plan_dir, &goal_name, &title, &outcome, &revision),
        Request::PlanRoot { directory } => plan_root(bin_dir, directory.as_deref()),
        Request::RegisterRead {
            kind,
            mode,
            args,
            file,
        } => register_read(bin_dir, &kind, &mode, &args, &file),
        Request::AddPlanningBug {
            plan_dir,
            id,
            title,
            reproduce,
            observed,
            expected,
            args,
        } => add_planning_bug(
            bin_dir, &plan_dir, &id, &title, &reproduce, &observed, &expected, &args,
        ),
        Request::ValidatePlan { plan_dir, complete } => validate_plan(bin_dir, &plan_dir, complete),
    }
}

fn program_path(bin_dir: Option<&Path>, name: &str) -> PathBuf {
    match bin_dir {
        Some(dir) => dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
        None => PathBuf::from(name),
    }
}

fn run_command(bin_dir: Option<&Path>, name: &str, args: &[&str]) -> Result<(), String> {
    let program = program_path(bin_dir, name);
    let output = Command::new(&program)
        .args(args)
        .output()
        .map_err(|error| format!("could not run {}: {error}", program.display()))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

fn parse_guard(revision_hex: &str) -> Result<PlanRevision, Response> {
    PlanRevision::from_hex(revision_hex).map_err(|message| Response::Error { message })
}

fn respond_from(result: Result<PlanRevision, RevisionError>) -> Response {
    match result {
        Ok(revision) => Response::Written {
            revision: revision.to_hex(),
        },
        Err(RevisionError::Stale { expected, actual }) => Response::Stale { expected, actual },
        Err(RevisionError::Io(message)) => Response::Error { message },
    }
}

fn read_plan_document(plan_dir: &str, document_id: &str, view: Option<&str>) -> Response {
    let plan = Path::new(plan_dir);
    let path = match plan_context_core::resolve_document(plan, document_id) {
        Ok(path) => path,
        Err(message) => return Response::Error { message },
    };
    let (_bytes, revision) = match read_with_revision(&path) {
        Ok(pair) => pair,
        Err(error) => {
            return Response::Error {
                message: error.to_string(),
            }
        }
    };
    let row_text = document_id
        .strip_prefix("unit:")
        .and_then(|unit| plan_context_core::inventory_row_text(plan, unit).ok());
    let view = view.unwrap_or("full");
    match plan_context_core::view_text(&path, view, row_text.as_deref()) {
        Ok(content) => Response::Document {
            content,
            revision: revision.to_hex(),
        },
        Err(message) => Response::Error { message },
    }
}

#[allow(clippy::too_many_arguments)]
fn update_step(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    goal: &str,
    step: &str,
    status: &str,
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let goal_dir = Path::new(plan_dir).join(goal);
    // update-step's own write target is the GOAL's progress.md, not the
    // step's own .md file -- confirmed directly: running the standalone
    // update-step binary against a fresh step leaves the step document
    // byte-for-byte unchanged and only rewrites progress.md's status row.
    // Guarding the step file (as first written here) meant the guard's own
    // hash never changed regardless of how many updates ran, so every call
    // -- racing or sequential, correct revision or stale -- always passed.
    let progress_path = goal_dir.join("progress.md");
    let goal_dir_str = goal_dir.to_string_lossy().into_owned();
    respond_from(guarded_call(&progress_path, guard, || {
        run_command(bin_dir, "update-step", &[&goal_dir_str, step, status])
    }))
}

#[allow(clippy::too_many_arguments)]
fn add_work_unit(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    id: &str,
    unit_type: &str,
    file: &str,
    scope: &str,
    subscope: &str,
    change: &str,
    depends_on: &str,
    goal: &str,
    step: &str,
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let inventory_path = Path::new(plan_dir).join("work-unit-inventory.md");
    respond_from(guarded_call(&inventory_path, guard, || {
        run_command(
            bin_dir,
            "add-work-unit",
            &[
                plan_dir,
                "--id",
                id,
                "--type",
                unit_type,
                "--file",
                file,
                "--scope",
                scope,
                "--subscope",
                subscope,
                "--change",
                change,
                "--depends-on",
                depends_on,
                "--goal",
                goal,
                "--step",
                step,
            ],
        )
    }))
}

fn update_work_unit(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    unit_id: &str,
    args: &[String],
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let inventory_path = Path::new(plan_dir).join("work-unit-inventory.md");
    let mut full_args: Vec<&str> = vec![plan_dir, unit_id];
    full_args.extend(args.iter().map(String::as_str));
    respond_from(guarded_call(&inventory_path, guard, || {
        run_command(bin_dir, "update-work-unit", &full_args)
    }))
}

fn remove_work_unit(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    unit_id: &str,
    confirm_cascade: bool,
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let inventory_path = Path::new(plan_dir).join("work-unit-inventory.md");
    let mut full_args: Vec<&str> = vec![plan_dir, unit_id];
    if confirm_cascade {
        full_args.push("--confirm-cascade");
    }
    respond_from(guarded_call(&inventory_path, guard, || {
        run_command(bin_dir, "remove-work-unit", &full_args)
    }))
}

/// The document id a given update-plan-content `mode` call targets, from
/// `mode` and its own leading arguments. Some modes take an explicit
/// document id as their first argument (append-paragraph, table-paragraph,
/// insert-after, insert-before, delete-paragraph, title, field); others
/// imply one from the flag itself (description-* implies "plan", goal-*
/// implies "goal:<args[0]>", step-* implies "step:<args[0]>", review-*
/// implies "adversarial-review", decomposition-review implies "plan") --
/// mirroring update-plan-content's own `document_path` dispatch exactly
/// (src/update-plan-content/src/main.rs). Public so planning-mcp's own
/// revision-auto-read convenience (reading the current revision when a
/// caller omits one) can name the same document this handler will guard,
/// without a second copy of this table.
pub fn update_plan_content_document_id(mode: &str, args: &[String]) -> Result<String, String> {
    match mode {
        "description-paragraph" | "description-section" => Ok("plan".to_string()),
        "goal-paragraph" | "goal-section" => {
            let goal = args
                .first()
                .ok_or_else(|| format!("{mode} needs a goal name"))?;
            Ok(format!("goal:{goal}"))
        }
        "step-paragraph" | "step-section" => {
            let step = args
                .first()
                .ok_or_else(|| format!("{mode} needs a goal/step"))?;
            Ok(format!("step:{step}"))
        }
        "review-paragraph" | "review-section" => Ok("adversarial-review".to_string()),
        "decomposition-review" => Ok("plan".to_string()),
        "append-paragraph" | "table-paragraph" | "insert-after" | "insert-before"
        | "delete-paragraph" | "title" | "field" => args
            .first()
            .cloned()
            .ok_or_else(|| format!("{mode} needs a document id")),
        other => Err(format!("unknown update-plan-content mode: {other}")),
    }
}

/// Resolves the file a given update-plan-content `mode` call will write to,
/// reusing the same plan_context_core::resolve_document this crate's own
/// read path already calls rather than a second copy of that id-to-path
/// table.
fn update_plan_content_target(
    plan_dir: &str,
    mode: &str,
    args: &[String],
) -> Result<PathBuf, String> {
    let document_id = update_plan_content_document_id(mode, args)?;
    plan_context_core::resolve_document(Path::new(plan_dir), &document_id)
}

fn update_plan_content(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    mode: &str,
    args: &[String],
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let target = match update_plan_content_target(plan_dir, mode, args) {
        Ok(path) => path,
        Err(message) => return Response::Error { message },
    };
    let flag = format!("--{mode}");
    let mut full_args: Vec<&str> = vec![flag.as_str(), plan_dir];
    full_args.extend(args.iter().map(String::as_str));
    respond_from(guarded_call(&target, guard, || {
        run_command(bin_dir, "update-plan-content", &full_args)
    }))
}

fn set_review_status(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    status: &str,
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let plan_description = Path::new(plan_dir).join("plan-description.md");
    respond_from(guarded_call(&plan_description, guard, || {
        run_command(
            bin_dir,
            "update-plan-content",
            &["--review-status", plan_dir, status],
        )
    }))
}

fn set_testing_requirement(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    goal: &str,
    required: bool,
    rationale: &str,
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let goal_path = Path::new(plan_dir).join(goal).join("goal.md");
    let required_text = if required { "yes" } else { "no" };
    respond_from(guarded_call(&goal_path, guard, || {
        run_command(
            bin_dir,
            "update-plan-content",
            &[
                "--testing-requirement",
                plan_dir,
                goal,
                required_text,
                rationale,
            ],
        )
    }))
}

/// Runs `name`, then reports the new revision of `target` on success --
/// the shared shape `create_adversarial_review`/`mint_fix_keys`/
/// `add_fix_claim` all need for their own unguarded creates/regenerations:
/// no guard to check beforehand, but still a revision worth handing back
/// so a caller's very next guarded call on the same file has one.
fn run_then_report_revision(
    bin_dir: Option<&Path>,
    name: &str,
    args: &[&str],
    target: &Path,
) -> Response {
    match run_command(bin_dir, name, args) {
        Ok(()) => match read_with_revision(target) {
            Ok((_, revision)) => Response::Written {
                revision: revision.to_hex(),
            },
            Err(error) => Response::Error {
                message: error.to_string(),
            },
        },
        Err(message) => Response::Error { message },
    }
}

fn create_adversarial_review(bin_dir: Option<&Path>, plan_dir: &str) -> Response {
    let review_path = Path::new(plan_dir).join("adversarial-review.md");
    run_then_report_revision(
        bin_dir,
        "create-adversarial-review",
        &[plan_dir],
        &review_path,
    )
}

fn update_adversarial_review(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    args: &[String],
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let review_path = Path::new(plan_dir).join("adversarial-review.md");
    let mut full_args: Vec<&str> = vec![plan_dir];
    full_args.extend(args.iter().map(String::as_str));
    respond_from(guarded_call(&review_path, guard, || {
        run_command(bin_dir, "update-adversarial-review", &full_args)
    }))
}

fn add_adversarial_finding(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    finding_id: &str,
    args: &[String],
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let review_path = Path::new(plan_dir).join("adversarial-review.md");
    let mut full_args: Vec<&str> = vec![plan_dir, finding_id];
    full_args.extend(args.iter().map(String::as_str));
    respond_from(guarded_call(&review_path, guard, || {
        run_command(bin_dir, "add-adversarial-finding", &full_args)
    }))
}

fn resolve_finding(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    finding_id: &str,
    args: &[String],
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let review_path = Path::new(plan_dir).join("adversarial-review.md");
    let mut full_args: Vec<&str> = vec![plan_dir, finding_id];
    full_args.extend(args.iter().map(String::as_str));
    respond_from(guarded_call(&review_path, guard, || {
        run_command(bin_dir, "resolve-finding", &full_args)
    }))
}

fn mint_fix_keys(bin_dir: Option<&Path>, plan_dir: &str) -> Response {
    let fix_keys_path = Path::new(plan_dir).join("fix-keys.json");
    run_then_report_revision(bin_dir, "mint-fix-keys", &[plan_dir], &fix_keys_path)
}

fn verify_fix_keys(bin_dir: Option<&Path>, plan_dir: &str, claimed_by: Option<&str>) -> Response {
    let program = program_path(bin_dir, "verify-fix-keys");
    let mut args: Vec<&str> = vec![plan_dir];
    if let Some(id) = claimed_by {
        args.push("--claimed-by");
        args.push(id);
    }
    match Command::new(&program).args(&args).output() {
        Ok(output) => {
            let mut report = String::from_utf8_lossy(&output.stdout).into_owned();
            report.push_str(&String::from_utf8_lossy(&output.stderr));
            Response::Validated {
                passed: output.status.success(),
                report,
            }
        }
        Err(error) => Response::Error {
            message: format!("could not run {}: {error}", program.display()),
        },
    }
}

fn add_fix_claim(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    finding_id: &str,
    work_unit: &str,
    key: &str,
) -> Response {
    let fixes_path = Path::new(plan_dir).join("fixes.md");
    run_then_report_revision(
        bin_dir,
        "add-fix-claim",
        &[
            plan_dir,
            "--finding",
            finding_id,
            "--work-unit",
            work_unit,
            "--key",
            key,
        ],
        &fixes_path,
    )
}

#[allow(clippy::too_many_arguments)]
fn add_coverage(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    required_outcome: &str,
    work_units: &str,
    notes: &str,
    replace: bool,
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let inventory_path = Path::new(plan_dir).join("work-unit-inventory.md");
    let mut full_args: Vec<&str> = vec![plan_dir, required_outcome, work_units, notes];
    if replace {
        full_args.push("--replace");
    }
    respond_from(guarded_call(&inventory_path, guard, || {
        run_command(bin_dir, "add-coverage", &full_args)
    }))
}

fn remove_coverage(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    required_outcome: &str,
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let inventory_path = Path::new(plan_dir).join("work-unit-inventory.md");
    respond_from(guarded_call(&inventory_path, guard, || {
        run_command(bin_dir, "remove-coverage", &[plan_dir, required_outcome])
    }))
}

fn create_work_unit_inventory(bin_dir: Option<&Path>, plan_dir: &str) -> Response {
    let inventory_path = Path::new(plan_dir).join("work-unit-inventory.md");
    run_then_report_revision(
        bin_dir,
        "create-work-unit-inventory",
        &[plan_dir],
        &inventory_path,
    )
}

fn create_plan_progress(bin_dir: Option<&Path>, plan_dir: &str) -> Response {
    let progress_path = Path::new(plan_dir).join("progress.md");
    run_then_report_revision(bin_dir, "create-plan-progress", &[plan_dir], &progress_path)
}

fn update_plan_progress(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    goal: &str,
    status: &str,
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let progress_path = Path::new(plan_dir).join("progress.md");
    respond_from(guarded_call(&progress_path, guard, || {
        run_command(bin_dir, "update-plan-progress", &[plan_dir, goal, status])
    }))
}

fn rebuild_plan_progress(bin_dir: Option<&Path>, plan_dir: &str) -> Response {
    let progress_path = Path::new(plan_dir).join("progress.md");
    run_then_report_revision(
        bin_dir,
        "rebuild-plan-progress",
        &[plan_dir],
        &progress_path,
    )
}

fn create_plan(bin_dir: Option<&Path>, plan_dir: &str, title: &str) -> Response {
    let description_path = Path::new(plan_dir).join("plan-description.md");
    run_then_report_revision(
        bin_dir,
        "create-plan",
        &[plan_dir, title],
        &description_path,
    )
}

/// remove-plan itself has no confirmation flag at all; `confirm` is this
/// adapter's own gate, checked before the binary is ever invoked, so a
/// caller cannot delete a whole plan through one unconfirmed tool call the
/// way every other write in this crate is only one call away. Reports
/// success/failure and the binary's own output the same way ValidatePlan
/// does, since there is no file left afterward to report a revision for.
fn remove_plan(bin_dir: Option<&Path>, plan_dir: &str, confirm: bool) -> Response {
    if !confirm {
        return Response::Error {
            message: "remove_plan refuses without confirm: true -- this permanently deletes the whole plan directory".to_string(),
        };
    }
    let program = program_path(bin_dir, "remove-plan");
    match Command::new(&program).arg(plan_dir).output() {
        Ok(output) => {
            let mut report = String::from_utf8_lossy(&output.stdout).into_owned();
            report.push_str(&String::from_utf8_lossy(&output.stderr));
            Response::Validated {
                passed: output.status.success(),
                report,
            }
        }
        Err(error) => Response::Error {
            message: format!("could not run {}: {error}", program.display()),
        },
    }
}

/// Bulk-removes completed plans under the whole plans root (no single
/// plan_dir addresses this operation, unlike everything else in this
/// crate). `list_only` is cleanup-plans' own read-only `--list`, needing no
/// confirmation. The real removal mode needs the same adapter-level
/// `confirm: true` gate RemovePlan uses, checked before the binary runs at
/// all, and -- once confirmed -- always runs with `--yes` so
/// cleanup-plans' own interactive confirmation prompt, which would
/// otherwise block forever with no terminal on the other end, is never
/// reached.
fn cleanup_plans(
    bin_dir: Option<&Path>,
    list_only: bool,
    plan_names: &[String],
    confirm: bool,
) -> Response {
    if !list_only && !confirm {
        return Response::Error {
            message: "cleanup_plans refuses without confirm: true, unless list_only -- this permanently deletes plan directories".to_string(),
        };
    }
    let program = program_path(bin_dir, "cleanup-plans");
    let mut args: Vec<&str> = Vec::new();
    if list_only {
        args.push("--list");
    } else {
        args.push("--yes");
    }
    args.extend(plan_names.iter().map(String::as_str));
    match Command::new(&program).args(&args).output() {
        Ok(output) => {
            let mut report = String::from_utf8_lossy(&output.stdout).into_owned();
            report.push_str(&String::from_utf8_lossy(&output.stderr));
            Response::Validated {
                passed: output.status.success(),
                report,
            }
        }
        Err(error) => Response::Error {
            message: format!("could not run {}: {error}", program.display()),
        },
    }
}

fn add_goal(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    goal_name: &str,
    title: &str,
    outcome: &str,
    revision_hex: &str,
) -> Response {
    let guard = match parse_guard(revision_hex) {
        Ok(guard) => guard,
        Err(response) => return response,
    };
    let progress_path = Path::new(plan_dir).join("progress.md");
    respond_from(guarded_call(&progress_path, guard, || {
        run_command(bin_dir, "add-goal", &[plan_dir, goal_name, title, outcome])
    }))
}

/// Runs a read-only command and reports pass/fail plus its combined
/// stdout+stderr as `report` -- the same shape ValidatePlan/VerifyFixKeys
/// already use for "ran a command, here is what it printed", reused here
/// rather than inventing a document-shaped response for output that has no
/// revision to guard.
fn run_readonly(bin_dir: Option<&Path>, name: &str, args: &[&str]) -> Response {
    let program = program_path(bin_dir, name);
    match Command::new(&program).args(args).output() {
        Ok(output) => {
            let mut report = String::from_utf8_lossy(&output.stdout).into_owned();
            report.push_str(&String::from_utf8_lossy(&output.stderr));
            Response::Validated {
                passed: output.status.success(),
                report,
            }
        }
        Err(error) => Response::Error {
            message: format!("could not run {}: {error}", program.display()),
        },
    }
}

fn plan_root(bin_dir: Option<&Path>, directory: Option<&str>) -> Response {
    let mut args: Vec<&str> = vec!["project-root"];
    if let Some(directory) = directory {
        args.push(directory);
    }
    run_readonly(bin_dir, "plan-root", &args)
}

fn register_read(
    bin_dir: Option<&Path>,
    kind: &str,
    mode: &str,
    args: &[String],
    file: &str,
) -> Response {
    let mut full_args: Vec<&str> = vec![kind, mode];
    full_args.extend(args.iter().map(String::as_str));
    full_args.push("--file");
    full_args.push(file);
    run_readonly(bin_dir, "register-read", &full_args)
}

#[allow(clippy::too_many_arguments)]
fn add_planning_bug(
    bin_dir: Option<&Path>,
    plan_dir: &str,
    id: &str,
    title: &str,
    reproduce: &str,
    observed: &str,
    expected: &str,
    args: &[String],
) -> Response {
    let bugs_path = Path::new(plan_dir).join("planning-bugs.json");
    let mut full_args: Vec<&str> = vec![
        plan_dir,
        "--id",
        id,
        "--title",
        title,
        "--reproduce",
        reproduce,
        "--observed",
        observed,
        "--expected",
        expected,
    ];
    full_args.extend(args.iter().map(String::as_str));
    run_then_report_revision(bin_dir, "add-planning-bug", &full_args, &bugs_path)
}

fn validate_plan(bin_dir: Option<&Path>, plan_dir: &str, complete: bool) -> Response {
    let program = program_path(bin_dir, "validate-plan");
    let mut args: Vec<&str> = Vec::new();
    if complete {
        args.push("--complete");
    }
    args.push(plan_dir);
    match Command::new(&program).args(&args).output() {
        Ok(output) => {
            let mut report = String::from_utf8_lossy(&output.stdout).into_owned();
            report.push_str(&String::from_utf8_lossy(&output.stderr));
            Response::Validated {
                passed: output.status.success(),
                report,
            }
        }
        Err(error) => Response::Error {
            message: format!("could not run {}: {error}", program.display()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let mut dir = std::env::temp_dir();
            dir.push(format!(
                "planning-server-handlers-test-{}-{}",
                std::process::id(),
                unique()
            ));
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn unique() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Walks up from this test binary's own path to the workspace's shared
    /// target/{debug,release} directory, where every workspace binary this
    /// crate delegates to (update-step, add-work-unit, update-plan-content,
    /// validate-plan, create-plan, add-goal) lands once built -- avoiding
    /// any PATH mutation (unsafe on this toolchain) by resolving each
    /// program's full path explicitly instead.
    fn sibling_bin_dir() -> PathBuf {
        let mut dir = std::env::current_exe().expect("current test binary path");
        dir.pop(); // the test binary itself
        if dir.file_name().is_some_and(|name| name == "deps") {
            dir.pop();
        }
        dir
    }

    /// Builds `name` into `bin_dir` if it is not there yet.
    ///
    /// This crate has no Cargo dependency edge on create-plan/add-goal/etc
    /// (they are invoked as plain subprocesses, not linked), so `cargo test
    /// --workspace` gives no ordering guarantee that they finish building
    /// before this crate's own test binaries start running -- its scheduler
    /// runs a package's tests as soon as THAT package is ready, in parallel
    /// with unrelated packages still compiling. Observed for real in CI (13
    /// passed, 8 failed, "No such file or directory" for create-plan) but
    /// never locally, where a prior full build already left the binary
    /// staged -- a scheduling race, not a environment difference.
    fn ensure_built(bin_dir: &Path, name: &str) -> PathBuf {
        // Some commands run a sibling binary from their own directory
        // (update-step hands the progress rewrite to update-progress,
        // update-plan-content verifies fix keys with verify-fix-keys), and
        // `cargo test --workspace` builds neither as a side effect.
        let siblings: &[&str] = match name {
            "update-step" => &["update-progress"],
            "update-plan-content" => &["verify-fix-keys"],
            _ => &[],
        };
        for sibling in siblings {
            ensure_built(bin_dir, sibling);
        }
        let program = bin_dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        if program.is_file() {
            return program;
        }
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("build").arg("-p").arg(name);
        // bin_dir is target/debug (native) or target/<triple>/debug
        // (cross-compiled); an explicit --target is required in the second
        // case or this build would land in target/debug instead, right
        // where bin_dir does NOT point. Either way, walk up from bin_dir
        // past whatever sits above "target" -- one level native, two
        // cross-compiled -- to find the workspace root cargo must run from
        // for its own default output location to match bin_dir.
        if let Some(triple) = bin_dir
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .filter(|name| *name != "target")
        {
            cmd.arg("--target").arg(triple);
        }
        let mut workspace_root = bin_dir.to_path_buf();
        loop {
            let popped = workspace_root.file_name().map(|n| n.to_os_string());
            if !workspace_root.pop() {
                panic!("bin_dir has no 'target' ancestor: {}", bin_dir.display());
            }
            if popped.as_deref() == Some(std::ffi::OsStr::new("target")) {
                break;
            }
        }
        // One build at a time: tests run on parallel threads and two of them
        // asking for the same sibling would otherwise race to build it.
        static BUILDING: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _one_at_a_time = BUILDING.lock().unwrap_or_else(|p| p.into_inner());
        if program.is_file() {
            return program;
        }
        let output = cmd
            .arg("--message-format=json-render-diagnostics")
            .current_dir(&workspace_root)
            .output()
            .unwrap_or_else(|error| panic!("could not build {name}: {error}"));
        assert!(
            output.status.success(),
            "building {name} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if !program.is_file() {
            // Cargo reports where it actually put the binary; trust that over
            // the target/<triple>/debug layout assumed above.
            let stdout = String::from_utf8_lossy(&output.stdout);
            let built = stdout
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter(|message| {
                    message["reason"] == "compiler-artifact" && message["target"]["name"] == name
                })
                .filter_map(|message| message["executable"].as_str().map(PathBuf::from))
                .next_back();
            if let Some(built) = built.filter(|path| path.is_file()) {
                std::fs::copy(&built, &program).unwrap_or_else(|error| {
                    panic!("copy {} to {}: {error}", built.display(), program.display())
                });
            }
        }
        assert!(
            program.is_file(),
            "{name} still missing at {} after building it; cargo reported:\n{}\n{}",
            program.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        program
    }

    fn run(bin_dir: &Path, name: &str, args: &[&str]) {
        let program = ensure_built(bin_dir, name);
        let output = Command::new(&program)
            .args(args)
            .output()
            .unwrap_or_else(|error| panic!("could not run {}: {error}", program.display()));
        assert!(
            output.status.success(),
            "{} {:?} failed: {}",
            program.display(),
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Copies `src` (a plan directory) into a fresh directory named
    /// `container` under `scratch`, PRESERVING src's own basename -- some
    /// generated plan documents (progress.md's own title line) embed the
    /// plan directory's basename, so two copies compared byte-for-byte must
    /// keep the identical basename or an unrelated naming difference reads
    /// as a false parity mismatch. Returns the copy's own plan-directory path.
    fn cloned_plan(scratch: &Path, container: &str, src: &Path) -> PathBuf {
        let dst_parent = scratch.join(container);
        fs::create_dir_all(&dst_parent).unwrap();
        let output = Command::new("cp")
            .args(["-r", &src.to_string_lossy(), &dst_parent.to_string_lossy()])
            .output()
            .expect("run cp -r");
        assert!(
            output.status.success(),
            "cp -r failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        dst_parent.join(src.file_name().unwrap())
    }

    /// A relative-path -> content snapshot of every file under `root`, for
    /// asserting two plan-directory copies ended up byte-identical after
    /// two different mechanisms (the handler vs. the standalone command)
    /// each applied what should be the same change to an equivalent start.
    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else {
                    let relative = path.strip_prefix(root).unwrap().to_path_buf();
                    out.insert(relative, fs::read(&path).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    fn setup_plan(bin_dir: &Path, dir: &Path) -> PathBuf {
        let plan_dir = dir.join("plan");
        run(
            bin_dir,
            "create-plan",
            &[plan_dir.to_str().unwrap(), "Demo plan"],
        );
        run(
            bin_dir,
            "add-goal",
            &[
                plan_dir.to_str().unwrap(),
                "01-demo",
                "Demo goal",
                "Demo outcome",
            ],
        );
        run(
            bin_dir,
            "create-adversarial-review",
            &[plan_dir.to_str().unwrap()],
        );
        plan_dir
    }

    fn assert_snapshots_match(a: &Path, b: &Path) {
        let snap_a = snapshot(a);
        let snap_b = snapshot(b);
        if snap_a == snap_b {
            return;
        }
        let mut report = String::new();
        for key in snap_a
            .keys()
            .chain(snap_b.keys())
            .collect::<std::collections::BTreeSet<_>>()
        {
            match (snap_a.get(key), snap_b.get(key)) {
                (Some(x), Some(y)) if x != y => {
                    report.push_str(&format!(
                        "--- differs: {} ---\nA: {}\nB: {}\n",
                        key.display(),
                        String::from_utf8_lossy(x),
                        String::from_utf8_lossy(y)
                    ));
                }
                (Some(_), None) => report.push_str(&format!("only in A: {}\n", key.display())),
                (None, Some(_)) => report.push_str(&format!("only in B: {}\n", key.display())),
                _ => {}
            }
        }
        panic!("snapshots differ:\n{report}");
    }

    #[test]
    fn read_plan_document_returns_the_real_content_and_a_matching_revision() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());

        let response = dispatch_with_bin_dir(
            Request::ReadPlanDocument {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                document_id: "plan".to_string(),
                view: None,
            },
            Some(&bin_dir),
        );
        match response {
            Response::Document { content, revision } => {
                assert!(content.contains("Demo plan"));
                let raw = fs::read(plan_dir.join("plan-description.md")).unwrap();
                assert_eq!(revision, PlanRevision::of(&raw).to_hex());
            }
            other => panic!("expected Document, got {other:?}"),
        }
    }

    #[test]
    fn read_work_unit_is_sugar_for_reading_the_units_own_step_document() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        run(
            &bin_dir,
            "add-work-unit",
            &[
                plan_dir.to_str().unwrap(),
                "--id",
                "W01",
                "--type",
                "source",
                "--file",
                "src/x.rs",
                "--scope",
                "x",
                "--subscope",
                "N/A",
                "--change",
                "do x",
                "--depends-on",
                "--",
                "--goal",
                "01-demo",
                "--step",
                "01-step-x",
            ],
        );

        let via_unit = dispatch_with_bin_dir(
            Request::ReadWorkUnit {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                unit_id: "W01".to_string(),
            },
            Some(&bin_dir),
        );
        let via_document = dispatch_with_bin_dir(
            Request::ReadPlanDocument {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                document_id: "unit:W01".to_string(),
                view: None,
            },
            Some(&bin_dir),
        );
        assert_eq!(via_unit, via_document);
        match via_unit {
            Response::Document { content, .. } => {
                assert!(
                    content.contains("do x"),
                    "expected the step's own content, got: {content}"
                )
            }
            other => panic!("expected Document, got {other:?}"),
        }
    }

    #[test]
    fn update_step_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        run(
            &bin_dir,
            "add-work-unit",
            &[
                plan_dir.to_str().unwrap(),
                "--id",
                "W01",
                "--type",
                "source",
                "--file",
                "src/x.rs",
                "--scope",
                "x",
                "--subscope",
                "N/A",
                "--change",
                "do x",
                "--depends-on",
                "--",
                "--goal",
                "01-demo",
                "--step",
                "01-step-x",
            ],
        );

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let progress_path = copy_a.join("01-demo").join("progress.md");
        let (_, guard) = read_with_revision(&progress_path).unwrap();
        ensure_built(&bin_dir, "update-step");
        let response = dispatch_with_bin_dir(
            Request::UpdateStep {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                goal: "01-demo".to_string(),
                step: "01-step-x".to_string(),
                status: "in-progress".to_string(),
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "update-step",
            &[
                copy_b.join("01-demo").to_str().unwrap(),
                "01-step-x",
                "in-progress",
            ],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn update_step_with_a_stale_revision_is_refused_and_changes_nothing() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        run(
            &bin_dir,
            "add-work-unit",
            &[
                plan_dir.to_str().unwrap(),
                "--id",
                "W01",
                "--type",
                "source",
                "--file",
                "src/x.rs",
                "--scope",
                "x",
                "--subscope",
                "N/A",
                "--change",
                "do x",
                "--depends-on",
                "--",
                "--goal",
                "01-demo",
                "--step",
                "01-step-x",
            ],
        );
        let before = snapshot(&plan_dir);

        let bogus_guard = PlanRevision::of(b"not the real hash");
        let response = dispatch_with_bin_dir(
            Request::UpdateStep {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                goal: "01-demo".to_string(),
                step: "01-step-x".to_string(),
                status: "in-progress".to_string(),
                revision: bogus_guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Stale { .. }),
            "expected Stale, got {response:?}"
        );
        assert_eq!(
            before,
            snapshot(&plan_dir),
            "a stale UpdateStep must change nothing on disk"
        );
    }

    #[test]
    fn add_work_unit_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let inventory_path = copy_a.join("work-unit-inventory.md");
        let (_, guard) = read_with_revision(&inventory_path).unwrap();
        ensure_built(&bin_dir, "add-work-unit");
        let response = dispatch_with_bin_dir(
            Request::AddWorkUnit {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                id: "W01".to_string(),
                unit_type: "source".to_string(),
                file: "src/x.rs".to_string(),
                scope: "x".to_string(),
                subscope: "N/A".to_string(),
                change: "do x".to_string(),
                depends_on: "--".to_string(),
                goal: "01-demo".to_string(),
                step: "01-step-x".to_string(),
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "add-work-unit",
            &[
                copy_b.to_str().unwrap(),
                "--id",
                "W01",
                "--type",
                "source",
                "--file",
                "src/x.rs",
                "--scope",
                "x",
                "--subscope",
                "N/A",
                "--change",
                "do x",
                "--depends-on",
                "--",
                "--goal",
                "01-demo",
                "--step",
                "01-step-x",
            ],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn set_review_status_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let plan_description = copy_a.join("plan-description.md");
        let (_, guard) = read_with_revision(&plan_description).unwrap();
        ensure_built(&bin_dir, "update-plan-content");
        let response = dispatch_with_bin_dir(
            Request::SetReviewStatus {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                status: "pending".to_string(),
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "update-plan-content",
            &["--review-status", copy_b.to_str().unwrap(), "pending"],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn set_testing_requirement_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let goal_path = copy_a.join("01-demo").join("goal.md");
        let (_, guard) = read_with_revision(&goal_path).unwrap();
        ensure_built(&bin_dir, "update-plan-content");
        let response = dispatch_with_bin_dir(
            Request::SetTestingRequirement {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                goal: "01-demo".to_string(),
                required: true,
                rationale: "exercised directly".to_string(),
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "update-plan-content",
            &[
                "--testing-requirement",
                copy_b.to_str().unwrap(),
                "01-demo",
                "yes",
                "exercised directly",
            ],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    /// The same add-work-unit invocation `read_work_unit_is_sugar_for...`
    /// and both `update_step` tests already repeat inline, factored out
    /// only for the new tests below (existing ones are left as they are,
    /// to keep this change's diff scoped to what it actually touches).
    fn add_demo_work_unit(bin_dir: &Path, plan_dir: &Path) {
        run(
            bin_dir,
            "add-work-unit",
            &[
                plan_dir.to_str().unwrap(),
                "--id",
                "W01",
                "--type",
                "source",
                "--file",
                "src/x.rs",
                "--scope",
                "x",
                "--subscope",
                "N/A",
                "--change",
                "do x",
                "--depends-on",
                "--",
                "--goal",
                "01-demo",
                "--step",
                "01-step-x",
            ],
        );
    }

    #[test]
    fn update_work_unit_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        add_demo_work_unit(&bin_dir, &plan_dir);

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let inventory_path = copy_a.join("work-unit-inventory.md");
        let (_, guard) = read_with_revision(&inventory_path).unwrap();
        ensure_built(&bin_dir, "update-work-unit");
        let response = dispatch_with_bin_dir(
            Request::UpdateWorkUnit {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                unit_id: "W01".to_string(),
                args: vec!["--scope".to_string(), "new scope".to_string()],
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "update-work-unit",
            &[copy_b.to_str().unwrap(), "W01", "--scope", "new scope"],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn update_work_unit_with_a_stale_revision_is_refused_and_changes_nothing() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        add_demo_work_unit(&bin_dir, &plan_dir);
        let before = snapshot(&plan_dir);

        let bogus_guard = PlanRevision::of(b"not the real hash");
        let response = dispatch_with_bin_dir(
            Request::UpdateWorkUnit {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                unit_id: "W01".to_string(),
                args: vec!["--scope".to_string(), "new scope".to_string()],
                revision: bogus_guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Stale { .. }),
            "expected Stale, got {response:?}"
        );
        assert_eq!(
            before,
            snapshot(&plan_dir),
            "a stale guard must change nothing"
        );
    }

    #[test]
    fn remove_work_unit_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        add_demo_work_unit(&bin_dir, &plan_dir);

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let inventory_path = copy_a.join("work-unit-inventory.md");
        let (_, guard) = read_with_revision(&inventory_path).unwrap();
        ensure_built(&bin_dir, "remove-work-unit");
        let response = dispatch_with_bin_dir(
            Request::RemoveWorkUnit {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                unit_id: "W01".to_string(),
                confirm_cascade: false,
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "remove-work-unit",
            &[copy_b.to_str().unwrap(), "W01"],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn update_plan_content_decomposition_review_matches_the_standalone_command() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let plan_description = copy_a.join("plan-description.md");
        let (_, guard) = read_with_revision(&plan_description).unwrap();
        ensure_built(&bin_dir, "update-plan-content");
        let response = dispatch_with_bin_dir(
            Request::UpdatePlanContent {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                mode: "decomposition-review".to_string(),
                args: vec!["completed".to_string()],
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "update-plan-content",
            &[
                "--decomposition-review",
                copy_b.to_str().unwrap(),
                "completed",
            ],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn update_plan_content_title_matches_the_standalone_command_on_a_generic_document_id() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let plan_description = copy_a.join("plan-description.md");
        let (_, guard) = read_with_revision(&plan_description).unwrap();
        ensure_built(&bin_dir, "update-plan-content");
        let response = dispatch_with_bin_dir(
            Request::UpdatePlanContent {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                mode: "title".to_string(),
                args: vec!["plan".to_string(), "A new title".to_string()],
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "update-plan-content",
            &["--title", copy_b.to_str().unwrap(), "plan", "A new title"],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn update_plan_content_with_an_unknown_mode_is_a_clean_error_not_a_panic() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());

        let response = dispatch_with_bin_dir(
            Request::UpdatePlanContent {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                mode: "not-a-real-mode".to_string(),
                args: vec![],
                revision: "0".repeat(64),
            },
            Some(&bin_dir),
        );
        match response {
            Response::Error { message } => {
                assert!(message.contains("unknown update-plan-content mode"))
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn create_adversarial_review_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        // setup_plan already runs create-adversarial-review; remove it from
        // both copies first so there is something to create.
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);
        fs::remove_file(copy_a.join("adversarial-review.md")).unwrap();
        fs::remove_file(copy_b.join("adversarial-review.md")).unwrap();

        ensure_built(&bin_dir, "create-adversarial-review");
        let response = dispatch_with_bin_dir(
            Request::CreateAdversarialReview {
                plan_dir: copy_a.to_string_lossy().into_owned(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "create-adversarial-review",
            &[copy_b.to_str().unwrap()],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn create_adversarial_review_refuses_cleanly_when_it_already_exists() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        // setup_plan already runs create-adversarial-review once.
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        ensure_built(&bin_dir, "create-adversarial-review");

        let response = dispatch_with_bin_dir(
            Request::CreateAdversarialReview {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
            },
            Some(&bin_dir),
        );
        match response {
            Response::Error { message } => assert!(!message.is_empty()),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn the_adversarial_review_and_fix_key_workflow_runs_end_to_end() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        add_demo_work_unit(&bin_dir, &plan_dir);
        for name in [
            "add-adversarial-finding",
            "resolve-finding",
            "mint-fix-keys",
            "verify-fix-keys",
            "add-fix-claim",
        ] {
            ensure_built(&bin_dir, name);
        }
        let plan_dir_str = plan_dir.to_string_lossy().into_owned();

        // 1. Gate a finding on W01, guarded on adversarial-review.md.
        // AR-01 is create-adversarial-review's own scaffolded placeholder
        // row, so the new finding this test adds is AR-02.
        let review_path = plan_dir.join("adversarial-review.md");
        let (_, guard) = read_with_revision(&review_path).unwrap();
        let response = dispatch_with_bin_dir(
            Request::AddAdversarialFinding {
                plan_dir: plan_dir_str.clone(),
                finding_id: "AR-02".to_string(),
                args: vec![
                    "Missing X".to_string(),
                    "Add X".to_string(),
                    "--work-unit".to_string(),
                    "W01".to_string(),
                ],
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written for add_adversarial_finding, got {response:?}"
        );

        // 2. Mint fix keys (unguarded, full regeneration).
        let response = dispatch_with_bin_dir(
            Request::MintFixKeys {
                plan_dir: plan_dir_str.clone(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written for mint_fix_keys, got {response:?}"
        );
        let fix_keys: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(plan_dir.join("fix-keys.json")).unwrap())
                .unwrap();
        let key = fix_keys["keys"]["AR-02"]["W01"]
            .as_str()
            .expect("a minted key for AR-02/W01")
            .to_string();

        // 3. Claim the fix (unguarded, append-only).
        let response = dispatch_with_bin_dir(
            Request::AddFixClaim {
                plan_dir: plan_dir_str.clone(),
                finding_id: "AR-02".to_string(),
                work_unit: "W01".to_string(),
                key: key.clone(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written for add_fix_claim, got {response:?}"
        );

        // 4. Verify the claim (read-only, unguarded) -- must pass.
        let response = dispatch_with_bin_dir(
            Request::VerifyFixKeys {
                plan_dir: plan_dir_str.clone(),
                claimed_by: None,
            },
            Some(&bin_dir),
        );
        match response {
            Response::Validated { passed, report } => {
                assert!(passed, "expected the claim to verify, got: {report}")
            }
            other => panic!("expected Validated, got {other:?}"),
        }

        // 5. Resolve the finding, guarded on adversarial-review.md.
        let (_, guard) = read_with_revision(&review_path).unwrap();
        let response = dispatch_with_bin_dir(
            Request::ResolveFinding {
                plan_dir: plan_dir_str.clone(),
                finding_id: "AR-02".to_string(),
                args: vec!["--status".to_string(), "resolved".to_string()],
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        match response {
            Response::Written { .. } => {}
            other => panic!("expected Written for resolve_finding, got {other:?}"),
        }
        let after_resolve = fs::read_to_string(&review_path).unwrap();
        assert!(
            after_resolve.contains("resolved"),
            "expected the finding's status to read resolved: {after_resolve}"
        );
    }

    #[test]
    fn add_adversarial_finding_with_a_stale_revision_is_refused_and_changes_nothing() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        let before = snapshot(&plan_dir);

        let bogus_guard = PlanRevision::of(b"not the real hash");
        let response = dispatch_with_bin_dir(
            Request::AddAdversarialFinding {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                finding_id: "AR-02".to_string(),
                args: vec!["Missing X".to_string(), "Add X".to_string()],
                revision: bogus_guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Stale { .. }),
            "expected Stale, got {response:?}"
        );
        assert_eq!(
            before,
            snapshot(&plan_dir),
            "a stale guard must change nothing"
        );
    }

    #[test]
    fn add_coverage_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        add_demo_work_unit(&bin_dir, &plan_dir);

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let inventory_path = copy_a.join("work-unit-inventory.md");
        let (_, guard) = read_with_revision(&inventory_path).unwrap();
        ensure_built(&bin_dir, "add-coverage");
        let response = dispatch_with_bin_dir(
            Request::AddCoverage {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                required_outcome: "It works".to_string(),
                work_units: "W01".to_string(),
                notes: "verified manually".to_string(),
                replace: false,
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "add-coverage",
            &[
                copy_b.to_str().unwrap(),
                "It works",
                "W01",
                "verified manually",
            ],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn add_coverage_with_a_stale_revision_is_refused_and_changes_nothing() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        add_demo_work_unit(&bin_dir, &plan_dir);
        let before = snapshot(&plan_dir);

        let bogus_guard = PlanRevision::of(b"not the real hash");
        let response = dispatch_with_bin_dir(
            Request::AddCoverage {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                required_outcome: "It works".to_string(),
                work_units: "W01".to_string(),
                notes: "verified manually".to_string(),
                replace: false,
                revision: bogus_guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Stale { .. }),
            "expected Stale, got {response:?}"
        );
        assert_eq!(
            before,
            snapshot(&plan_dir),
            "a stale guard must change nothing"
        );
    }

    #[test]
    fn remove_coverage_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        add_demo_work_unit(&bin_dir, &plan_dir);
        run(
            &bin_dir,
            "add-coverage",
            &[
                plan_dir.to_str().unwrap(),
                "It works",
                "W01",
                "verified manually",
            ],
        );

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let inventory_path = copy_a.join("work-unit-inventory.md");
        let (_, guard) = read_with_revision(&inventory_path).unwrap();
        ensure_built(&bin_dir, "remove-coverage");
        let response = dispatch_with_bin_dir(
            Request::RemoveCoverage {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                required_outcome: "It works".to_string(),
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "remove-coverage",
            &[copy_b.to_str().unwrap(), "It works"],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn create_work_unit_inventory_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        // create-plan already scaffolds work-unit-inventory.md; remove it
        // from both copies first so there is something to create.
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);
        fs::remove_file(copy_a.join("work-unit-inventory.md")).unwrap();
        fs::remove_file(copy_b.join("work-unit-inventory.md")).unwrap();

        ensure_built(&bin_dir, "create-work-unit-inventory");
        let response = dispatch_with_bin_dir(
            Request::CreateWorkUnitInventory {
                plan_dir: copy_a.to_string_lossy().into_owned(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "create-work-unit-inventory",
            &[copy_b.to_str().unwrap()],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn create_plan_progress_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        // setup_plan's own add-goal call already wrote progress.md (it
        // rebuilds the file wholesale from every goal directory present,
        // progress.md is not part of create-plan's own scaffold); remove
        // it from both copies first so there is something to create.
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);
        fs::remove_file(copy_a.join("progress.md")).unwrap();
        fs::remove_file(copy_b.join("progress.md")).unwrap();

        ensure_built(&bin_dir, "create-plan-progress");
        let response = dispatch_with_bin_dir(
            Request::CreatePlanProgress {
                plan_dir: copy_a.to_string_lossy().into_owned(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "create-plan-progress",
            &[copy_b.to_str().unwrap()],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn update_plan_progress_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let progress_path = copy_a.join("progress.md");
        let (_, guard) = read_with_revision(&progress_path).unwrap();
        ensure_built(&bin_dir, "update-plan-progress");
        let response = dispatch_with_bin_dir(
            Request::UpdatePlanProgress {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                goal: "01-demo".to_string(),
                status: "in-progress".to_string(),
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "update-plan-progress",
            &[copy_b.to_str().unwrap(), "01-demo", "in-progress"],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn rebuild_plan_progress_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        add_demo_work_unit(&bin_dir, &plan_dir);

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        ensure_built(&bin_dir, "rebuild-plan-progress");
        let response = dispatch_with_bin_dir(
            Request::RebuildPlanProgress {
                plan_dir: copy_a.to_string_lossy().into_owned(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "rebuild-plan-progress",
            &[copy_b.to_str().unwrap()],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn create_plan_scaffolds_a_real_plan_directory() {
        // Not an assert_snapshots_match comparison against a standalone run
        // at a different path: create-plan bakes each plan's own absolute
        // path into .env and its initial git commit, so two independently
        // created plans can never be byte-identical even with the same
        // basename -- unlike every other "matches the standalone command"
        // test in this file, which clones ONE already-created plan so both
        // sides inherit the identical (if path-stale) baked-in content.
        // This checks create_plan's own real properties instead.
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = scratch.path().join("new-plan");

        ensure_built(&bin_dir, "create-plan");
        let response = dispatch_with_bin_dir(
            Request::CreatePlan {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                title: "Demo plan".to_string(),
            },
            Some(&bin_dir),
        );
        let Response::Written { revision } = response else {
            panic!("expected Written, got {response:?}");
        };
        let description_path = plan_dir.join("plan-description.md");
        let raw = fs::read(&description_path).unwrap();
        assert_eq!(
            revision,
            PlanRevision::of(&raw).to_hex(),
            "the reported revision must match the file actually written"
        );
        assert!(
            String::from_utf8_lossy(&raw).contains("Demo plan"),
            "plan-description.md must carry the title"
        );
        assert!(
            plan_dir.join("work-unit-inventory.md").is_file(),
            "create-plan must also scaffold work-unit-inventory.md"
        );
        // progress.md is NOT part of create-plan's own scaffold -- it is
        // add-goal that first writes it (rebuilding it wholesale from
        // every goal directory present, including the one it just added),
        // confirmed directly against add-goal's own rebuild_plan_progress.
        assert!(
            !plan_dir.join("progress.md").exists(),
            "progress.md should not exist before any goal has been added"
        );
    }

    #[test]
    fn create_plan_refuses_cleanly_when_the_target_already_exists() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        ensure_built(&bin_dir, "create-plan");

        let response = dispatch_with_bin_dir(
            Request::CreatePlan {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                title: "Demo plan".to_string(),
            },
            Some(&bin_dir),
        );
        match response {
            Response::Error { message } => assert!(!message.is_empty()),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn remove_plan_refuses_without_confirm_and_leaves_the_plan_on_disk() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());

        let response = dispatch_with_bin_dir(
            Request::RemovePlan {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                confirm: false,
            },
            Some(&bin_dir),
        );
        match response {
            Response::Error { message } => assert!(message.contains("confirm")),
            other => panic!("expected Error, got {other:?}"),
        }
        assert!(plan_dir.is_dir(), "the plan must still exist on disk");
    }

    #[test]
    fn remove_plan_with_confirm_matches_the_standalone_command() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        ensure_built(&bin_dir, "remove-plan");

        let response = dispatch_with_bin_dir(
            Request::RemovePlan {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                confirm: true,
            },
            Some(&bin_dir),
        );
        match response {
            Response::Validated { passed, report } => {
                assert!(passed, "expected the removal to succeed, got: {report}")
            }
            other => panic!("expected Validated, got {other:?}"),
        }
        assert!(
            !plan_dir.exists(),
            "the plan directory must be gone after a confirmed removal"
        );
    }

    #[test]
    fn cleanup_plans_list_only_never_deletes_anything() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        ensure_built(&bin_dir, "cleanup-plans");

        let response = dispatch_with_bin_dir(
            Request::CleanupPlans {
                list_only: true,
                plan_names: Vec::new(),
                confirm: false,
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Validated { .. }),
            "expected Validated, got {response:?}"
        );
        assert!(
            plan_dir.is_dir(),
            "list_only must never remove anything, regardless of what it lists"
        );
    }

    #[test]
    fn add_goal_matches_the_standalone_command_on_an_equivalent_copy() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());

        let copy_a = cloned_plan(scratch.path(), "copy-a", &plan_dir);
        let copy_b = cloned_plan(scratch.path(), "copy-b", &plan_dir);

        let progress_path = copy_a.join("progress.md");
        let (_, guard) = read_with_revision(&progress_path).unwrap();
        ensure_built(&bin_dir, "add-goal");
        let response = dispatch_with_bin_dir(
            Request::AddGoal {
                plan_dir: copy_a.to_string_lossy().into_owned(),
                goal_name: "02-next".to_string(),
                title: "Next goal".to_string(),
                outcome: "Next outcome".to_string(),
                revision: guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Written { .. }),
            "expected Written, got {response:?}"
        );

        run(
            &bin_dir,
            "add-goal",
            &[
                copy_b.to_str().unwrap(),
                "02-next",
                "Next goal",
                "Next outcome",
            ],
        );

        assert_snapshots_match(&copy_a, &copy_b);
    }

    #[test]
    fn add_goal_with_a_stale_revision_is_refused_and_changes_nothing() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        let before = snapshot(&plan_dir);

        let bogus_guard = PlanRevision::of(b"not the real hash");
        let response = dispatch_with_bin_dir(
            Request::AddGoal {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                goal_name: "02-next".to_string(),
                title: "Next goal".to_string(),
                outcome: "Next outcome".to_string(),
                revision: bogus_guard.to_hex(),
            },
            Some(&bin_dir),
        );
        assert!(
            matches!(response, Response::Stale { .. }),
            "expected Stale, got {response:?}"
        );
        assert_eq!(
            before,
            snapshot(&plan_dir),
            "a stale guard must change nothing"
        );
    }

    #[test]
    fn plan_root_reports_the_resolved_project_root() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        ensure_built(&bin_dir, "plan-root");

        let response = dispatch_with_bin_dir(
            Request::PlanRoot {
                directory: Some(scratch.path().to_string_lossy().into_owned()),
            },
            Some(&bin_dir),
        );
        match response {
            Response::Validated { passed, report } => {
                assert!(passed, "expected plan-root to succeed, got: {report}");
                assert!(
                    !report.trim().is_empty(),
                    "expected plan-root to print the resolved root"
                );
            }
            other => panic!("expected Validated, got {other:?}"),
        }
    }

    #[test]
    fn register_read_matches_the_standalone_command_on_a_fresh_register() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let register = scratch.path().join("BUGS.json");
        fs::write(&register, r#"{"bugs": []}"#).unwrap();
        ensure_built(&bin_dir, "register-read");

        let response = dispatch_with_bin_dir(
            Request::RegisterRead {
                kind: "bug".to_string(),
                mode: "count".to_string(),
                args: Vec::new(),
                file: register.to_string_lossy().into_owned(),
            },
            Some(&bin_dir),
        );
        let Response::Validated { passed, report } = response else {
            panic!("expected Validated, got {response:?}");
        };
        assert!(passed, "expected register-read to succeed, got: {report}");

        let program = bin_dir.join("register-read");
        let output = Command::new(&program)
            .args(["bug", "count", "--file", &register.to_string_lossy()])
            .output()
            .unwrap();
        let mut expected = String::from_utf8_lossy(&output.stdout).into_owned();
        expected.push_str(&String::from_utf8_lossy(&output.stderr));
        assert_eq!(report, expected);
    }

    #[test]
    fn register_read_reports_failure_cleanly_for_a_missing_file() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        ensure_built(&bin_dir, "register-read");

        let response = dispatch_with_bin_dir(
            Request::RegisterRead {
                kind: "bug".to_string(),
                mode: "count".to_string(),
                args: Vec::new(),
                file: scratch
                    .path()
                    .join("does-not-exist.json")
                    .to_string_lossy()
                    .into_owned(),
            },
            Some(&bin_dir),
        );
        match response {
            Response::Validated { passed, .. } => assert!(!passed),
            other => panic!("expected Validated, got {other:?}"),
        }
    }

    #[test]
    fn add_planning_bug_creates_the_register_with_the_right_entry() {
        // Not an assert_snapshots_match comparison against a standalone run:
        // add-planning-bug embeds a real wall-clock created_at/updated_at
        // timestamp (confirmed directly in its own source), so two
        // independently run invocations can differ in that one field even
        // with identical arguments -- the same class of thing that ruled out
        // a byte-for-byte comparison for create_plan. This checks the
        // write's own real properties instead.
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = scratch.path().join("plan");
        fs::create_dir_all(&plan_dir).unwrap();

        ensure_built(&bin_dir, "add-planning-bug");
        let response = dispatch_with_bin_dir(
            Request::AddPlanningBug {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                id: "PB-01".to_string(),
                title: "It breaks".to_string(),
                reproduce: "run it".to_string(),
                observed: "it broke".to_string(),
                expected: "it should not".to_string(),
                args: vec!["--severity".to_string(), "major".to_string()],
            },
            Some(&bin_dir),
        );
        let Response::Written { revision } = response else {
            panic!("expected Written, got {response:?}");
        };
        let bugs_path = plan_dir.join("planning-bugs.json");
        let raw = fs::read(&bugs_path).unwrap();
        assert_eq!(
            revision,
            PlanRevision::of(&raw).to_hex(),
            "the reported revision must match the file actually written"
        );
        let text = String::from_utf8_lossy(&raw);
        for expected in ["PB-01", "It breaks", "run it", "it broke", "major"] {
            assert!(
                text.contains(expected),
                "expected planning-bugs.json to contain {expected:?}: {text}"
            );
        }
    }

    #[test]
    fn validate_plan_matches_the_standalone_binarys_own_output() {
        let bin_dir = sibling_bin_dir();
        let scratch = TempDir::new();
        let plan_dir = setup_plan(&bin_dir, scratch.path());
        ensure_built(&bin_dir, "validate-plan");

        let response = dispatch_with_bin_dir(
            Request::ValidatePlan {
                plan_dir: plan_dir.to_string_lossy().into_owned(),
                complete: false,
            },
            Some(&bin_dir),
        );
        let Response::Validated { passed, report } = response else {
            panic!("expected Validated");
        };

        let program = bin_dir.join("validate-plan");
        let output = Command::new(&program)
            .arg(plan_dir.to_str().unwrap())
            .output()
            .unwrap();
        let mut expected_report = String::from_utf8_lossy(&output.stdout).into_owned();
        expected_report.push_str(&String::from_utf8_lossy(&output.stderr));

        assert_eq!(passed, output.status.success());
        assert_eq!(report, expected_report);
    }
}
