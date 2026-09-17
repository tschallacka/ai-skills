// MODE: DEV
// PACKAGE: PROD

//! The crate/binary table setup-dev-env-lib.sh's own plan()/plan_primary_*/
//! plan_secondary functions produce, embedded as static Rust data grouped
//! to mirror the bash structure 1:1 for diffability -- this is
//! DELIBERATELY a second source of truth alongside setup-dev-env-lib.sh's
//! real plan(), not a runtime call into that script (which would leave
//! setup-dev-env-lib.sh a permanent runtime dependency of the "ported"
//! binary, contradicting this crate's whole purpose). A crate added to or
//! removed from the workspace must be registered in BOTH places (the bash
//! row for the fallback path, this table for the wired path) or the two
//! paths silently diverge on what gets built -- an accepted, documented
//! risk, matching the same shape run-tests's own SUITES array already
//! carries in this plan, guarded by an automated drift test (W93) that
//! diffs this table against the real bash plan() output.

use std::path::Path;

pub type Row = (&'static str, &'static str);

const PRIMARY_ADD_AND_INFRA: &[Row] = &[
    ("add-adversarial-finding", "add-adversarial-finding"),
    ("add-coverage", "add-coverage"),
    ("add-fix-claim", "add-fix-claim"),
    ("add-goal", "add-goal"),
    ("add-planning-bug", "add-planning-bug"),
    ("add-ui-story", "add-ui-story"),
    ("add-ui-story-links", "add-ui-story-links"),
    ("add-work-unit", "add-work-unit"),
    ("chat-client-rs", "chat-client-rs"),
    ("chat-mcp", "chat-mcp"),
    ("chat-server-rs", "chat-server-rs"),
    ("configure-ui-story-cache", "configure-ui-story-cache"),
    ("create-adversarial-review", "create-adversarial-review"),
    ("create-plan", "create-plan"),
    ("create-plan-progress", "create-plan-progress"),
    ("update-plan-progress", "update-plan-progress"),
    ("rebuild-plan-progress", "rebuild-plan-progress"),
    ("register-read", "register-read"),
    ("register-command", "register-command"),
    ("register-rebuild", "register-rebuild"),
];

const PRIMARY_UPDATE_AND_VERIFY: &[Row] = &[
    ("plan-mutate", "plan-mutate"),
    ("todo-add", "todo-add"),
    ("todo-update", "todo-update"),
    ("bug-add", "bug-add"),
    ("bug-update", "bug-update"),
    ("supervision-frame", "supervision-frame"),
    ("generate-reviewer", "generate-reviewer"),
    ("cleanup-plans", "cleanup-plans"),
    ("verify-target", "verify-target"),
    ("update-step", "update-step"),
    ("verify-fix-keys", "verify-fix-keys"),
    ("update-work-unit", "update-work-unit"),
    ("validate-plan", "validate-plan"),
    ("run-adversary-probe", "run-adversary-probe"),
    ("update-plan-content", "update-plan-content"),
    ("monitor-read", "monitor-read"),
    ("create-progress", "create-progress"),
    ("create-step-testing", "create-step-testing"),
    ("create-ui-story-run-cache", "create-ui-story-run-cache"),
    ("create-ui-validation", "create-ui-validation"),
    ("create-work-unit-inventory", "create-work-unit-inventory"),
];

const SECONDARY: &[Row] = &[
    ("mint-fix-keys", "mint-fix-keys"),
    ("plan-crypt", "plan-crypt"),
    ("plan-env", "plan-env"),
    ("plan-content", "plan-content"),
    ("plan-context-wrapper", "plan-context-wrapper"),
    ("plan-context", "plan-context"),
    ("role-context", "role-context"),
    ("plan-overview", "plan-overview"),
    ("plan-overview", "overview-state"),
    ("plan-root", "plan-root"),
    ("remove-coverage", "remove-coverage"),
    ("remove-plan", "remove-plan"),
    ("remove-work-unit", "remove-work-unit"),
    ("resolve-finding", "resolve-finding"),
    ("tony-the-pony", "tony-the-pony"),
    ("update-adversarial-review", "update-adversarial-review"),
    ("update-progress", "update-progress"),
    ("update-ui-story", "update-ui-story"),
    ("rjq", "rjq"),
    ("bug-report", "bugs"),
    ("todo", "todo"),
    ("build-plan-libs", "build-plan-libs"),
    ("generate-skill-docs", "generate-skill-docs"),
    ("verify-skill-load", "verify-skill-load"),
    ("ci-failures", "ci-failures"),
    ("pre-push-check", "pre-push-check"),
    ("run-tests", "run-tests"),
    ("planning-server", "planning-server"),
    ("planning-server", "planning-client"),
    ("planning-mcp", "planning-mcp"),
    // This crate's own row: the self-hosting bootstrap closes exactly
    // because this row exists. On a fresh checkout no compiled
    // setup-dev-env binary can exist yet, so the bash fallback builds and
    // stages this crate like every other; only the NEXT invocation of
    // setup-dev-env.sh execs into it.
    ("setup-dev-env", "setup-dev-env"),
    // Registered after this crate itself, matching setup-dev-env-lib.sh's
    // own real plan_secondary() row order exactly (goal 17).
    ("generate-portability", "generate-portability"),
    // Goal 18: appended after generate-portability, matching
    // setup-dev-env-lib.sh's own real plan_secondary() row order exactly.
    ("blast-radius", "blast-radius"),
    // Goal 19: appended after blast-radius, matching setup-dev-env-lib.sh's
    // own real plan_secondary() row order exactly.
    ("verify-both-shells", "verify-both-shells"),
    // Goal 20: appended after verify-both-shells, matching
    // setup-dev-env-lib.sh's own real plan_secondary() row order exactly.
    ("ci-subjects", "ci-subjects"),
    // Goal 21: appended after ci-subjects, matching setup-dev-env-lib.sh's
    // own real plan_secondary() row order exactly.
    ("ci-scope", "ci-scope"),
    // Goal 22: appended after ci-scope, matching setup-dev-env-lib.sh's own
    // real plan_secondary() row order exactly.
    ("ci-test-scope", "ci-test-scope"),
    // Goal 26: appended after ci-test-scope, matching setup-dev-env-lib.sh's
    // own real plan_secondary() row order exactly.
    ("test-mermaid-accuracy", "test-mermaid-accuracy"),
];

