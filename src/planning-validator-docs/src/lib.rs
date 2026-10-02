// MODE: DEV
// PACKAGE: PROD
//! Document-level validation passes.

use planning_validator_common::{get_single_field, require_heading, Findings};
use regex::Regex;
use std::path::{Path, PathBuf};

pub const REQUIRED_PLAN_HEADINGS: &[&str] = &[
    "## Current state",
    "## Desired outcome",
    "## Approach",
    "## Approach decisions",
    "## Scope",
    "## Affected areas",
    "## Constraints and decisions",
    "## Risks and open questions",
    "## Environment facts",
    "## UI classification",
    "## Adversarial review",
];

#[derive(Debug, Default, Eq, PartialEq)]
pub struct DocumentState {
    pub ui_affected: String,
    pub review_approved: bool,
    pub plan_docs: Vec<PathBuf>,
}

/// Returns a process status code while recording ordinary missing-input
/// findings in the shared accumulator.
pub fn validate_existence(plan: &Path, findings: &mut Findings) -> i32 {
    if !plan.is_dir() {
        eprintln!("Plan directory not found: {}", plan.display());
        return 66;
    }
    if !plan.join("plan-description.md").is_file() {
        findings.fail("Missing plan-description.md");
    }
    if !plan.join("work-unit-inventory.md").is_file() {
        findings.fail("Missing work-unit-inventory.md");
    }
    if findings.errors > 0 {
        1
    } else {
        0
    }
}

/// Check the marker used to retire old plans. `None` means validation may
/// continue; `Some(65)` is a hard refusal.
pub fn validate_obsolete(plan: &Path, script_name: &str) -> Option<i32> {
    let marker = plan.join("OBSOLETE");
    if !marker.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(&marker).unwrap_or_default();
    let replacement = text.lines().find_map(|line| {
        line.trim_start()
            .strip_prefix("replaced-by:")
            .map(str::trim)
            .filter(|value| !value.is_empty())
    });
    eprintln!(
        "{script_name}: {} is marked obsolete by its OBSOLETE marker: it was built by an older planning-skill version and is never validated, resumed, or migrated.",
        plan.display()
    );
    if let Some(replacement) = replacement {
        eprintln!("{script_name}: the initiative was rebuilt as: {replacement}");
    } else {
        eprintln!("{script_name}: the marker names no replacement; add a \"replaced-by: <plan directory>\" line naming the plan that supersedes this one.");
    }
    eprintln!("{script_name}: nothing here was deleted — this directory is kept as history. Validate the replacement instead.");
    Some(65)
}

/// Detect duplicate numeric step prefixes within each goal and report a
/// collision description for each.
pub fn validate_step_numbers(plan: &Path, findings: &mut Findings) {
    let Ok(goals) = std::fs::read_dir(plan) else {
        return;
    };
    let mut by_goal = std::collections::BTreeMap::<String, Vec<(String, String)>>::new();
    for goal in goals.flatten().filter(|entry| entry.path().is_dir()) {
        let steps = goal.path().join("steps");
        let Ok(entries) = std::fs::read_dir(&steps) else {
            continue;
        };
        for step in entries.flatten().filter(|entry| entry.path().is_file()) {
            let Some(name) = step.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            // A testing companion shares its step's number by design (B335) --
            // exclude it before counting rather than reporting it as a
            // collision.
            if name.ends_with("-testing.md") {
                continue;
            }
            let Some((number, _)) = name.split_once('-') else {
                continue;
            };
            if number.len() != 2 || !number.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            by_goal
                .entry(goal.file_name().to_string_lossy().into_owned())
                .or_default()
                .push((number.to_owned(), name));
        }
    }
    for (goal, mut steps) in by_goal {
        steps.sort();
        for pair in steps.windows(2) {
            if pair[0].0 == pair[1].0 {
                findings.fail(format!(
                    "{goal} has two steps numbered {} ({} and {}); their order is undefined. Rename one to a free number and sweep the five surfaces that name a step: the step file, its testing companion, the inventory row, the goal blurb, and any progress tracker",
                    pair[0].0,
                    pair[0].1,
                    pair[1].1
                ));
            }
        }
    }
}

