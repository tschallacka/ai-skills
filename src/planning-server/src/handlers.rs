// MODE: DEV
// PACKAGE: PROD
//! The seven MVP request handlers. The two reads call directly into
//! plan-context-core's own bounded-read machinery (the same implementation
//! plan-context.sh is already wired onto). The four writes DELEGATE to the
//! existing standalone commands (update-step, add-work-unit,
//! update-plan-content) as real subprocesses rather than reimplementing
//! their own logic -- update-step/add-work-unit/update-plan-content are all
//! `main.rs`-only binaries with no `[lib]` target, so there is no library to
//! call instead, and delegation guarantees byte-identical output by
//! construction rather than by separately-verified parity. The guard gates
//! whether the subprocess runs at all: a stale guard means the subprocess is
//! never invoked. ValidatePlan is unguarded and read-only.
//!
//! Known MVP simplification, recorded rather than hidden: add-work-unit
//! touches TWO files (it inserts a row into work-unit-inventory.md and
//! creates a brand-new step .md file); only the inventory row is guarded
//! here, since the new step file does not exist yet at guard time and a
//! creation has nothing to check a hash against. update-step, in fact,
//! writes only ONE file -- confirmed directly against the real standalone
//! binary: it rewrites the goal's own progress.md status row and leaves the
//! step document itself completely untouched, so progress.md, not the step
//! file, is what update-step's own guard must check (an earlier version of
//! this handler guarded the step file instead, which never changed, so the
//! guard never actually detected anything -- every call, racing or
//! sequential, correct revision or stale, silently passed). Closing the
//! add-work-unit gap (covering its new step file too) is exactly the
//! "migrate every other command onto this mechanism" follow-on this goal's
//! own scope section defers.

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
        Request::ValidatePlan { plan_dir, complete } => validate_plan(bin_dir, &plan_dir, complete),
    }
}

fn program_path(bin_dir: Option<&Path>, name: &str) -> PathBuf {
    match bin_dir {
        Some(dir) => dir.join(name),
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
        let program = bin_dir.join(name);
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
        let output = cmd
            .current_dir(&workspace_root)
            .output()
            .unwrap_or_else(|error| panic!("could not build {name}: {error}"));
        assert!(
            output.status.success(),
            "building {name} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            program.is_file(),
            "{name} still missing at {} after building it",
            program.display()
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