const HARDCODED: &[Row] = &[
    ("ai-text-editor", "ai-text-editor"),
    ("ai-text-editor", "ai-text-editor-server"),
    ("ai-text-editor-mcp", "ai-text-editor-mcp"),
    ("interactive-shell", "interactive-shell"),
    ("interactive-shell", "interactive-shell-input"),
];

/// The full plan, in the same order setup-dev-env-lib.sh's own plan()
/// concatenates its groups: plan_primary (add_and_infra then
/// update_and_verify), plan_secondary, then the hardcoded tail.
pub fn plan() -> Vec<Row> {
    let mut rows = Vec::with_capacity(
        PRIMARY_ADD_AND_INFRA.len()
            + PRIMARY_UPDATE_AND_VERIFY.len()
            + SECONDARY.len()
            + HARDCODED.len(),
    );
    rows.extend_from_slice(PRIMARY_ADD_AND_INFRA);
    rows.extend_from_slice(PRIMARY_UPDATE_AND_VERIFY);
    rows.extend_from_slice(SECONDARY);
    rows.extend_from_slice(HARDCODED);
    rows
}

/// Whether `binary`'s compiled output belongs in planning/scripts/ alongside
/// its bin/<triple> copy: exactly the crates whose `.sh` predecessor still
/// exists, plus the register-rebuild special case (its own .sh oracle was
/// retired once skill_files() took over listing the binary directly).
pub fn stages_into_planning_scripts(repo_root: &Path, binary: &str) -> bool {
    if repo_root
        .join("planning/scripts")
        .join(format!("{binary}.sh"))
        .is_file()
    {
        return true;
    }
    binary == "register-rebuild"
}

/// B164: the root Cargo.toml is a virtual workspace globbing
/// members = ["src/*"], so cargo treats every directory under src/ as a
/// member and a missing Cargo.toml is fatal for the WHOLE workspace before
/// a single crate builds. Named explicitly here rather than let cargo's own
/// "failed to read .../Cargo.toml" -- true but silent about the real cause
/// -- be the only word.
pub fn check_stray_src_dirs(repo_root: &Path) -> Result<(), String> {
    let src_dir = repo_root.join("src");
    let mut stray = Vec::new();
    let Ok(entries) = std::fs::read_dir(&src_dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join("Cargo.toml").is_file() {
            stray.push(path);
        }
    }
    if stray.is_empty() {
        return Ok(());
    }
    let noun = if stray.len() == 1 { "y" } else { "ies" };
    let mut message = format!(
        "setup-dev-env: src/ has {} director{noun} with no Cargo.toml, which breaks the whole workspace build:\n",
        stray.len()
    );
    for dir in &stray {
        let relative = dir.strip_prefix(repo_root).unwrap_or(dir);
        message.push_str(&format!("  {}\n", relative.display()));
    }
    message.push_str("Remove it (a leftover from a deleted or renamed crate) and re-run:\n");
    for dir in &stray {
        let relative = dir.strip_prefix(repo_root).unwrap_or(dir);
        message.push_str(&format!("  rm -rf {}\n", relative.display()));
    }
    Err(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("setup-dev-env-plan-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn plan_includes_the_self_hosting_row() {
        assert!(plan().contains(&("setup-dev-env", "setup-dev-env")));
    }

    #[test]
    fn stages_into_planning_scripts_true_when_sh_sibling_exists() {
        let dir = scratch("sibling-present");
        fs::create_dir_all(dir.join("planning/scripts")).unwrap();
        fs::write(dir.join("planning/scripts/todo.sh"), "").unwrap();
        assert!(stages_into_planning_scripts(&dir, "todo"));
    }

    #[test]
    fn stages_into_planning_scripts_false_when_sh_sibling_absent() {
        let dir = scratch("sibling-absent");
        fs::create_dir_all(dir.join("planning/scripts")).unwrap();
        assert!(!stages_into_planning_scripts(&dir, "setup-dev-env"));
    }

    #[test]
    fn stages_into_planning_scripts_register_rebuild_special_case() {
        let dir = scratch("register-rebuild");
        fs::create_dir_all(dir.join("planning/scripts")).unwrap();
        assert!(stages_into_planning_scripts(&dir, "register-rebuild"));
    }

    #[test]
    fn check_stray_src_dirs_clean_tree_is_ok() {
        let dir = scratch("clean");
        fs::create_dir_all(dir.join("src/foo")).unwrap();
        fs::write(dir.join("src/foo/Cargo.toml"), "").unwrap();
        assert!(check_stray_src_dirs(&dir).is_ok());
    }

    #[test]
    fn check_stray_src_dirs_reports_a_stray_directory() {
        let dir = scratch("stray");
        fs::create_dir_all(dir.join("src/leftover")).unwrap();
        let err = check_stray_src_dirs(&dir).unwrap_err();
        assert!(err.contains("leftover"));
        assert!(err.contains("no Cargo.toml"));
    }
}