pub fn validate_plan_documents(
    plan: &Path,
    complete_mode: bool,
    findings: &mut Findings,
) -> DocumentState {
    let description = plan.join("plan-description.md");
    for heading in REQUIRED_PLAN_HEADINGS {
        require_heading(&description, heading, findings);
    }
    let ui_affected = get_single_field(&description, "UI affected", findings);
    if ui_affected != "yes" && ui_affected != "no" {
        findings.fail("UI classification must declare '- UI affected: yes' or 'no'");
    }
    let review = plan.join("adversarial-review.md");
    let mut review_approved = false;
    if !review.is_file() {
        findings.fail("Missing adversarial-review.md");
    } else {
        for heading in ["## Review scope", "## Findings", "## Verdict"] {
            require_heading(&review, heading, findings);
        }
        let review_text = std::fs::read_to_string(&review).unwrap_or_default();
        review_approved = review_text
            .lines()
            .any(|line| line == "- Status: `✅ approved`");
        if review_approved {
            let description_approved = std::fs::read_to_string(&description)
                .map(|text| text.lines().any(|line| line == "- Status: ✅ approved"))
                .unwrap_or(false);
            if !description_approved {
                findings
                    .fail("Plan description does not mirror approved adversarial-review status");
            }
            if review_text.lines().any(|line| {
                line.starts_with('|')
                    && line.contains("AR-")
                    && (line.contains("💤 open") || line.contains("⏳ in progress"))
            }) {
                findings.fail("Adversarial review has unresolved findings");
            }
            if rationale_staleness(&review_text, &plan.join("adversarial-review-history.md"))
                == Some(true)
            {
                findings.fail("Verdict rationale is stale: at least one more review cycle has been archived since it was written (T56 -- run update-adversarial-review.sh --set-rationale again after reviewing the latest cycle)");
            }
        } else if std::fs::read_to_string(&description)
            .map(|text| text.lines().any(|line| line == "- Status: ✅ approved"))
            .unwrap_or(false)
        {
            findings
                .fail("Plan description claims approval but adversarial review is not approved");
        } else if complete_mode {
            findings.fail("Adversarial review is not approved");
        } else {
            findings.warn("Adversarial review is not approved (expected mid-cycle; use validate-plan.sh --complete for the strict gate)");
        }
    }
    let inventory = plan.join("work-unit-inventory.md");
    for heading in [
        "## Definition-of-done coverage",
        "## Work units",
        "## Decomposition review",
    ] {
        require_heading(&inventory, heading, findings);
    }
    let inventory_text = std::fs::read_to_string(&inventory).unwrap_or_default();
    if inventory_text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("- [") && !trimmed.starts_with("- [x]") && !trimmed.starts_with("- [X]")
    }) {
        findings.fail("Decomposition review contains unchecked items");
    }
    for review in [
        "- [x] Every definition-of-done item maps to one or more work units.",
        "- [x] Every known affected file and changing symbol has its own work unit.",
        "- [x] Every work unit has exactly one goal and one step.",
        "- [x] Each goal has 2–10 work units, or records an allowed exception.",
        "- [x] Each step has exactly one work unit and no unnamed incidental edits.",
        "- [x] Dependencies form an executable order with no cycle.",
    ] {
        if !inventory_text.lines().any(|line| line == review) {
            findings.fail(format!("Missing completed decomposition review: {review}"));
        }
    }
    if inventory_text.to_ascii_lowercase().contains("tbd") {
        findings.fail("Inventory contains TBD; add a bounded discovery work unit instead");
    }
    let mut plan_docs = vec![description, review];
    let mut goal_files = std::fs::read_dir(plan)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path().join("goal.md"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    goal_files.sort();
    for goal_file in goal_files {
        plan_docs.push(goal_file.clone());
        let Some(steps_dir) = goal_file.parent().map(|parent| parent.join("steps")) else {
            continue;
        };
        let mut steps = std::fs::read_dir(steps_dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| !name.ends_with("-testing.md"))
            })
            .collect::<Vec<_>>();
        steps.sort();
        plan_docs.extend(steps);
    }
    plan_docs.push(inventory);
    validate_hardening(&plan_docs, findings);
    DocumentState {
        ui_affected,
        review_approved,
        plan_docs,
    }
}

/// Whether the Verdict's own "- Rationale cycle: N" stamp (written by
/// update-adversarial-review's own `--set-rationale`, T56) has fallen
/// behind the archive. `cycle_number` below mirrors that same binary's own
/// function exactly: `max(archived "## Cycle N") + 1`, the number the
/// CURRENT (not yet archived) findings table would get if archived right
/// now -- the same value a rationale is stamped with at write time. A
/// rationale is stale when a FRESH `cycle_number` computed now no longer
/// equals the stamp: at least one more review cycle has landed and been
/// archived since the rationale was written, so it may no longer describe
/// the findings actually in front of the reader.
///
/// `None` when no stamp is present at all -- a plan written before this
/// check existed, or a rationale someone hand-edited outside
/// `--set-rationale` -- so this never retroactively fails older plans; it
/// can only compare a stamp that is actually there.
fn rationale_staleness(review_text: &str, history: &Path) -> Option<bool> {
    let stamped: i64 = review_text.lines().find_map(|line| {
        line.strip_prefix("- Rationale cycle: ")?
            .trim()
            .parse()
            .ok()
    })?;
    Some(cycle_number(history) > stamped)
}

/// Mirrors update-adversarial-review's own `cycle_number(history, None)`
/// exactly (see that function's own doc comment for why the two must stay
/// byte-for-byte in agreement rather than merely "similar"). Duplicated
/// here rather than shared through a new crate: it is 8 lines, pure, and
/// keyed to the `## Cycle N` archive heading format, a stable convention
/// already load-bearing elsewhere in this same document family.
fn cycle_number(history: &Path) -> i64 {
    std::fs::read_to_string(history)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.strip_prefix("## Cycle ")?.parse::<i64>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

/// Hardens every plan document against hand-edit damage:
/// helper-flag-shaped text, duplicate paragraph labels, and shell-variable
/// path fragments.
fn validate_hardening(plan_docs: &[PathBuf], findings: &mut Findings) {
    let swallowed_flag = swallowed_flag_regex();
    let label_line = paragraph_label_regex();
    for doc in plan_docs {
        if !doc.is_file() {
            continue;
        }
        let name = doc
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let Ok(text) = std::fs::read_to_string(doc) else {
            continue;
        };
        if swallowed_flag.is_match(&text) {
            findings.fail(format!("{name} contains helper-flag-shaped text (-p N.N: etc.); mutate plan documents through the helpers, never by hand"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for line in text.lines() {
            if label_line.is_match(line) && !seen.insert(line.to_owned()) {
                findings.fail(format!(
                    "{name} has duplicate paragraph label {line}; renumber through the helpers"
                ));
                break;
            }
        }
        if text.contains("$script_dir/") || text.contains("$PLANNING_SKILL_DIR/") {
            findings.fail(format!(
                "{name} contains a shell-variable path fragment; bind file paths to the plan, not to script internals"
            ));
        }
    }
}

fn swallowed_flag_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(^|[[:space:]])-(p|dp|gp|sp|rp|tp|ia|ib)[[:space:]]+[0-9]+\.[0-9]+[[:space:]]*:",
        )
        .expect("swallowed-flag regex is a fixed literal")
    })
}

fn paragraph_label_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^§ [0-9]+\.[0-9]+$").expect("paragraph-label regex is a fixed literal")
    })
}

#[cfg(test)]
mod tests {
    use super::{
        rationale_staleness, validate_hardening, validate_obsolete, validate_step_numbers,
    };
    use planning_validator_common::Findings;
    use std::fs;

    fn scratch_history(name: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "validator-docs-rationale-{name}-{}.md",
            std::process::id()
        ));
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn rationale_staleness_is_none_with_no_stamp_at_all() {
        // Pre-existing content from before T56 shipped, or a rationale
        // someone hand-edited outside --set-rationale: nothing to compare,
        // so this must never retroactively flag an older plan.
        let history = scratch_history("no-stamp", "\n## Cycle 1\n\nrow\n");
        let review = "## Verdict\n\n- Status: `✅ approved`\n- Rationale: no findings remain.\n";
        assert_eq!(rationale_staleness(review, &history), None);
        let _ = fs::remove_file(history);
    }

    #[test]
    fn rationale_staleness_is_false_when_the_stamp_matches_the_current_cycle() {
        let history = scratch_history("fresh", "\n## Cycle 1\n\nrow\n");
        // cycle_number(history) here is 2 (max archived + 1) -- the same
        // value --set-rationale would have stamped if it ran right now.
        let review =
            "## Verdict\n\n- Status: `✅ approved`\n- Rationale: current.\n- Rationale cycle: 2\n";
        assert_eq!(rationale_staleness(review, &history), Some(false));
        let _ = fs::remove_file(history);
    }

    #[test]
    fn rationale_staleness_is_true_once_a_later_cycle_has_been_archived() {
        let history = scratch_history("stale", "\n## Cycle 1\n\nrow\n\n## Cycle 2\n\nrow\n");
        // Stamped for cycle 2, but a THIRD cycle's worth of archiving has
        // happened since (cycle_number(history) is now 3): the rationale
        // describes findings that have since been superseded.
        let review =
            "## Verdict\n\n- Status: `✅ approved`\n- Rationale: old.\n- Rationale cycle: 2\n";
        assert_eq!(rationale_staleness(review, &history), Some(true));
        let _ = fs::remove_file(history);
    }

    #[test]
    fn obsolete_marker_returns_the_shell_refusal_code() {
        let root = std::env::temp_dir().join(format!("validator-docs-{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        fs::write(root.join("OBSOLETE"), "replaced-by: newer-plan\n").unwrap();
        assert_eq!(validate_obsolete(&root, "validate-plan.sh"), Some(65));
        let _ = fs::remove_dir_all(root);
    }

    fn scratch_plan(name: &str, steps: &[&str]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "validator-docs-step-numbers-{name}-{}",
            std::process::id()
        ));
        let steps_dir = root.join("01-goal").join("steps");
        let _ = fs::create_dir_all(&steps_dir);
        for step in steps {
            fs::write(steps_dir.join(step), "").unwrap();
        }
        root
    }

    #[test]
    fn a_step_and_its_own_testing_companion_are_not_flagged_as_duplicates() {
        let root = scratch_plan("companion", &["01-step-foo.md", "01-step-foo-testing.md"]);
        let mut findings = Findings::default();
        validate_step_numbers(&root, &mut findings);
        assert_eq!(findings.errors, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn two_genuinely_different_steps_sharing_a_number_are_still_flagged() {
        let root = scratch_plan("collision", &["01-step-foo.md", "01-step-bar.md"]);
        let mut findings = Findings::default();
        validate_step_numbers(&root, &mut findings);
        assert_eq!(findings.errors, 1);
        let _ = fs::remove_dir_all(root);
    }

    fn scratch_doc(name: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "validator-docs-hardening-{name}-{}.md",
            std::process::id()
        ));
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn helper_flag_shaped_text_pasted_into_prose_is_flagged() {
        let doc = scratch_doc(
            "flag-shaped",
            "§ 5.9\nrun update-plan-content.sh -dp 2.3: x\n",
        );
        let mut findings = Findings::default();
        validate_hardening(std::slice::from_ref(&doc), &mut findings);
        assert_eq!(findings.errors, 1);
        let _ = fs::remove_file(doc);
    }

    #[test]
    fn ordinary_prose_mentioning_a_paragraph_label_is_not_flagged() {
        let doc = scratch_doc(
            "ordinary",
            "§ 5.1\nRun the migration and check the output.\n",
        );
        let mut findings = Findings::default();
        validate_hardening(std::slice::from_ref(&doc), &mut findings);
        assert_eq!(findings.errors, 0);
        let _ = fs::remove_file(doc);
    }

    #[test]
    fn a_duplicate_paragraph_label_is_flagged() {
        let doc = scratch_doc(
            "duplicate-label",
            "§ 2.1\nfirst\n\n§ 2.1\nrepeated by mistake\n",
        );
        let mut findings = Findings::default();
        validate_hardening(std::slice::from_ref(&doc), &mut findings);
        assert_eq!(findings.errors, 1);
        let _ = fs::remove_file(doc);
    }

    #[test]
    fn a_shell_variable_path_fragment_is_flagged() {
        let doc = scratch_doc(
            "shell-path",
            "§ 5.1\nSee $script_dir/validate-plan.sh for the logic.\n",
        );
        let mut findings = Findings::default();
        validate_hardening(std::slice::from_ref(&doc), &mut findings);
        assert_eq!(findings.errors, 1);
        let _ = fs::remove_file(doc);
    }
}
